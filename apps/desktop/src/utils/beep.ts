let sharedContext: AudioContext | null = null;

async function getAudioContext() {
  if (!sharedContext || sharedContext.state === "closed") {
    sharedContext = new AudioContext();
  }
  if (sharedContext.state === "suspended") {
    await sharedContext.resume();
  }
  return sharedContext;
}

/** Plays a soft ascending "start" tone — signals recording has begun. */
export async function playStartBeep() {
  const ctx = await getAudioContext();
  const osc = ctx.createOscillator();
  const gain = ctx.createGain();

  osc.connect(gain);
  gain.connect(ctx.destination);

  osc.type = "sine";
  const t = ctx.currentTime;

  // Gentle ascending sweep: 430 Hz → 570 Hz
  osc.frequency.setValueAtTime(430, t);
  osc.frequency.linearRampToValueAtTime(570, t + 0.12);

  gain.gain.setValueAtTime(0, t);
  gain.gain.linearRampToValueAtTime(0.09, t + 0.015); // soft attack
  gain.gain.setValueAtTime(0.09, t + 0.08);            // brief sustain
  gain.gain.exponentialRampToValueAtTime(0.001, t + 0.22); // smooth decay

  osc.start(t);
  osc.stop(t + 0.22);
  osc.onended = () => {
    osc.disconnect();
    gain.disconnect();
  };
}

/** Plays a soft descending "stop" tone — signals recording has ended. */
export async function playStopBeep() {
  const ctx = await getAudioContext();
  const osc = ctx.createOscillator();
  const gain = ctx.createGain();

  osc.connect(gain);
  gain.connect(ctx.destination);

  osc.type = "sine";
  const t = ctx.currentTime;

  // Gentle descending sweep: 430 Hz → 290 Hz
  osc.frequency.setValueAtTime(430, t);
  osc.frequency.linearRampToValueAtTime(290, t + 0.14);

  gain.gain.setValueAtTime(0, t);
  gain.gain.linearRampToValueAtTime(0.08, t + 0.015); // soft attack
  gain.gain.setValueAtTime(0.08, t + 0.07);            // brief sustain
  gain.gain.exponentialRampToValueAtTime(0.001, t + 0.25); // slightly longer decay

  osc.start(t);
  osc.stop(t + 0.25);
  osc.onended = () => {
    osc.disconnect();
    gain.disconnect();
  };
}

/** @deprecated Use playStartBeep instead */
export const playBeep = playStartBeep;
