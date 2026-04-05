interface PermissionIconProps {
  className?: string;
}

export function PermissionIcon({ className }: PermissionIconProps) {
  return (
    <svg viewBox="0 0 160 90" fill="none" xmlns="http://www.w3.org/2000/svg" className={className}>
      <defs>
        <radialGradient id="permissionBg" cx="50%" cy="50%" r="50%" fx="50%" fy="50%">
          <stop offset="60%" stopColor="#ECFDF5" stopOpacity="0.8" className="dark:stop-color-[#064E3B]" />
          <stop offset="100%" stopColor="#ECFDF5" stopOpacity="0" className="dark:stop-color-[#064E3B]" />
        </radialGradient>
      </defs>

      {/* Background soft colorful glow */}
      <rect x="0" y="0" width="160" height="90" rx="24" fill="url(#permissionBg)" />
      
      {/* Center Shield - 3D effect */}
      <path d="M 80 20 L 55 30 L 55 55 C 55 70 65 85 80 90 C 95 85 105 70 105 55 L 105 30 Z" fill="#6EE7B7" opacity="0.5" transform="translate(0, 4)" />
      <path d="M 80 20 L 55 30 L 55 55 C 55 70 65 85 80 90 C 95 85 105 70 105 55 L 105 30 Z" fill="#34D399" />
      <path d="M 80 23 L 60 32 L 60 54 C 60 67 68 79 80 84 C 92 79 100 67 100 54 L 100 32 Z" fill="#A7F3D0" />
      
      {/* Cheeks */}
      <ellipse cx="72" cy="55" rx="4" ry="3" fill="#10B981" opacity="0.4" />
      <ellipse cx="88" cy="55" rx="4" ry="3" fill="#10B981" opacity="0.4" />
      
      {/* Eyes */}
      <circle cx="75" cy="50" r="3.5" fill="#064E3B" />
      <circle cx="85" cy="50" r="3.5" fill="#064E3B" />
      {/* Eye highlights */}
      <circle cx="76" cy="49" r="1.2" fill="#FFFFFF" />
      <circle cx="86" cy="49" r="1.2" fill="#FFFFFF" />
      
      {/* Mouth (Smile) */}
      <path d="M 77 58 Q 80 62 83 58" stroke="#064E3B" strokeWidth="2.5" strokeLinecap="round" fill="none" />
      
      {/* Floating Microphone on the left */}
      <g transform="translate(30, 30) rotate(-15)">
        <rect x="0" y="0" width="12" height="20" rx="6" fill="#60A5FA" />
        <rect x="2" y="2" width="8" height="10" rx="4" fill="#93C5FD" />
        <path d="M -3 12 Q 6 22 15 12" stroke="#3B82F6" strokeWidth="2" strokeLinecap="round" fill="none" />
        <line x1="6" y1="18" x2="6" y2="24" stroke="#3B82F6" strokeWidth="2" strokeLinecap="round" />
        <line x1="2" y1="24" x2="10" y2="24" stroke="#3B82F6" strokeWidth="2" strokeLinecap="round" />
      </g>

      {/* Floating Sparkle/Accessibility abstract symbol on the right */}
      <g transform="translate(120, 35) rotate(15)">
        <circle cx="10" cy="10" r="8" fill="#F472B6" />
        <circle cx="10" cy="10" r="4" fill="#FBCFE8" />
        <circle cx="10" cy="5" r="2" fill="#BE185D" />
        <line x1="10" y1="7" x2="10" y2="13" stroke="#BE185D" strokeWidth="1.5" strokeLinecap="round" />
        <line x1="6" y1="9" x2="14" y2="9" stroke="#BE185D" strokeWidth="1.5" strokeLinecap="round" />
        <line x1="10" y1="13" x2="7" y2="17" stroke="#BE185D" strokeWidth="1.5" strokeLinecap="round" />
        <line x1="10" y1="13" x2="13" y2="17" stroke="#BE185D" strokeWidth="1.5" strokeLinecap="round" />
      </g>
      
      {/* Magic Sparkles */}
      <path d="M 25 65 Q 25 70 30 70 Q 25 70 25 75 Q 25 70 20 70 Q 25 70 25 65 Z" fill="#FBBF24" />
      <path d="M 130 20 Q 130 24 134 24 Q 130 24 130 28 Q 130 24 126 24 Q 130 24 130 20 Z" fill="#F472B6" />
      <circle cx="45" cy="20" r="2" fill="#34D399" />
      <circle cx="110" cy="75" r="2.5" fill="#60A5FA" />
    </svg>
  );
}
