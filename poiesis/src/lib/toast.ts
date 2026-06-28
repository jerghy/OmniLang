/** 命令感知的通知封装（基于 sonner）。*/

import { toast as sonnerToast } from "sonner";

export type ToastType = "success" | "error" | "info" | "loading";

export function showToast(message: string, type: ToastType = "info"): void {
  switch (type) {
    case "success":
      sonnerToast.success(message);
      break;
    case "error":
      sonnerToast.error(message);
      break;
    case "info":
      sonnerToast(message);
      break;
    case "loading":
      sonnerToast.loading(message);
      break;
  }
}
