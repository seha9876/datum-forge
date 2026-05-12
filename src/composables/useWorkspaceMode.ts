import { ref } from "vue";

export type WorkspaceMode = "design" | "data" | "master" | "view" | "settings";

/**
 * ワークスペースの表示モードを管理します。
 *
 * @returns 現在モードと切り替え関数
 */
export function useWorkspaceMode() {
  /** 画面に表示しているワークスペースモードです。 */
  const currentMode = ref<WorkspaceMode>("design");

  /**
   * 指定したワークスペースモードへ切り替えます。
   *
   * @param mode 切り替え先モード
   */
  function setMode(mode: WorkspaceMode) {
    currentMode.value = mode;
  }

  return {
    currentMode,
    setMode
  };
}
