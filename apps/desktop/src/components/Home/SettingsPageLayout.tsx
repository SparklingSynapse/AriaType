import { ReactNode } from "react";

interface SettingsPageLayoutProps {
  title?: string;
  description?: string;
  children: ReactNode;
  className?: string;
  testId?: string;
  variant?: "page" | "modal";
  showHeader?: boolean;
}

export function SettingsPageLayout({
  title,
  description,
  children,
  className = "",
  testId,
  variant = "page",
  showHeader = true,
}: SettingsPageLayoutProps) {
  const isModal = variant === "modal";

  return (
    <div
      className={
        isModal
          ? `w-full ${className}`
          : `mx-auto max-w-[1120px] px-8 py-8 md:px-10 md:py-9 ${className}`
      }
      data-testid={testId}
    >
      <div className={isModal ? "space-y-4" : "space-y-5"}>
        {showHeader && (title || description) && (
          <div className="space-y-2">
            {title && <h1 className="text-2xl font-semibold text-foreground">{title}</h1>}
            {description && <p className="max-w-3xl text-sm text-muted-foreground">{description}</p>}
          </div>
        )}
        {children}
      </div>
    </div>
  );
}
