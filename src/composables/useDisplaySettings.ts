import { computed, ref, watch } from "vue";

import { useAppStore } from "../stores/app";

import type { NotificationSettings } from "../types";

const DEFAULT_NOTIFICATION_SETTINGS: NotificationSettings = {
  usePerKindDurations: false,
  commonDurationSeconds: 4,
  successDurationSeconds: 4,
  warningDurationSeconds: 4,
  errorDurationSeconds: 4
};

function normalizeDurationInput(value: string) {
  const parsed = Number.parseInt(value, 10);
  if (Number.isNaN(parsed)) {
    return 0;
  }
  return Math.min(60, Math.max(0, parsed));
}

export function useDisplaySettings() {
  // 表示カテゴリでは、DB ではなくアプリ全体の見た目に関わる設定だけを扱います。
  const store = useAppStore();
  const errorMessage = ref("");
  const toastMessage = ref("");
  const isToastVisible = ref(false);
  const usePerKindDurations = ref(false);
  const commonDurationSeconds = ref("4");
  const successDurationSeconds = ref("4");
  const warningDurationSeconds = ref("4");
  const errorDurationSeconds = ref("4");

  const showRecordIdsInNavigation = computed(
    () => store.settings?.showRecordIdsInNavigation ?? true
  );
  const notificationSettings = computed(
    () => store.settings?.notificationSettings ?? DEFAULT_NOTIFICATION_SETTINGS
  );

  function syncNotificationForm(settings = notificationSettings.value) {
    usePerKindDurations.value = settings.usePerKindDurations;
    commonDurationSeconds.value = String(settings.commonDurationSeconds);
    successDurationSeconds.value = String(settings.successDurationSeconds);
    warningDurationSeconds.value = String(settings.warningDurationSeconds);
    errorDurationSeconds.value = String(settings.errorDurationSeconds);
  }

  syncNotificationForm();

  watch(notificationSettings, (settings) => {
    syncNotificationForm(settings);
  });

  /** 設定保存に成功したことを、画面下部の通知で知らせます。 */
  function showToast(message: string) {
    toastMessage.value = message;
    isToastVisible.value = true;
  }

  /** 閲覧モード目次にレコードIDを表示するかどうかを保存します。 */
  async function updateRecordIdVisibility(show: boolean | null) {
    errorMessage.value = "";

    try {
      await store.updateRecordIdVisibility(show ?? false);
      showToast("表示設定を保存しました");
    } catch (error) {
      errorMessage.value =
        error instanceof Error ? error.message : String(error);
    }
  }

  async function saveNotificationSettings() {
    errorMessage.value = "";
    const nextSettings: NotificationSettings = {
      usePerKindDurations: usePerKindDurations.value,
      commonDurationSeconds: normalizeDurationInput(
        commonDurationSeconds.value
      ),
      successDurationSeconds: normalizeDurationInput(
        successDurationSeconds.value
      ),
      warningDurationSeconds: normalizeDurationInput(
        warningDurationSeconds.value
      ),
      errorDurationSeconds: normalizeDurationInput(errorDurationSeconds.value)
    };

    try {
      await store.updateNotificationSettings({
        notificationSettings: nextSettings
      });
      syncNotificationForm(nextSettings);
      showToast("通知設定を保存しました");
    } catch (error) {
      errorMessage.value =
        error instanceof Error ? error.message : String(error);
    }
  }

  return {
    commonDurationSeconds,
    errorDurationSeconds,
    errorMessage,
    isToastVisible,
    loading: computed(() => store.loading),
    saveNotificationSettings,
    showRecordIdsInNavigation,
    successDurationSeconds,
    toastMessage,
    updateRecordIdVisibility,
    usePerKindDurations,
    warningDurationSeconds
  };
}
