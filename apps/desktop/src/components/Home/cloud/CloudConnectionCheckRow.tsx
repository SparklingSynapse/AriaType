import { CheckCircle, CircleNotch, WarningCircle } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import type { CloudConnectionCheckResult } from "@/lib/tauri";

interface CloudConnectionCheckRowProps {
  result: CloudConnectionCheckResult | null;
  checking: boolean;
  onCheck: () => void;
}

function getStatusClass(result: CloudConnectionCheckResult | null) {
  if (!result) return "text-muted-foreground";
  return result.ok ? "text-emerald-600 dark:text-emerald-400" : "text-destructive";
}

function useStatusMessage(
  result: CloudConnectionCheckResult | null,
  checking: boolean,
) {
  const { t } = useTranslation();

  if (checking) return t("cloud.check.checking");
  if (!result) return t("cloud.check.notChecked");

  switch (result.kind) {
    case "ok":
      return t("cloud.check.ok");
    case "disabled":
      return t("cloud.check.disabled");
    case "missing_required":
      return t("cloud.check.missing_required");
    case "invalid_url":
      return t("cloud.check.invalid_url");
    case "unsupported_provider":
      return t("cloud.check.unsupported_provider");
    case "auth_failed":
      return t("cloud.check.auth_failed");
    case "model_failed":
      return t("cloud.check.model_failed");
    case "network_failed":
      return t("cloud.check.network_failed");
    case "timeout":
      return t("cloud.check.timeout");
    case "provider_error":
      return t("cloud.check.provider_error");
  }
}

export function CloudConnectionCheckRow({
  result,
  checking,
  onCheck,
}: CloudConnectionCheckRowProps) {
  const { t } = useTranslation();
  const message = useStatusMessage(result, checking);
  const StatusIcon = result?.ok ? CheckCircle : WarningCircle;

  return (
    <div className="flex items-center justify-between gap-3 pt-2">
      <div className="flex min-w-0 items-center gap-2">
        {checking ? (
          <CircleNotch className="h-4 w-4 shrink-0 animate-spin text-muted-foreground" />
        ) : result ? (
          <StatusIcon className={`h-4 w-4 shrink-0 ${getStatusClass(result)}`} />
        ) : null}
        <p className={`truncate text-xs ${getStatusClass(result)}`}>{message}</p>
      </div>
      <button
        type="button"
        onClick={onCheck}
        disabled={checking}
        className="inline-flex h-8 shrink-0 items-center gap-1.5 rounded-md border border-border bg-background px-3 text-xs font-medium text-foreground transition-colors hover:bg-secondary disabled:cursor-not-allowed disabled:opacity-60"
      >
        {checking ? (
          <CircleNotch className="h-3.5 w-3.5 animate-spin" />
        ) : (
          <CheckCircle className="h-3.5 w-3.5" />
        )}
        {t("cloud.check.button")}
      </button>
    </div>
  );
}
