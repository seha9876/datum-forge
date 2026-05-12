import { reactive } from "vue";

import type { AppColumn } from "../types";

/**
 * カラム編集フォームで使う一時的な編集中状態を管理します。
 *
 * @returns 編集中カラム情報と編集開始・取消ハンドラ
 */
export function useColumnEditing() {
  /** カラム編集フォームへバインドする編集中データです。 */
  const editingColumn = reactive({
    id: null as number | null,
    columnName: "",
    displayName: "",
    isRequired: false
  });

  /**
   * 指定したカラムの情報を編集フォームへ読み込みます。
   *
   * @param column 編集対象カラム
   */
  function startColumnEdit(column: AppColumn) {
    editingColumn.id = column.id;
    editingColumn.columnName = column.columnName;
    editingColumn.displayName = column.displayName;
    editingColumn.isRequired = column.isRequired;
  }

  /**
   * 編集フォームを初期状態へ戻します。
   */
  function cancelColumnEdit() {
    editingColumn.id = null;
    editingColumn.columnName = "";
    editingColumn.displayName = "";
    editingColumn.isRequired = false;
  }

  return {
    editingColumn,
    startColumnEdit,
    cancelColumnEdit
  };
}
