<script setup lang="ts">
import { computed, ref, watch } from "vue";

import RecordEditorPanel from "./RecordEditorPanel.vue";
import RecordListPanel from "./RecordListPanel.vue";

import type {
  AppBootstrap,
  AppColumn,
  FieldType,
  ReferenceChoice,
  TableDetail
} from "../../../types";

const props = defineProps<{
  bootstrap: AppBootstrap | null;
  editingRecordId: number | null;
  inputType: (fieldType: FieldType) => string;
  recordValues: Record<string, unknown>;
  referenceChoices: (column: AppColumn) => ReferenceChoice[];
  selectedTable: TableDetail;
  onDeleteRecord: (recordId: number) => Promise<void>;
  onResetRecordForm: () => void;
  onStartEditRecord: (recordId: number) => void;
  onSubmitRecord: () => Promise<boolean>;
}>();

/** レコード編集パネルの開閉状態です。 */
const openPanels = ref<number[]>([]);
/** 主表示用カラムです。 */
const labelColumn = computed(
  () =>
    props.selectedTable.columns.find(
      (column) => column.id === props.selectedTable.table.labelColumnId
    ) ?? null
);
/** 編集中レコードの表示名です。 */
const editingRecordLabel = computed(() => {
  if (props.editingRecordId === null) {
    return null;
  }

  const record = props.selectedTable.records.find(
    (item) => item.id === props.editingRecordId
  );
  if (!record) {
    return `ID: ${props.editingRecordId}`;
  }

  if (labelColumn.value) {
    const label = record.displayValues[labelColumn.value.columnName];
    if (label) {
      return label;
    }
  }

  return `ID: ${record.id}`;
});
/** パネル見出しに出す現在の状態です。 */
const editorStatus = computed(() => {
  if (props.editingRecordId !== null) {
    return `編集中: ${editingRecordLabel.value ?? `ID: ${props.editingRecordId}`}`;
  }

  return openPanels.value.includes(0) ? "新規作成中" : "新規レコードを作成";
});

watch(
  () => props.selectedTable.table.id,
  () => {
    openPanels.value = [];
  }
);

/** 新規レコード作成を始めるため、入力フォームを初期化して開きます。 */
function handleStartCreateRecord() {
  props.onResetRecordForm();
  openPanels.value = [0];
}

/** 既存レコード編集を始めるため、対象 ID を親へ伝えてフォームを開きます。 */
function handleStartEditRecord(recordId: number) {
  props.onStartEditRecord(recordId);
  openPanels.value = [0];
}

/** レコード編集を取り消し、入力フォームを閉じます。 */
function handleCancelRecord() {
  props.onResetRecordForm();
  openPanels.value = [];
}

/** レコード保存を実行し、成功したときだけフォームを閉じます。 */
async function handleSubmitRecord() {
  try {
    const isSaved = await props.onSubmitRecord();
    if (isSaved) {
      openPanels.value = [];
    }
    return isSaved;
  } catch {
    return false;
  }
}

/**
 * レコード削除を親へ依頼し、削除後に編集フォームを閉じます。
 *
 * @param recordId 削除対象レコードID
 */
async function handleDeleteRecord(recordId: number) {
  try {
    await props.onDeleteRecord(recordId);
    openPanels.value = [];
  } catch {
    return;
  }
}
</script>

<template>
  <!-- データモード全体です。上に編集フォーム、下にレコード一覧を配置します。 -->
  <div class="design-layout">
    <!-- レコード編集フォームは必要なときだけ開けるアコーディオンです。 -->
    <v-expansion-panels v-model="openPanels" multiple class="inline-panel">
      <v-expansion-panel :value="0" rounded="xl" elevation="0">
        <v-expansion-panel-title>
          <div class="expansion-title">
            <strong>レコード編集</strong>
            <small>{{ editorStatus }}</small>
          </div>
        </v-expansion-panel-title>
        <v-expansion-panel-text class="px-4 pt-2 pb-4">
          <RecordEditorPanel
            :bootstrap="props.bootstrap"
            :columns="props.selectedTable.columns"
            :editing-record-id="props.editingRecordId"
            :input-type="props.inputType"
            :record-values="props.recordValues"
            :reference-choices="props.referenceChoices"
            :on-cancel-record="handleCancelRecord"
            :on-submit-record="handleSubmitRecord"
          />
        </v-expansion-panel-text>
      </v-expansion-panel>
    </v-expansion-panels>

    <!-- 保存済みレコードの一覧です。ここから新規作成や編集を開始できます。 -->
    <RecordListPanel
      :columns="props.selectedTable.columns"
      :records="props.selectedTable.records"
      :on-delete-record="handleDeleteRecord"
      :on-start-create-record="handleStartCreateRecord"
      :on-start-edit-record="handleStartEditRecord"
    />
  </div>
</template>
