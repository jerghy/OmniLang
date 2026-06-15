/** Simple toast notification system — replaces alert() for connection errors. */

type ToastLevel = "error" | "warning" | "info";

interface Toast {
  id: number;
  message: string;
  level: ToastLevel;
}

let _toastId = 0;
let _toasts: Toast[] = [];
let _listener: ((toasts: Toast[]) => void) | null = null;

/** Subscribe to toast changes. Returns unsubscribe function. */
export function onToastChange(fn: (toasts: Toast[]) => void): () => void {
  _listener = fn;
  fn(_toasts);
  return () => { _listener = null; };
}

/** Show a toast. Auto-dismisses after `duration` ms. */
export function showToast(message: string, level: ToastLevel = "error", duration = 4000): void {
  const id = ++_toastId;
  _toasts = [..._toasts, { id, message, level }];
  _listener?.(_toasts);

  setTimeout(() => {
    _toasts = _toasts.filter((t) => t.id !== id);
    _listener?.(_toasts);
  }, duration);
}

/** Get current toasts (for React components). */
export function getToasts(): Toast[] {
  return _toasts;
}

/** Toast styling helpers. */
export const TOAST_STYLES: Record<ToastLevel, { bg: string; border: string; color: string }> = {
  error:   { bg: "#FEF2F2", border: "#FECACA", color: "#991B1B" },
  warning: { bg: "#FFFBEB", border: "#FDE68A", color: "#92400E" },
  info:    { bg: "#EFF6FF", border: "#BFDBFE", color: "#1E40AF" },
};
