"use client";

import { useRef, useState } from "react";
import Link from "next/link";
import { useTranslation } from "react-i18next";
import { useParams } from "next/navigation";
import { motion } from "framer-motion";
import { FileText, Globe, Keyboard, Lock, Mic, Sparkles } from "lucide-react";
import { HomeDownloadButton } from "@/components/HomeDownloadButton";

const reveal = {
  hidden: { opacity: 0, y: 16 },
  visible: { opacity: 1, y: 0 },
};

const transition = {
  duration: 0.6,
  ease: [0.16, 1, 0.3, 1],
};

function SectionLabel({ children }: { children: string }) {
  return (
    <p className="text-xs font-medium uppercase tracking-[0.2em] text-muted-foreground">
      {children}
    </p>
  );
}

function ContextVisual({ t }: { t: (key: string) => string }) {
  return (
    <div className="relative min-h-[460px] min-w-0 overflow-hidden rounded-3xl border border-border bg-card shadow-sm md:min-h-0" style={{ aspectRatio: "4 / 3" }}>
      <div className="absolute inset-0 bg-[radial-gradient(circle_at_25%_20%,rgba(96,165,250,0.16),transparent_32%),radial-gradient(circle_at_80%_10%,rgba(192,132,252,0.12),transparent_30%),linear-gradient(135deg,rgba(255,255,255,0.7),rgba(231,229,228,0.32))]" />
      <div className="relative flex h-full flex-col p-6 md:p-8">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <span className="h-2.5 w-2.5 rounded-full bg-red-400" />
            <span className="h-2.5 w-2.5 rounded-full bg-amber-400" />
            <span className="h-2.5 w-2.5 rounded-full bg-green-400" />
          </div>
          <div className="rounded-full border border-border bg-background/80 px-3 py-1 text-xs text-muted-foreground">
            {t("homePage.visual.contextBadge")}
          </div>
        </div>

        <div className="mt-8 rounded-2xl border border-border bg-background/85 p-5 shadow-sm backdrop-blur">
          <div className="flex items-center gap-2 text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">
            <FileText className="h-3.5 w-3.5" />
            {t("homePage.visual.activeField")}
          </div>
          <div className="mt-5 space-y-3 text-sm leading-6 text-foreground">
            <p className="text-muted-foreground">{t("homePage.visual.roughSpeech")}</p>
            <p className="rounded-2xl bg-card p-4 shadow-sm">
              {t("homePage.visual.contextOutput")}
              <span className="ml-1 inline-block h-5 w-0.5 animate-pulse bg-foreground align-middle" />
            </p>
          </div>
        </div>

        <div className="mt-auto grid grid-cols-1 gap-2 text-xs sm:grid-cols-3 sm:gap-3">
          <div className="rounded-2xl border border-border bg-background/70 p-2.5 sm:p-3">
            <div className="text-muted-foreground">{t("homePage.visual.appLabel")}</div>
            <div className="mt-1 font-medium text-foreground">{t("homePage.visual.appValue")}</div>
          </div>
          <div className="rounded-2xl border border-border bg-background/70 p-2.5 sm:p-3">
            <div className="text-muted-foreground">{t("homePage.visual.fieldLabel")}</div>
            <div className="mt-1 font-medium text-foreground">{t("homePage.visual.fieldValue")}</div>
          </div>
          <div className="rounded-2xl border border-border bg-background/70 p-2.5 sm:p-3">
            <div className="text-muted-foreground">{t("homePage.visual.toneLabel")}</div>
            <div className="mt-1 font-medium text-foreground">{t("homePage.visual.toneValue")}</div>
          </div>
        </div>
      </div>
    </div>
  );
}

