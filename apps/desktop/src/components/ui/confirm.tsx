import React, { createContext, useContext, useState, useCallback } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { motion, AnimatePresence } from "framer-motion";
import { Button } from "./button";

interface ConfirmOptions {
  title: string;
  description?: string;
  confirmText?: string;
  cancelText?: string;
  variant?: "default" | "danger";
}

interface ConfirmContextValue {
  confirm: (options: ConfirmOptions) => Promise<boolean>;
}

const ConfirmContext = createContext<ConfirmContextValue | null>(null);

export function useConfirm() {
  const context = useContext(ConfirmContext);
  if (!context) {
    throw new Error("useConfirm must be used within a ConfirmProvider");
  }
  return context.confirm;
}

interface ConfirmState extends ConfirmOptions {
  resolve: (value: boolean) => void;
  isOpen: boolean;
}

export function ConfirmProvider({ children }: { children: React.ReactNode }) {
  const [state, setState] = useState<ConfirmState | null>(null);

  const confirm = useCallback((options: ConfirmOptions): Promise<boolean> => {
    return new Promise((resolve) => {
      setState({
        ...options,
        resolve,
        isOpen: true,
      });
    });
  }, []);

  const handleConfirm = useCallback(() => {
    state?.resolve(true);
    setState(null);
  }, [state]);

  const handleCancel = useCallback(() => {
    state?.resolve(false);
    setState(null);
  }, [state]);

  const handleOpenChange = useCallback((open: boolean) => {
    if (!open) {
      handleCancel();
    }
  }, [handleCancel]);

  return (
    <ConfirmContext.Provider value={{ confirm }}>
      {children}
      <Dialog.Root open={state?.isOpen ?? false} onOpenChange={handleOpenChange}>
        <AnimatePresence>
          {state?.isOpen && (
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
              <div className="fixed inset-0 z-50 flex items-center justify-center pointer-events-none">
                <Dialog.Content asChild>
                  <motion.div
                    initial={{ opacity: 0, scale: 0.95 }}
                    animate={{ opacity: 1, scale: 1 }}
                    exit={{ opacity: 0, scale: 0.95 }}
                    transition={{ duration: 0.15 }}
                    className="bg-card border border-border rounded-xl p-6 max-w-sm w-[calc(100%-2rem)] shadow-lg pointer-events-auto"
                  >
                    <Dialog.Title className="text-lg font-semibold mb-2">
                      {state.title}
                    </Dialog.Title>
                    {state.description && (
                      <Dialog.Description className="text-muted-foreground text-sm mb-4">
                        {state.description}
                      </Dialog.Description>
                    )}
                    <div className="flex gap-3 justify-end mt-4">
                      <Dialog.Close asChild>
                        <Button variant="outline" size="sm">
                          {state.cancelText || "Cancel"}
                        </Button>
                      </Dialog.Close>
                      <Button
                        size="sm"
                        onClick={handleConfirm}
                        className={state.variant === "danger" ? "bg-destructive hover:bg-destructive/90" : ""}
                      >
                        {state.confirmText || "Confirm"}
                      </Button>
                    </div>
                  </motion.div>
                </Dialog.Content>
              </div>
            </Dialog.Portal>
          )}
        </AnimatePresence>
      </Dialog.Root>
    </ConfirmContext.Provider>
  );
}

let globalConfirm: ((options: ConfirmOptions) => Promise<boolean>) | null = null;

export function setGlobalConfirm(fn: (options: ConfirmOptions) => Promise<boolean>) {
  globalConfirm = fn;
}

export function confirm(options: ConfirmOptions): Promise<boolean> {
  if (!globalConfirm) {
    console.warn("confirm() called before ConfirmProvider was mounted");
    return Promise.resolve(false);
  }
  return globalConfirm(options);
}