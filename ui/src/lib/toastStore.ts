import { writable } from "svelte/store";

interface Toast {
  id: number;
  message: string;
  type: "error" | "success";
}

export const toasts = writable<Toast[]>([]);
let nextId = 0;

export function showToast(message: string, type: "error" | "success" = "error") {
  const id = nextId++;
  toasts.update(t => [...t, { id, message, type }]);
  setTimeout(() => {
    toasts.update(t => t.filter(x => x.id !== id));
  }, 5000);
}

export function dismissToast(id: number) {
  toasts.update(t => t.filter(x => x.id !== id));
}
