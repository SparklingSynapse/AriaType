import { useState, useEffect } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { motion, AnimatePresence } from "framer-motion";
import { CaretDown, Plus, Trash } from "@phosphor-icons/react";
import {
  useFloating,
  offset,
  flip,
  shift,
  size,
  autoUpdate,
  useClick,
  useDismiss,
  useRole,
  useInteractions,
  FloatingPortal,
  FloatingFocusManager,
} from "@floating-ui/react";
import { modelCommands } from "@/lib/tauri";
import { logger } from "@/lib/logger";
import { analytics } from "@/lib/analytics";
import { AnalyticsEvents } from "@/lib/events";
import { useTranslation } from "react-i18next";
import { useConfirm } from "@/components/ui/confirm";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { CustomPolishTemplate, PolishTemplate } from "@/lib/tauri";

interface TemplateSelectorProps {
  selectedTemplate: string;
  customTemplates: CustomPolishTemplate[];
  onSelect: (templateId: string) => void;
  onTemplatesChange: () => void;
}

export function TemplateSelector({
  selectedTemplate,
  customTemplates,
  onSelect,
  onTemplatesChange,
}: TemplateSelectorProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [builtInTemplates, setBuiltInTemplates] = useState<PolishTemplate[]>([]);
  const confirm = useConfirm();

  useEffect(() => {
    modelCommands.getPolishTemplates()
      .then(setBuiltInTemplates)
      .catch((err: unknown) => logger.error("failed_to_get_polish_templates", { error: String(err) }));
  }, []);

  const { refs, floatingStyles, context } = useFloating({
    open,
    onOpenChange: setOpen,
    placement: "bottom-start",
    whileElementsMounted: autoUpdate,
    middleware: [
      offset(4),
      flip({ padding: 8 }),
      shift({ padding: 8 }),
      size({
        apply({ rects, elements, availableHeight }) {
          Object.assign(elements.floating.style, {
            width: `${rects.reference.width}px`,
            maxHeight: `${Math.min(availableHeight, 360)}px`,
          });
        },
        padding: 8,
      }),
    ],
  });

  const click = useClick(context);
  const dismiss = useDismiss(context);
  const role = useRole(context, { role: "listbox" });

  const { getReferenceProps, getFloatingProps } = useInteractions([
    click,
    dismiss,
    role,
]);

const BUILT_IN_TEMPLATE_KEY_MAP: Record<string, string> = {
  filler: "model.polish.templateFiller",
  formal: "model.polish.templateFormal",
  concise: "model.polish.templateConcise",
  agent: "model.polish.templateAgent",
};

const getTemplateName = (id: string) => {
  const i18nKey = BUILT_IN_TEMPLATE_KEY_MAP[id];
  if (i18nKey) return t(i18nKey);

  const custom = customTemplates.find((t) => t.id === id);
  if (custom) return custom.name;

  return id;
  };

  const handleDelete = async (template: CustomPolishTemplate, e: React.MouseEvent) => {
    e.stopPropagation();

    const confirmed = await confirm({
      title: t("model.polish.templateDeleteConfirm"),
      description: t("model.polish.templateDeleteConfirmDesc", { name: template.name }),
      confirmText: t("common.delete"),
      cancelText: t("common.cancel"),
      variant: "danger",
    });

    if (confirmed) {
      try {
        await modelCommands.deletePolishCustomTemplate(template.id);
        analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "polish_template_deleted", value: template.id });
        onTemplatesChange();
      } catch (err) {
        logger.error("failed_to_delete_template", { error: String(err) });
      }
    }
  };

  const handleCreateTemplate = () => {
    setOpen(false);
    setShowCreateModal(true);
  };

  const selectedLabel = getTemplateName(selectedTemplate);

  return (
    <>
      <button
        ref={refs.setReference}
        type="button"
        data-state={open ? "open" : "closed"}
        className={cn(
          "flex h-10 w-full items-center justify-between rounded-2xl border border-border bg-background px-4 py-2 text-sm transition-colors hover:bg-backgroundHover data-[state=open]:border-primary focus-visible:border-primary focus-visible:outline-none"
        )}
        {...getReferenceProps()}
      >
        <span>{selectedLabel}</span>
        <CaretDown
          className={cn(
            "h-4 w-4 text-muted-foreground transition-transform duration-200",
            open && "rotate-180"
          )}
        />
      </button>

      {open && (
        <FloatingPortal>
          <FloatingFocusManager context={context}>
            <div
              ref={refs.setFloating}
              style={floatingStyles}
              className="z-[9999] flex flex-col rounded-2xl border border-border bg-card shadow-lg outline-none overflow-hidden"
              {...getFloatingProps()}
            >
              <div className="overflow-y-auto">
                {builtInTemplates.length > 0 && (
                  <div className="p-1">
                    <div className="px-3 py-1.5 text-xs text-muted-foreground font-medium">
                      {t("model.polish.templateBuiltIn")}
                    </div>
                    {builtInTemplates.map((template) => (
                      <button
                        key={template.id}
                        type="button"
                        onClick={() => {
                          onSelect(template.id);
                          setOpen(false);
                        }}
                        className={cn(
                          "flex w-full items-center px-3 py-2 text-sm transition-colors hover:bg-backgroundHover outline-none rounded-xl",
                          template.id === selectedTemplate && "bg-background font-medium"
                        )}
                      >
                        {getTemplateName(template.id)}
                      </button>
                    ))}
                  </div>
                )}

                {customTemplates.length > 0 && (
                  <div className="p-1 border-t border-border">
                    <div className="px-3 py-1.5 text-xs text-muted-foreground font-medium">
                      {t("model.polish.templateMyTemplates")}
                    </div>
                    {customTemplates.map((template) => (
                      <div
                        key={template.id}
                        className="flex items-center gap-1 px-1 rounded-xl hover:bg-backgroundHover"
                      >
                        <button
                          type="button"
                          onClick={() => {
                            onSelect(template.id);
                            setOpen(false);
                          }}
                          className={cn(
                            "flex-1 text-left px-2 py-2 text-sm transition-colors outline-none rounded-l-xl",
                            template.id === selectedTemplate && "font-medium"
                          )}
                        >
                          {template.name}
                        </button>
                        <button
                          type="button"
                          onClick={(e) => handleDelete(template, e)}
                          className="p-1.5 rounded-lg text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors outline-none"
                        >
                          <Trash className="h-4 w-4" />
                        </button>
                      </div>
                    ))}
                  </div>
                )}

                <div className="p-1 border-t border-border">
                  <button
                    type="button"
                    onClick={handleCreateTemplate}
                    className="flex w-full items-center gap-2 px-3 py-2 text-sm text-primary transition-colors hover:bg-backgroundHover outline-none rounded-xl"
                  >
                    <Plus className="h-4 w-4" />
                    {t("model.polish.templateCreate")}
                  </button>
                </div>
              </div>
            </div>
          </FloatingFocusManager>
        </FloatingPortal>
      )}

      <CreateTemplateModal
        open={showCreateModal}
        onOpenChange={setShowCreateModal}
        onCreated={() => {
          onTemplatesChange();
        }}
      />
    </>
  );
}

