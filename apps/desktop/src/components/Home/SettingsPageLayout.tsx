import { ReactNode } from "react";

interface SettingsPageLayoutProps {
  title?: string;
  description?: string;
  children: ReactNode;
  className?: string;
}

export function SettingsPageLayout({
  title,
  description,
  children,
  className = "",
}: SettingsPageLayoutProps) {
  return (
    <div className={`max-w-2xl mx-auto p-8 ${className}`}>
      {(title || description) && (
        <div className="mb-6">
          {title && (
            <h2 className="text-2xl font-semibold text-primary">{title}</h2>
          )}
          {description && (
            <p className="text-muted-foreground mt-1">{description}</p>
          )}
        </div>
      )}
      <div className="space-y-6">{children}</div>
    </div>
  );
}
