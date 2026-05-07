import { ReactNode } from "react";

interface SettingsPageLayoutProps {
  title?: string;
  description?: string;
  children: ReactNode;
  className?: string;
  testId?: string;
}

export function SettingsPageLayout({
  title,
  description,
  children,
  className = "",
  testId,
}: SettingsPageLayoutProps) {
  return (
    <div className={`mx-auto max-w-6xl p-10 ${className}`} data-testid={testId}>
      <div className="space-y-6">
        {(title || description) && (
          <div className="space-y-2">
            {title && <h1 className="text-xl font-semibold tracking-[-0.02em] text-foreground">{title}</h1>}
            {description && <p className="max-w-3xl text-sm text-muted-foreground">{description}</p>}
          </div>
        )}
        {children}
      </div>
    </div>
  );
}
