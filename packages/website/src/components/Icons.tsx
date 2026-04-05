'use client';

interface IconProps {
  className?: string;
  strokeWidth?: number;
}

export function WaveformIcon({ className, strokeWidth = 1.5 }: IconProps) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      className={className}
      stroke="currentColor"
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M4 24c0 0 4-12 8-12s4 12 8 12 4-12 8-12 4 12 8 12 4-12 8-12" />
    </svg>
  );
}

export function ShieldIcon({ className, strokeWidth = 1.5 }: IconProps) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      className={className}
      stroke="currentColor"
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M24 4L6 12v12c0 10 8 18 18 20 10-2 18-10 18-20V12L24 4z" />
      <path d="M17 24l5 5 9-9" />
    </svg>
  );
}

export function SparklesIcon({ className, strokeWidth = 1.5 }: IconProps) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      className={className}
      stroke="currentColor"
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M24 4v8M24 36v8M4 24h8M36 24h8" />
      <path d="M12 12l6 6M30 30l6 6M12 36l6-6M30 18l6-6" />
      <circle cx="24" cy="24" r="4" fill="currentColor" stroke="none" />
    </svg>
  );
}

export function LockIcon({ className, strokeWidth = 1.5 }: IconProps) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      className={className}
      stroke="currentColor"
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <rect x="8" y="20" width="32" height="24" rx="4" />
      <path d="M16 20v-4a8 8 0 1 1 16 0v4" />
      <circle cx="24" cy="32" r="3" fill="currentColor" stroke="none" />
    </svg>
  );
}

export function HoldIcon({ className, strokeWidth = 1.5 }: IconProps) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      className={className}
      stroke="currentColor"
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <rect x="10" y="14" width="28" height="20" rx="4" />
      <path d="M24 14v-4" />
      <path d="M24 8c-4 0-6 2-6 4" />
    </svg>
  );
}

export function SpeakIcon({ className, strokeWidth = 1.5 }: IconProps) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      className={className}
      stroke="currentColor"
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <circle cx="24" cy="20" r="8" />
      <path d="M16 28c0 0 4 8 8 8s8-8 8-8" />
      <path d="M4 20c0-6 8-12 8-12s-4 6-4 12" />
      <path d="M44 20c0-6-8-12-8-12s4 6 4 12" />
    </svg>
  );
}

export function TypeIcon({ className, strokeWidth = 1.5 }: IconProps) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      className={className}
      stroke="currentColor"
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <rect x="4" y="10" width="40" height="28" rx="4" />
      <path d="M12 20h6M12 28h10" />
      <path d="M28 20l4 4-4 4M36 28h-8" />
    </svg>
  );
}