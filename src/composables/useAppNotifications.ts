import { computed, inject, provide, reactive } from "vue";

import type { InjectionKey } from "vue";

export type AppNotificationKind = "success" | "warning" | "error" | "info";

export interface AppNotificationMetrics {
  insertedCount?: number;
  updatedCount?: number;
  skippedCount?: number;
  errorCount?: number;
}

export interface AppNotificationOptions {
  kind: AppNotificationKind;
  title: string;
  message?: string;
  metrics?: AppNotificationMetrics;
  details?: string[];
  timeout?: number;
}

export interface AppNotificationItem extends Required<AppNotificationOptions> {
  id: number;
}

interface AppNotificationState {
  current: AppNotificationItem | null;
  queue: AppNotificationItem[];
}

export interface AppNotificationController {
  state: Readonly<AppNotificationState>;
  isOpen: Readonly<{ value: boolean }>;
  notify: (options: AppNotificationOptions) => void;
  close: () => void;
}

const appNotificationKey: InjectionKey<AppNotificationController> =
  Symbol("appNotification");

/** 通知が連続したときに順番に表示できる、アプリ全体で1つだけ使うコントローラーを作ります。 */
export function createAppNotifications() {
  let nextId = 1;
  const state = reactive<AppNotificationState>({
    current: null,
    queue: []
  });

  function showNext() {
    state.current = state.queue.shift() ?? null;
  }

  return {
    state,
    isOpen: computed(() => state.current !== null),
    notify(options: AppNotificationOptions) {
      const item: AppNotificationItem = {
        id: nextId,
        kind: options.kind,
        title: options.title,
        message: options.message ?? "",
        metrics: options.metrics ?? {},
        details: options.details ?? [],
        timeout: options.timeout ?? (options.kind === "success" ? 4000 : -1)
      };
      nextId += 1;

      if (state.current) {
        state.queue.push(item);
      } else {
        state.current = item;
      }
    },
    close() {
      showNext();
    }
  } satisfies AppNotificationController;
}

export function provideAppNotifications(controller: AppNotificationController) {
  provide(appNotificationKey, controller);
}

export function useAppNotifications() {
  const controller = inject(appNotificationKey);
  if (!controller) {
    throw new Error("App notification provider is not registered.");
  }
  return controller;
}
