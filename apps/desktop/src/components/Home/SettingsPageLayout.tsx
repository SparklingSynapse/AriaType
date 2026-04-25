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
      {(title || description) && (
        <div className="mb-6 md:mb-8">
          {title && (
            <h2 className="text-[1.7rem] font-semibold tracking-[-0.05em] text-primary">{title}</h2>
          )}
          {description && (
            <p className="mt-2 text-sm leading-7 text-muted-foreground">{description}</p>
          )}
        </div>
      )}
      <div className="space-y-6 md:space-y-8">{children}</div>
    </div>
  );
}
