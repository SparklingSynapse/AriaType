interface DoneIconProps {
  className?: string;
}

export function DoneIcon({ className }: DoneIconProps) {
  return (
    <svg viewBox="0 0 160 90" fill="none" xmlns="http://www.w3.org/2000/svg" className={className}>
      <defs>
        <radialGradient id="doneBg" cx="50%" cy="50%" r="50%" fx="50%" fy="50%">
          <stop offset="60%" stopColor="#FEF9C3" stopOpacity="0.8" className="dark:stop-color-[#713F12]" />
          <stop offset="100%" stopColor="#FEF9C3" stopOpacity="0" className="dark:stop-color-[#713F12]" />
        </radialGradient>
      </defs>

      {/* Background */}
      <rect x="0" y="0" width="160" height="90" rx="24" fill="url(#doneBg)" />
      
      {/* Confetti in background */}
      <path d="M 20 30 Q 30 20 40 30" stroke="#F472B6" strokeWidth="3" strokeLinecap="round" fill="none" />
      <path d="M 120 60 Q 130 70 140 60" stroke="#60A5FA" strokeWidth="3" strokeLinecap="round" fill="none" />
      <rect x="30" y="60" width="6" height="12" rx="3" fill="#34D399" transform="rotate(45 30 60)" />
      <rect x="130" y="25" width="6" height="12" rx="3" fill="#A78BFA" transform="rotate(-30 130 25)" />
      <circle cx="115" cy="20" r="3" fill="#F87171" />
      <circle cx="45" cy="75" r="4" fill="#FBBF24" />

      {/* Little floating ribbon badge below */}
      <path d="M 68 50 L 68 75 L 80 68 L 92 75 L 92 50 Z" fill="#EF4444" />
      <path d="M 72 50 L 72 70 L 80 64 L 88 70 L 88 50 Z" fill="#F87171" />

      {/* Center Big Star */}
      <path d="M 80 15 L 85 30 L 100 30 L 88 40 L 92 55 L 80 45 L 68 55 L 72 40 L 60 30 L 75 30 Z" fill="#FDE047" opacity="0.6" transform="translate(0, 5) scale(1.15) translate(-12, -7)" />
      <path d="M 80 15 L 85 30 L 100 30 L 88 40 L 92 55 L 80 45 L 68 55 L 72 40 L 60 30 L 75 30 Z" fill="#FBBF24" transform="scale(1.15) translate(-12, -7)" />
      <path d="M 80 18 L 84 31 L 97 31 L 87 40 L 90 53 L 80 44 L 70 53 L 73 40 L 63 31 L 76 31 Z" fill="#FDE047" transform="scale(1.15) translate(-12, -7)" />
      
      {/* Cheeks */}
      <ellipse cx="70" cy="42" rx="5" ry="3" fill="#F59E0B" opacity="0.6" />
      <ellipse cx="90" cy="42" rx="5" ry="3" fill="#F59E0B" opacity="0.6" />
      
      {/* Eyes (Happy closed eyes) */}
      <path d="M 67 36 Q 71 33 75 36" stroke="#78350F" strokeWidth="2.5" strokeLinecap="round" fill="none" />
      <path d="M 85 36 Q 89 33 93 36" stroke="#78350F" strokeWidth="2.5" strokeLinecap="round" fill="none" />
      
      {/* Mouth (Open happy mouth) */}
      <path d="M 75 42 Q 80 50 85 42 Z" fill="#78350F" />
      <path d="M 77 45 Q 80 48 83 45 Z" fill="#EF4444" />
      
      {/* Magic Sparkles */}
      <path d="M 50 20 Q 50 26 56 26 Q 50 26 50 32 Q 50 26 44 26 Q 50 26 50 20 Z" fill="#60A5FA" />
      <path d="M 105 65 Q 105 70 110 70 Q 105 70 105 75 Q 105 70 100 70 Q 105 70 105 65 Z" fill="#34D399" />
    </svg>
  );
}
