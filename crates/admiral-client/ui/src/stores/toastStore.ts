import { create } from "zustand";

export type ToastType = "success" | "error" | "warning" | "info";

interface ToastItem {
  id: number;
  message: string;
  type: ToastType;
  duration: number;
}

interface ToastState {
  toasts: ToastItem[];
  nextId: number;
  show: (message: string, type?: ToastType, duration?: number) => void;
  dismiss: (id: number) => void;
}

export const useToastStore = create<ToastState>((set, get) => ({
  toasts: [],
  nextId: 0,
  show: (message, type = "info", duration) => {
    const d = duration ?? (type === "success" ? 3000 : type === "error" ? 5000 : 3000);
    const id = get().nextId;
    set((s) => ({
      toasts: [...s.toasts, { id, message, type, duration: d }],
      nextId: s.nextId + 1,
    }));
    setTimeout(() => {
      set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) }));
    }, d);
  },
  dismiss: (id) =>
    set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
}));
