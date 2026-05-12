import { computed, ref } from "vue";

import { useAppStore } from "../stores/app";

export function useDisplaySettings() {
  // 表示カテゴリでは、DB ではなくアプリ全体の見た目に関わる設定だけを扱います。
  const store = useAppStore();
  const errorMessage = ref("");
  const toastMessage = ref("");
  const isToastVisible = ref(false);

  const showRecordIdsInNavigation = computed(
    () => store.settings?.showRecordIdsInNavigation ?? true
  );

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

  return {
    errorMessage,
    isToastVisible,
    loading: computed(() => store.loading),
    showRecordIdsInNavigation,
    toastMessage,
    updateRecordIdVisibility
  };
}
