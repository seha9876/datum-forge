import { computed, inject, provide, reactive } from "vue";

import type { InjectionKey } from "vue";

export interface ConfirmDialogOptions {
  title: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  color?: string;
}

interface ConfirmDialogState {
  isOpen: boolean;
  title: string;
  message: string;
  confirmText: string;
  cancelText: string;
  color: string;
}

export interface ConfirmDialogController {
  state: Readonly<ConfirmDialogState>;
  isOpen: Readonly<{ value: boolean }>;
  open: (options: ConfirmDialogOptions) => Promise<boolean>;
  confirm: () => void;
  cancel: () => void;
}

const confirmDialogKey: InjectionKey<ConfirmDialogController> =
  Symbol("confirmDialog");

export function createConfirmDialog() {
  let resolver: ((value: boolean) => void) | null = null;
  const state = reactive<ConfirmDialogState>({
    isOpen: false,
    title: "",
    message: "",
    confirmText: "OK",
    cancelText: "キャンセル",
    color: "primary"
  });

  function settle(value: boolean) {
    if (!resolver) {
      state.isOpen = false;
      return;
    }

    const resolve = resolver;
    resolver = null;
    state.isOpen = false;
    resolve(value);
  }

  return {
    state,
    isOpen: computed(() => state.isOpen),
    open(options: ConfirmDialogOptions) {
      if (resolver) {
        settle(false);
      }

      state.title = options.title;
      state.message = options.message;
      state.confirmText = options.confirmText ?? "OK";
      state.cancelText = options.cancelText ?? "キャンセル";
      state.color = options.color ?? "primary";
      state.isOpen = true;

      return new Promise<boolean>((resolve) => {
        resolver = resolve;
      });
    },
    confirm() {
      settle(true);
    },
    cancel() {
      settle(false);
    }
  } satisfies ConfirmDialogController;
}

export function provideConfirmDialog(controller: ConfirmDialogController) {
  provide(confirmDialogKey, controller);
}

export function useConfirmDialog() {
  const controller = inject(confirmDialogKey);
  if (!controller) {
    throw new Error("Confirm dialog provider is not registered.");
  }
  return controller;
}
