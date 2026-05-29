import { ref } from "vue";

export type WorkspaceMode = "design" | "data" | "master" | "view" | "settings";

/** ワークスペース全体で現在表示しているモードを管理します。 */
export function useWorkspaceMode() {
  /** タイトルバーのタブや設定画面切り替えから直接更新される現在モードです。 */
  const currentMode = ref<WorkspaceMode>("design");

  return {
    currentMode
  };
}
