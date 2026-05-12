<script setup lang="ts">
import { computed, ref } from "vue";

import { useColumnEditing } from "../../../composables/useColumnEditing";
import { useConfirmDialog } from "../../../composables/useConfirmDialog";

import ColumnFormPanel from "./ColumnFormPanel.vue";
import ColumnListPanel from "./ColumnListPanel.vue";

import type {
  AddColumnPayload,
  AppBootstrap,
  AppColumn,
  FieldType,
  TableDetail
} from "../../../types";

const props = defineProps<{
  bootstrap: AppBootstrap | null;
  columnForm: AddColumnPayload;
  fieldTypes: FieldType[];
  fieldTypeLabel: (fieldType: FieldType) => string;
  fieldTypeMeta: (column: AppColumn) => string;
  selectedTable: TableDetail;
  onDeleteColumn: (columnId: number) => Promise<void>;
  onReorderColumns: (columns: AppColumn[]) => Promise<void>;
  onSubmitColumn: () => Promise<void>;
  onUpdateColumn: (
    columnId: number,
    columnName: string,
    displayName: string,
    isRequired: boolean
  ) => Promise<void>;
  onUpdateLabelColumn: (labelColumnId: number | null) => Promise<void>;
}>();

const { cancelColumnEdit, editingColumn, startColumnEdit } = useColumnEditing();
const confirmDialog = useConfirmDialog();
/** 現在選択中テーブルのカラム一覧です。 */
const columns = computed(() => props.selectedTable.columns);
/** 展開中のアコーディオンパネルを保持します。 */
const openPanels = ref<number[]>([]);

function isRequiredValueEmpty(value: unknown) {
  return (
    value === null ||
    value === undefined ||
    (typeof value === "string" && value.trim() === "")
  );
}

function countEmptyRecords(column: AppColumn) {
  return props.selectedTable.records.filter((record) =>
    isRequiredValueEmpty(record.values[column.columnName])
  ).length;
}

/**
 * 編集中カラムの内容を保存し、完了後に編集状態を解除します。
 */
async function submitColumnEdit() {
  if (editingColumn.id === null) {
    return;
  }

  const currentColumn = props.selectedTable.columns.find(
    (column) => column.id === editingColumn.id
  );
  if (currentColumn && !currentColumn.isRequired && editingColumn.isRequired) {
    const emptyRecordCount = countEmptyRecords(currentColumn);
    if (emptyRecordCount > 0) {
      const ok = await confirmDialog.open({
        title: "必須設定の変更",
        message: `既存レコードに未入力が ${emptyRecordCount} 件あります。今後の保存時には入力が必要になります。変更しますか？`,
        confirmText: "変更",
        color: "primary"
      });
      if (!ok) {
        return;
      }
    }
  }

  await props.onUpdateColumn(
    editingColumn.id,
    editingColumn.columnName,
    editingColumn.displayName,
    editingColumn.isRequired
  );
  cancelColumnEdit();
}

/**
 * カラム追加後に展開中パネルを閉じます。
 */
async function handleSubmitColumn() {
  await props.onSubmitColumn();
  openPanels.value = [];
}
</script>

<template>
  <div class="design-layout">
    <!-- 必要なときだけ開くカラム追加フォームです。 -->
    <v-expansion-panels v-model="openPanels" multiple class="inline-panel">
      <v-expansion-panel :value="0" rounded="xl" elevation="0">
        <v-expansion-panel-title>
          <div class="expansion-title">
            <strong>カラム追加</strong>
            <small>必要なときだけフォームを開けます</small>
          </div>
        </v-expansion-panel-title>
        <v-expansion-panel-text class="px-4 pt-2 pb-4">
          <ColumnFormPanel
            :bootstrap="bootstrap"
            :column-form="columnForm"
            :field-type-label="fieldTypeLabel"
            :field-types="fieldTypes"
            :on-submit-column="handleSubmitColumn"
          />
        </v-expansion-panel-text>
      </v-expansion-panel>
    </v-expansion-panels>

    <!-- 既存カラムの並び替え・編集・削除を行う一覧パネルです。 -->
    <ColumnListPanel
      :columns="columns"
      :editing-column="editingColumn"
      :field-type-label="fieldTypeLabel"
      :field-type-meta="fieldTypeMeta"
      :selected-table="selectedTable"
      :on-cancel-edit="cancelColumnEdit"
      :on-delete-column="onDeleteColumn"
      :on-reorder-columns="onReorderColumns"
      :on-start-edit="startColumnEdit"
      :on-submit-edit="submitColumnEdit"
      :on-update-label-column="onUpdateLabelColumn"
    />
  </div>
</template>
