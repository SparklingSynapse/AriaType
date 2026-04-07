import { useState, useEffect } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { motion, AnimatePresence } from "framer-motion";
import { Plus, Pencil, Trash2, Check } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { modelCommands } from "@/lib/tauri";
import { logger } from "@/lib/logger";
import { analytics } from "@/lib/analytics";
import { AnalyticsEvents } from "@/lib/events";
import { useTranslation } from "react-i18next";
import { useConfirm } from "@/components/ui/confirm";
import { SettingsPageLayout } from "./SettingsPageLayout";
import { cn } from "@/lib/utils";
import type { CustomPolishTemplate, PolishTemplate } from "@/lib/tauri";

export function PolishTemplatesPage() {
  const { t } = useTranslation();
  const [builtInTemplates, setBuiltInTemplates] = useState<PolishTemplate[]>([]);
  const [customTemplates, setCustomTemplates] = useState<CustomPolishTemplate[]>([]);
  const [selectedTemplate, setSelectedTemplate] = useState<string>("");
  const [editModalOpen, setEditModalOpen] = useState(false);
  const [editingTemplate, setEditingTemplate] = useState<CustomPolishTemplate | null>(null);
  const confirm = useConfirm();

  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    try {
      const [builtIn, custom, selected] = await Promise.all([
        modelCommands.getPolishTemplates(),
        modelCommands.getPolishCustomTemplates(),
        modelCommands.getPolishSelectedTemplate(),
      ]);
      setBuiltInTemplates(builtIn);
      setCustomTemplates(custom);
      setSelectedTemplate(selected);
    } catch (err) {
      logger.error("failed_to_load_templates", { error: String(err) });
    }
  };

  const handleSelectTemplate = async (templateId: string) => {
    try {
      await modelCommands.selectPolishTemplate(templateId);
      setSelectedTemplate(templateId);
      analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "polish_template", value: templateId });
    } catch (err) {
      logger.error("failed_to_select_template", { error: String(err) });
    }
  };

  const handleDelete = async (template: CustomPolishTemplate) => {
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
        loadData();
      } catch (err) {
        logger.error("failed_to_delete_template", { error: String(err) });
      }
    }
  };

  const handleEdit = (template: CustomPolishTemplate) => {
    setEditingTemplate(template);
    setEditModalOpen(true);
  };

  const handleCreate = () => {
    setEditingTemplate(null);
    setEditModalOpen(true);
  };

  return (
    <SettingsPageLayout
      title={t("polishTemplates.title")}
      description={t("polishTemplates.description")}
    >
      <Card>
        <CardHeader>
          <CardTitle>{t("polishTemplates.builtInTitle")}</CardTitle>
          <CardDescription>{t("polishTemplates.builtInDesc")}</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            {builtInTemplates.map((template) => (
              <button
                key={template.id}
                onClick={() => handleSelectTemplate(template.id)}
                className={cn(
                  "w-full flex items-center justify-between p-4 rounded-2xl border transition-all",
                  selectedTemplate === template.id
                    ? "border-primary bg-primary/5"
                    : "border-border hover:border-primary/50"
                )}
              >
                <div className="text-left">
                  <div className="font-medium">
                    {t(`model.polish.template${template.id.charAt(0).toUpperCase() + template.id.slice(1)}`)}
                  </div>
                  <div className="text-sm text-muted-foreground mt-1">
                    {t(`model.polish.template${template.id.charAt(0).toUpperCase() + template.id.slice(1)}Desc`)}
                  </div>
                </div>
                {selectedTemplate === template.id && (
                  <Check className="h-5 w-5 text-green-500" />
                )}
              </button>
            ))}
          </div>
        </CardContent>
      </Card>

      <Card className="mt-6">
        <CardHeader className="flex flex-row items-center justify-between">
          <div>
            <CardTitle>{t("polishTemplates.myTemplatesTitle")}</CardTitle>
            <CardDescription>{t("polishTemplates.myTemplatesDesc")}</CardDescription>
          </div>
          <Button size="sm" onClick={handleCreate}>
            <Plus className="h-4 w-4 mr-2" />
            {t("polishTemplates.createButton")}
          </Button>
        </CardHeader>
        <CardContent>
          {customTemplates.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              {t("polishTemplates.emptyHint")}
            </div>
          ) : (
            <div className="space-y-2">
              {customTemplates.map((template) => (
                <div
                  key={template.id}
                  className={cn(
                    "flex items-center justify-between p-4 rounded-2xl border transition-all",
                    selectedTemplate === template.id
                      ? "border-primary bg-primary/5"
                      : "border-border hover:border-primary/50"
                  )}
                >
                  <button
                    onClick={() => handleSelectTemplate(template.id)}
                    className="flex-1 text-left"
                  >
                    <div className="font-medium flex items-center gap-2">
                      {template.name}
                      {selectedTemplate === template.id && (
                        <Check className="h-4 w-4 text-green-500" />
                      )}
                    </div>
                    <div className="text-sm text-muted-foreground mt-1 line-clamp-2">
                      {template.system_prompt.slice(0, 100)}
                      {template.system_prompt.length > 100 && "..."}
                    </div>
                  </button>
                  <div className="flex gap-1 ml-4">
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => handleEdit(template)}
                    >
                      <Pencil className="h-4 w-4" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => handleDelete(template)}
                      className="text-muted-foreground hover:text-destructive"
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      <TemplateEditModal
        open={editModalOpen}
        onOpenChange={setEditModalOpen}
        template={editingTemplate}
        onSaved={loadData}
      />
    </SettingsPageLayout>
  );
}

interface TemplateEditModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  template: CustomPolishTemplate | null;
  onSaved: () => void;
}

function TemplateEditModal({ open, onOpenChange, template, onSaved }: TemplateEditModalProps) {
  const { t } = useTranslation();
  const [name, setName] = useState("");
  const [prompt, setPrompt] = useState("");
  const [loading, setLoading] = useState(false);
  const isEditing = template !== null;

  useEffect(() => {
    if (open) {
      if (template) {
        setName(template.name);
        setPrompt(template.system_prompt);
      } else {
        setName("");
        setPrompt("");
      }
    }
  }, [open, template]);

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();

    if (!name.trim() || !prompt.trim()) return;

    setLoading(true);
    try {
      if (isEditing && template) {
        await modelCommands.updatePolishCustomTemplate(template.id, name.trim(), prompt.trim());
        analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "polish_template_updated" });
      } else {
        await modelCommands.createPolishCustomTemplate(name.trim(), prompt.trim());
        analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "polish_template_created" });
      }
      onOpenChange(false);
      onSaved();
    } catch (err) {
      logger.error("failed_to_save_template", { error: String(err) });
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
                    {isEditing ? t("polishTemplates.editTitle") : t("polishTemplates.createTitle")}
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