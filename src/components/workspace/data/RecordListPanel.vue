<script setup lang="ts">
import { computed, ref } from "vue";

import { useConfirmDialog } from "../../../composables/useConfirmDialog";

import type { AppColumn, TableRecord } from "../../../types";

type DataTableHeader = {
  title: string;
  key: string;
  align?: "start" | "center" | "end";
  sortable?: boolean;
};

type RecordTableItem = {
  id: number;
  rawRecord: TableRecord;
} & Record<string, string | number | TableRecord>;

type SearchColumnItem = {
  title: string;
  value: string;
};

const props = defineProps<{
  columns: AppColumn[];
  records: TableRecord[];
  onDeleteRecord: (recordId: number) => Promise<void>;
  onStartCreateRecord: () => void;
  onStartEditRecord: (recordId: number) => void;
}>();

const confirmDialog = useConfirmDialog();
const recordSearchColumn = ref("all");
const recordSearchQuery = ref("");
const displayColumns = computed(() =>
  props.columns.filter((item) => item.columnName !== "id")
);
const tableHeaders = computed<DataTableHeader[]>(() => [
  { title: "ID", key: "id" },
  ...displayColumns.value.map((column) => ({
    title: column.displayName,
    key: column.columnName
  })),
  { title: "", key: "actions", align: "end", sortable: false }
]);
const searchColumnItems = computed<SearchColumnItem[]>(() => [
  { title: "すべて", value: "all" },
  { title: "ID", value: "id" },
  ...displayColumns.value.map((column) => ({
    title: column.displayName,
    value: column.columnName
  }))
]);
const tableFilterKeys = computed(() => {
  if (recordSearchColumn.value === "all") {
    return ["id", ...displayColumns.value.map((column) => column.columnName)];
  }

  return [recordSearchColumn.value];
});
const tableItems = computed<RecordTableItem[]>(() =>
  props.records.map((record) => {
    const item: RecordTableItem = {
      id: record.id,
      rawRecord: record
    };

    for (const column of displayColumns.value) {
      item[column.columnName] = record.displayValues[column.columnName] ?? "";
    }

    return item;
  })
);

/**
 * レコード削除前に確認ダイアログを表示します。
 *
 * @param recordId 削除対象レコード ID
 */
async function confirmDeleteRecord(recordId: number) {
  const ok = await confirmDialog.open({
    title: "レコードの削除",
    message: `レコード ID: ${recordId} を削除しますか？`,
    confirmText: "削除",
    color: "error"
  });

  if (!ok) {
    return;
  }

  await props.onDeleteRecord(recordId);
}
</script>

<template>
  <!-- 登録済みレコードを表形式で表示するパネルです。 -->
  <v-card
    tag="section"
    color="surface"
    variant="elevated"
    rounded="xl"
    elevation="2"
    border
    class="pa-4"
  >
    <!-- 一覧の見出し、検索、新規作成ボタンです。 -->
    <div class="section-heading">
      <div class="section-header">
        <div>
          <h2>レコード一覧</h2>
          <p class="help-text">ID以外のカラムを一覧に表示しています。</p>
        </div>
        <div class="record-list-actions">
          <v-chip size="small" color="primary" variant="tonal">
            レコード {{ props.records.length }}
          </v-chip>
          <v-select
            v-model="recordSearchColumn"
            density="comfortable"
            hide-details
            item-title="title"
            item-value="value"
            label="検索対象"
            :items="searchColumnItems"
            variant="outlined"
          />
          <v-text-field
            v-model="recordSearchQuery"
            clearable
            hide-details
            label="検索"
            placeholder="IDや表示値で検索"
            prepend-inner-icon="mdi-magnify"
            type="search"
            variant="outlined"
          />
          <v-btn
            color="primary"
            prepend-icon="mdi-plus"
            variant="flat"
            @click="props.onStartCreateRecord"
          >
            新規作成
          </v-btn>
        </div>
      </div>
    </div>

    <div class="table-wrap">
      <v-data-table
        density="comfortable"
        hover
        item-value="id"
        multi-sort
        :filter-keys="tableFilterKeys"
        :headers="tableHeaders"
        :items="tableItems"
        :search="recordSearchQuery"
        no-data-text="レコードがありません"
      >
        <template #[`item.actions`]="{ item }">
          <div class="table-row-actions">
            <v-btn
              density="comfortable"
              size="small"
              variant="tonal"
              @click="props.onStartEditRecord(item.rawRecord.id)"
            >
              編集
            </v-btn>
            <v-btn
              color="error"
              density="comfortable"
              size="small"
              variant="tonal"
              @click="confirmDeleteRecord(item.rawRecord.id)"
            >
              削除
            </v-btn>
          </div>
        </template>
      </v-data-table>
    </div>
  </v-card>
</template>

<style scoped>
.record-list-actions {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.record-list-actions :deep(.v-select) {
  min-width: 160px;
  max-width: 220px;
}

.record-list-actions :deep(.v-text-field) {
  min-width: 240px;
  max-width: 320px;
}
</style>