interface CreateTemplateModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreated: () => void;
}

function CreateTemplateModal({ open, onOpenChange, onCreated }: CreateTemplateModalProps) {
  const { t } = useTranslation();
  const [name, setName] = useState("");
  const [prompt, setPrompt] = useState("");
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();

    if (!name.trim() || !prompt.trim()) return;

    setLoading(true);
    try {
      await modelCommands.createPolishCustomTemplate(name.trim(), prompt.trim());
      analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "polish_template_created" });
      setName("");
      setPrompt("");
      onOpenChange(false);
      onCreated();
    } catch (err) {
      logger.error("failed_to_create_template", { error: String(err) });
    } finally {
      setLoading(false);
    }
  };

  const handleOpenChange = (newOpen: boolean) => {
    if (!newOpen) {
      setName("");
      setPrompt("");
    }
    onOpenChange(newOpen);
  };

  return (
    <Dialog.Root open={open} onOpenChange={handleOpenChange}>
      <AnimatePresence>
        {open && (
          <Dialog.Portal forceMount>
            <Dialog.Overlay asChild>
              <motion.div
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                transition={{ duration: 0.15 }}
                className="fixed inset-0 z-50 bg-black/50"
              />
            </Dialog.Overlay>
            <div className="fixed inset-0 z-50 flex items-center justify-center pointer-events-none p-4">
              <Dialog.Content asChild>
                <motion.div
                  initial={{ opacity: 0, scale: 0.95 }}
                  animate={{ opacity: 1, scale: 1 }}
                  exit={{ opacity: 0, scale: 0.95 }}
                  transition={{ duration: 0.15 }}
                  className="bg-card border border-border rounded-3xl p-6 max-w-md w-full shadow-lg pointer-events-auto"
                >
                  <Dialog.Title className="text-lg font-semibold mb-4">
                    {t("model.polish.templateCreateTitle")}
                  </Dialog.Title>

                  <form onSubmit={handleSubmit} className="space-y-4">
                    <div className="space-y-2">
                      <label htmlFor="template-name" className="text-sm font-medium">
                        {t("model.polish.templateName")}
                      </label>
                      <input
                        id="template-name"
                        type="text"
                        value={name}
                        onChange={(e) => setName(e.target.value)}
                        placeholder={t("model.polish.templateNamePlaceholder")}
                        className="flex h-10 w-full rounded-2xl border border-border bg-background px-4 py-2 text-sm placeholder:text-muted-foreground focus-visible:border-primary focus-visible:outline-none"
                        required
                      />
                    </div>

                    <div className="space-y-2">
                      <label htmlFor="template-prompt" className="text-sm font-medium">
                        {t("model.polish.templatePrompt")}
                      </label>
                      <textarea
                        id="template-prompt"
                        value={prompt}
                        onChange={(e) => setPrompt(e.target.value)}
                        placeholder={t("model.polish.templatePromptPlaceholder")}
                        className="flex min-h-[120px] w-full rounded-2xl border border-border bg-background px-4 py-3 text-sm placeholder:text-muted-foreground focus-visible:border-primary focus-visible:outline-none resize-none"
                        required
                      />
                    </div>

                    <div className="flex gap-3 justify-end pt-2">
                      <Dialog.Close asChild>
                        <Button type="button" variant="outline" size="sm">
                          {t("common.cancel")}
                        </Button>
                      </Dialog.Close>
                      <Button type="submit" size="sm" disabled={loading || !name.trim() || !prompt.trim()}>
                        {loading ? t("common.saving") : t("common.save")}
                      </Button>
                    </div>
                  </form>
                </motion.div>
              </Dialog.Content>
            </div>
          </Dialog.Portal>
        )}
      </AnimatePresence>
    </Dialog.Root>
  );
}