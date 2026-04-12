import { toast, type ExternalToast } from "sonner";

/**
 * Show a success toast message.
 * Minimum display duration: 2 seconds.
 * Single toast mode - replaces any existing toast.
 */
export function showToast(message: string, options?: ExternalToast) {
  toast.success(message, {
    duration: 2000,
    ...options,
  });
}

/**
 * Show an error toast message.
 * Longer duration (3s) for errors that need more attention.
 */
export function showErrorToast(message: string, options?: ExternalToast) {
  toast.error(message, {
    duration: 3000,
    ...options,
  });
}

/**
 * Show an info toast message.
 */
export function showInfoToast(message: string, options?: ExternalToast) {
  toast.info(message, {
    duration: 2000,
    ...options,
  });
}

/**
 * Show a warning toast message.
 */
export function showWarningToast(message: string, options?: ExternalToast) {
  toast.warning(message, {
    duration: 2500,
    ...options,
  });
}