function LayerVisual({ t }: { t: (key: string) => string }) {
  const items = [
    { icon: Mic, title: t("homePage.visual.noiseTitle"), detail: t("homePage.visual.noiseDetail") },
    { icon: Sparkles, title: t("homePage.visual.polishTitle"), detail: t("homePage.visual.polishDetail") },
    { icon: Lock, title: t("homePage.visual.localTitle"), detail: t("homePage.visual.localDetail") },
    { icon: Globe, title: t("homePage.visual.languageTitle"), detail: t("homePage.visual.languageDetail") },
  ];

  return (
    <div className="relative min-h-[520px] min-w-0 overflow-hidden rounded-3xl border border-border bg-card shadow-sm md:min-h-0" style={{ aspectRatio: "4 / 3" }}>
      <div className="absolute inset-0 bg-[radial-gradient(circle_at_50%_10%,rgba(74,222,128,0.14),transparent_30%),linear-gradient(160deg,rgba(28,25,23,0.04),rgba(231,229,228,0.45))]" />
      <div className="relative flex h-full flex-col p-5 md:p-6">
        <div className="mx-auto flex items-center gap-2 rounded-full border border-border bg-background/85 px-3 py-1.5 shadow-sm">
          <Keyboard className="h-3.5 w-3.5 text-muted-foreground" />
          <span className="text-xs font-medium text-foreground md:text-sm">{t("homePage.visual.shortcut")}</span>
        </div>

        <div className="mx-auto mt-5 flex h-20 w-20 items-center justify-center rounded-full border border-border bg-foreground text-primary-foreground shadow-sm md:h-24 md:w-24">
          <div className="flex items-end gap-1">
            {[24, 38, 52, 32, 44].map((height, index) => (
              <span
                key={index}
                className="w-1.5 rounded-full bg-primary-foreground/90"
                style={{ height }}
              />
            ))}
          </div>
        </div>

        <div className="mt-5 grid grid-cols-1 gap-2 sm:grid-cols-2">
          {items.map((item) => {
            const Icon = item.icon;
            return (
              <div key={item.title} className="rounded-2xl border border-border bg-background/75 p-3">
                <Icon className="h-3.5 w-3.5 text-muted-foreground" />
                <div className="mt-2 text-sm font-medium text-foreground">{item.title}</div>
                <div className="mt-0.5 text-xs leading-4 text-muted-foreground">{item.detail}</div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}

export default function HomePage() {
  const { t } = useTranslation();
  const { lang } = useParams() as { lang: string };
  const heroVideoRef = useRef<HTMLVideoElement>(null);
  const [heroPlaying, setHeroPlaying] = useState(false);

  const playHeroVideo = () => {
    heroVideoRef.current?.play();
    setHeroPlaying(true);
  };

  const steps = [
    {
      number: "01",
      title: t("homePage.steps.triggerTitle"),
      description: t("homePage.steps.triggerDescription"),
    },
    {
      number: "02",
      title: t("homePage.steps.speakTitle"),
      description: t("homePage.steps.speakDescription"),
    },
    {
      number: "03",
      title: t("homePage.steps.insertTitle"),
      description: t("homePage.steps.insertDescription"),
    },
  ];

  const featuresA = [
    {
      title: t("homePage.principles.cursorTitle"),
      description: t("homePage.principles.cursorDescription"),
    },
    {
      title: t("homePage.principles.privateTitle"),
      description: t("homePage.principles.privateDescription"),
    },
    {
      title: t("homePage.principles.desktopTitle"),
      description: t("homePage.principles.desktopDescription"),
    },
  ];

  const featuresB = [
    {
      title: t("homePage.controls.engineTitle"),
      description: t("homePage.controls.engineDescription"),
    },
    {
      title: t("homePage.controls.polishTitle"),
      description: t("homePage.controls.polishDescription"),
    },
  ];

  return (
    <div>
      <section className="pb-16 pt-32 md:pb-24 md:pt-44">
        <div className="mx-auto max-w-4xl px-6 text-center">
          <motion.div
            variants={reveal}
            initial="hidden"
            animate="visible"
            transition={{ ...transition, duration: 0.7 }}
            className="space-y-8"
          >
            <SectionLabel>{t("homePage.heroEyebrow")}</SectionLabel>
            <h1 className="text-[clamp(2.25rem,5.5vw,4.25rem)] font-semibold leading-[1.08] tracking-[-0.04em] text-foreground">
              {t("homePage.heroTitle")}
            </h1>
            <p className="mx-auto max-w-2xl text-lg leading-8 text-muted-foreground">
              {t("homePage.heroDescription")}
            </p>
          </motion.div>

          <motion.div
            variants={reveal}
            initial="hidden"
            animate="visible"
            transition={{ ...transition, delay: 0.12 }}
            className="mt-10 flex flex-col items-center justify-center gap-3 sm:flex-row"
          >
            <HomeDownloadButton lang={lang} />
            <Link
              href="https://github.com/joe223/AriaType"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex h-11 items-center justify-center rounded-full border border-border bg-card px-6 text-sm font-medium text-foreground transition-colors hover:bg-secondary"
            >
              {t("homePage.heroSecondaryCta")}
            </Link>
          </motion.div>
        </div>
      </section>

      <section className="pb-20 md:pb-28">
        <div className="mx-auto max-w-5xl px-6">
          <motion.div
            variants={reveal}
            initial="hidden"
            animate="visible"
            transition={{ ...transition, delay: 0.18, duration: 0.8 }}
            className="relative overflow-hidden rounded-3xl shadow-sm"
            style={{ aspectRatio: "4 / 3" }}
          >
            <video
              ref={heroVideoRef}
              muted
              playsInline
              src="/illustration/showcase.mp4"
              className="h-full w-full object-cover"
              onPlay={() => setHeroPlaying(true)}
              onPause={() => setHeroPlaying(false)}
              onEnded={() => setHeroPlaying(false)}
            />
            {!heroPlaying && (
              <button
                onClick={playHeroVideo}
                className="absolute inset-0 flex items-center justify-center bg-foreground/10 transition-colors hover:bg-foreground/15"
                aria-label={t("homePage.heroVideoPlay")}
              >
                <span className="flex h-16 w-16 items-center justify-center rounded-full bg-foreground/60 backdrop-blur-sm">
                  <svg width="24" height="24" viewBox="0 0 24 24" fill="white" xmlns="http://www.w3.org/2000/svg">
                    <path d="M8 5.14v13.72a1 1 0 0 0 1.5.86l11-6.86a1 1 0 0 0 0-1.72l-11-6.86a1 1 0 0 0-1.5.86Z" />
                  </svg>
                </span>
              </button>
            )}
          </motion.div>
        </div>
      </section>

      <section className="py-20 md:py-28">
        <div className="mx-auto max-w-4xl px-6">
          <motion.div
            variants={reveal}
            initial="hidden"
            whileInView="visible"
            viewport={{ once: true, margin: "-60px" }}
            transition={transition}
            className="text-center"
          >
            <SectionLabel>{t("homePage.workflowEyebrow")}</SectionLabel>
            <h2 className="mt-4 text-3xl font-semibold tracking-[-0.04em] text-foreground md:text-4xl">
              {t("homePage.workflowTitle")}
            </h2>
          </motion.div>

          <div className="mt-16 grid gap-12 md:grid-cols-3 md:gap-8">
            {steps.map((step, index) => (
              <motion.div
                key={step.number}
                variants={reveal}
                initial="hidden"
                whileInView="visible"
                viewport={{ once: true, margin: "-40px" }}
                transition={{ ...transition, delay: index * 0.08 }}
                className="text-center md:text-left"
              >
                <span className="text-xs font-medium uppercase tracking-[0.2em] text-muted-foreground">
                  {step.number}
                </span>
                <h3 className="mt-4 text-xl font-semibold tracking-[-0.03em] text-foreground">
                  {step.title}
                </h3>
                <p className="mt-3 text-sm leading-7 text-muted-foreground">
                  {step.description}
                </p>
              </motion.div>
            ))}
          </div>
        </div>
      </section>

      <section className="py-20 md:py-28">
        <div className="mx-auto max-w-6xl px-6">
          <div className="grid items-center gap-16 lg:grid-cols-2">
            <motion.div
              variants={reveal}
              initial="hidden"
              whileInView="visible"
              viewport={{ once: true, margin: "-60px" }}
              transition={transition}
              className="min-w-0"
            >
              <SectionLabel>{t("homePage.controlsEyebrow")}</SectionLabel>
              <h2 className="mt-4 text-3xl font-semibold tracking-[-0.04em] text-foreground md:text-4xl">
                {t("homePage.controlsTitle")}
              </h2>
              <p className="mt-4 text-base leading-8 text-muted-foreground">
                {t("homePage.controlsDescription")}
              </p>
              <div className="mt-10 space-y-8">
                {featuresA.map((feature) => (
                  <div key={feature.title} className="flex gap-3">
                    <span className="mt-2 h-1.5 w-1.5 flex-shrink-0 rounded-full bg-muted-foreground/40" />
                    <div>
                      <h3 className="text-base font-medium text-foreground">
                        {feature.title}
                      </h3>
                      <p className="mt-1.5 text-sm leading-7 text-muted-foreground">
                        {feature.description}
                      </p>
                    </div>
                  </div>
                ))}
              </div>
            </motion.div>

            <motion.div
              variants={reveal}
              initial="hidden"
              whileInView="visible"
              viewport={{ once: true, margin: "-60px" }}
              transition={{ ...transition, delay: 0.08 }}
              className="min-w-0"
            >
              <ContextVisual t={t} />
            </motion.div>
          </div>
        </div>
      </section>

      <section className="py-20 md:py-28">
        <div className="mx-auto max-w-6xl px-6">
          <div className="grid items-center gap-16 lg:grid-cols-2">
            <motion.div
              variants={reveal}
              initial="hidden"
              whileInView="visible"
              viewport={{ once: true, margin: "-60px" }}
              transition={transition}
              className="min-w-0"
            >
              <LayerVisual t={t} />
            </motion.div>

            <motion.div
              variants={reveal}
              initial="hidden"
              whileInView="visible"
              viewport={{ once: true, margin: "-60px" }}
              transition={{ ...transition, delay: 0.08 }}
              className="min-w-0"
            >
              <SectionLabel>{t("homePage.summaryEyebrow")}</SectionLabel>
              <h2 className="mt-4 text-3xl font-semibold tracking-[-0.04em] text-foreground md:text-4xl">
                {t("homePage.summaryTitle")}
              </h2>
              <p className="mt-4 text-base leading-8 text-muted-foreground">
                {t("homePage.summaryDescription")}
              </p>
              <div className="mt-10 space-y-8">
                {featuresB.map((feature) => (
                  <div key={feature.title} className="flex gap-3">
                    <span className="mt-2 h-1.5 w-1.5 flex-shrink-0 rounded-full bg-muted-foreground/40" />
                    <div>
                      <h3 className="text-base font-medium text-foreground">
                        {feature.title}
                      </h3>
                      <p className="mt-1.5 text-sm leading-7 text-muted-foreground">
                        {feature.description}
                      </p>
                    </div>
                  </div>
                ))}
              </div>
            </motion.div>
          </div>
        </div>
      </section>

      <section className="py-20 md:py-28">
        <div className="mx-auto max-w-3xl px-6 text-center">
          <motion.div
            variants={reveal}
            initial="hidden"
            whileInView="visible"
            viewport={{ once: true, margin: "-60px" }}
            transition={transition}
            className="space-y-6"
          >
            <SectionLabel>{t("homePage.closingEyebrow")}</SectionLabel>
            <h2 className="text-3xl font-semibold tracking-[-0.04em] text-foreground md:text-4xl">
              {t("homePage.closingTitle")}
            </h2>
            <p className="text-lg leading-8 text-muted-foreground">
              {t("homePage.closingDescription")}
            </p>
            <div className="pt-4">
              <HomeDownloadButton lang={lang} />
              <p className="mt-3 text-sm text-muted-foreground">
                {t("homePage.closingFootnote")}
              </p>
            </div>
          </motion.div>
        </div>
      </section>
    </div>
  );
}
