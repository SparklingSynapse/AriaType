import { ReactNode } from "react";

interface SettingsPageLayoutProps {
  title?: string;
  description?: string;
  children: ReactNode;
  className?: string;
  testId?: string;
}

export function SettingsPageLayout({
  children,
  className = "",
  testId,
}: SettingsPageLayoutProps) {
  return (
    <div className={`mx-auto max-w-6xl p-10 ${className}`} data-testid={testId}>
      <div className="space-y-6">{children}</div>
    </div>
  );
}
