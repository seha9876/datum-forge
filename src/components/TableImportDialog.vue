<script setup lang="ts">
import { computed, ref, watch } from "vue";

import { formatImportMessages } from "../utils/importErrorMessages";

import type {
  AppTableSummary,
  ImportColumnMappingPayload,
  ImportTableCsvMode,
  ImportTableCsvResult,
  InspectCsvImportResult,
  InspectExcelTablesResult,
  PreviewCsvImportResult,
  PreviewExcelTableImportResult
} from "../types";

type ImportSourceKind = "csv" | "excel";
type ImportPreview = PreviewCsvImportResult | PreviewExcelTableImportResult;

const props = defineProps<{
  modelValue: boolean;
  sourceKind: ImportSourceKind;
  table: AppTableSummary | null;
  inputPath: string;
  csvInspectResult: InspectCsvImportResult | null;
  inspectResult: InspectExcelTablesResult | null;
  mode: ImportTableCsvMode;
  onPreviewCsv: (
    tableId: number,
    inputPath: string,
    mode: ImportTableCsvMode,
    columnMapping: ImportColumnMappingPayload[]
  ) => Promise<PreviewCsvImportResult>;
  onPreview: (
    tableId: number,
    inputPath: string,
    excelTableName: string,
    mode: ImportTableCsvMode,
    columnMapping: ImportColumnMappingPayload[]
  ) => Promise<PreviewExcelTableImportResult>;
  onImport: (
    tableId: number,
    inputPath: string,
    excelTableName: string,
    mode: ImportTableCsvMode,
    columnMapping: ImportColumnMappingPayload[]
  ) => Promise<ImportTableCsvResult>;
  onImportCsv: (
    tableId: number,
    inputPath: string,
    mode: ImportTableCsvMode,
    columnMapping: ImportColumnMappingPayload[]
  ) => Promise<ImportTableCsvResult>;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: boolean];
  imported: [result: ImportTableCsvResult];
  error: [error: unknown];
}>();

const selectedExcelTableName = ref("");
const columnMapping = ref<ImportColumnMappingPayload[]>([]);
const preview = ref<ImportPreview | null>(null);
const busy = ref(false);

const isExcelSource = computed(() => props.sourceKind === "excel");
const dialogTitle = computed(() =>
  isExcelSource.value ? "Excelインポート" : "CSVインポート"
);
const sourceLabel = computed(() => (isExcelSource.value ? "Excel列" : "CSV列"));

const excelTableItems = computed(
  () =>
    props.inspectResult?.tables.map((table) => ({
      title: `${table.displayName} (${table.sheetName})`,
      value: table.name,
      props: {
        subtitle: `${table.range} / ${table.rowCount}件`
      }
    })) ?? []
);

const selectedExcelTable = computed(
  () =>
    props.inspectResult?.tables.find(
      (table) => table.name === selectedExcelTableName.value
    ) ?? null
);

const hasExcelTables = computed(
  () => (props.inspectResult?.tables.length ?? 0) > 0
);
const hasImportSource = computed(() =>
  isExcelSource.value
    ? (props.inspectResult?.tables.length ?? 0) > 0
    : Boolean(props.csvInspectResult)
);
const sourceColumnNames = computed(() =>
  isExcelSource.value
    ? (selectedExcelTable.value?.columnNames ?? [])
    : (props.csvInspectResult?.headers ?? [])
);
const canImport = computed(
  () => Boolean(preview.value) && (preview.value?.errors.length ?? 0) === 0
);
const formattedWarnings = computed(() =>
  formatImportMessages(preview.value?.warnings ?? [])
);
const formattedErrors = computed(() =>
  formatImportMessages(preview.value?.errors ?? [])
);
const previewChangeCount = computed(
  () => (preview.value?.insertedCount ?? 0) + (preview.value?.updatedCount ?? 0)
);

watch(
  () =>
    [props.sourceKind, props.inspectResult, props.csvInspectResult] as const,
  ([sourceKind, excelResult, csvResult]) => {
    if (sourceKind === "csv") {
      selectedExcelTableName.value = "";
      columnMapping.value = [];
      preview.value = null;
      if (csvResult) {
        void refreshPreview(true);
      }
      return;
    }

    if (!excelResult) {
      selectedExcelTableName.value = "";
      columnMapping.value = [];
      preview.value = null;
      return;
    }
    selectedExcelTableName.value =
      excelResult.suggestedTableName ?? excelResult.tables[0]?.name ?? "";
    columnMapping.value = [];
    void refreshPreview(true);
  },
  { immediate: true }
);

watch(
  () => props.mode,
  () => {
    void refreshPreview(false);
  }
);

async function refreshPreview(resetMapping: boolean) {
  if (!props.table || busy.value) {
    return;
  }
  if (isExcelSource.value && !selectedExcelTableName.value) {
    return;
  }
  if (!isExcelSource.value && !props.csvInspectResult) {
    return;
  }

  busy.value = true;
  try {
    const nextPreview = isExcelSource.value
      ? await props.onPreview(
          props.table.id,
          props.inputPath,
          selectedExcelTableName.value,
          props.mode,
          resetMapping ? [] : columnMapping.value
        )
      : await props.onPreviewCsv(
          props.table.id,
          props.inputPath,
          props.mode,
          resetMapping ? [] : columnMapping.value
        );
    preview.value = nextPreview;
    columnMapping.value = nextPreview.columnMappings.map((mapping) => ({
      targetColumnName: mapping.targetColumnName,
      sourceColumnName: mapping.sourceColumnName ?? ""
    }));
  } catch (error) {
    preview.value = null;
    emit("error", error);
  } finally {
    busy.value = false;
  }
}

async function handleExcelTableChange() {
  columnMapping.value = [];
  await refreshPreview(true);
}

function sourceColumnFor(targetColumnName: string) {
  return (
    columnMapping.value.find(
      (item) => item.targetColumnName === targetColumnName
    )?.sourceColumnName ?? ""
  );
}

function updateSourceColumn(
  targetColumnName: string,
  sourceColumnName: string | null
) {
  const mapping = columnMapping.value.find(
    (item) => item.targetColumnName === targetColumnName
  );
  if (mapping) {
    mapping.sourceColumnName = sourceColumnName ?? "";
  }
}

async function handleImport() {
  if (!props.table || !canImport.value) {
    return;
  }
  if (isExcelSource.value && !selectedExcelTableName.value) {
    return;
  }

  busy.value = true;
  try {
    const result = isExcelSource.value
      ? await props.onImport(
          props.table.id,
          props.inputPath,
          selectedExcelTableName.value,
          props.mode,
          columnMapping.value
        )
      : await props.onImportCsv(
          props.table.id,
          props.inputPath,
          props.mode,
          columnMapping.value
        );
    emit("imported", result);
    emit("update:modelValue", false);
  } catch (error) {
    emit("error", error);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <v-dialog
    :model-value="modelValue"
    max-width="980"
    scrollable
    @update:model-value="emit('update:modelValue', $event)"
  >
    <v-card rounded="xl">
      <v-card-title>{{ dialogTitle }}</v-card-title>
      <v-card-subtitle v-if="table">
        {{ table.displayName }} に取り込みます
      </v-card-subtitle>

      <v-card-text class="table-import-dialog">
        <v-alert
          v-if="isExcelSource && !hasExcelTables"
          type="warning"
          variant="tonal"
          class="table-import-alert"
        >
          このExcelファイルには、Excelの「テーブル」として設定された範囲が見つかりません。Excelで範囲を選択し、挿入
          > テーブル でテーブル化してから再度選択してください。
        </v-alert>

        <template v-else-if="hasImportSource">
          <v-select
            v-if="isExcelSource"
            v-model="selectedExcelTableName"
            :items="excelTableItems"
            label="取り込むExcelテーブル"
            density="compact"
            variant="outlined"
            :disabled="busy"
            @update:model-value="handleExcelTableChange"
          />

          <v-alert
            v-if="isExcelSource && inspectResult?.lastUsedTableName"
            type="info"
            variant="tonal"
            density="compact"
          >
            前回このDatum Forgeテーブルで使ったExcelテーブル:
            {{ inspectResult.lastUsedTableName }}
          </v-alert>

          <section v-if="preview" class="table-import-section">
            <div class="table-import-section-heading">
              <h3>列マッピング</h3>
              <v-btn
                size="small"
                variant="tonal"
                :loading="busy"
                @click="refreshPreview(false)"
              >
                プレビュー更新
              </v-btn>
            </div>
            <div class="table-import-mapping-grid">
              <div
                v-for="mapping in preview.columnMappings"
                :key="mapping.targetColumnName"
                class="table-import-mapping-row"
              >
                <div>
                  <strong>{{ mapping.targetDisplayName }}</strong>
                  <span class="text-medium-emphasis">
                    {{ mapping.targetColumnName }}
                  </span>
                  <v-chip
                    v-if="mapping.isRequired"
                    size="x-small"
                    color="primary"
                    variant="tonal"
                  >
                    必須
                  </v-chip>
                </div>
                <v-select
                  :model-value="sourceColumnFor(mapping.targetColumnName)"
                  :items="sourceColumnNames"
                  :label="sourceLabel"
                  clearable
                  density="compact"
                  hide-details
                  variant="outlined"
                  @update:model-value="
                    updateSourceColumn(mapping.targetColumnName, $event)
                  "
                />
              </div>
            </div>
          </section>

          <section v-if="preview" class="table-import-section">
            <h3>差分確認</h3>
            <div class="table-import-metrics">
              <v-chip color="success" variant="tonal">
                追加 {{ preview.insertedCount }}
              </v-chip>
              <v-chip color="primary" variant="tonal">
                更新 {{ preview.updatedCount }}
              </v-chip>
              <v-chip variant="tonal"
                >変更なし {{ preview.unchangedCount }}</v-chip
              >
              <v-chip color="warning" variant="tonal">
                スキップ {{ preview.skippedCount }}
              </v-chip>
              <v-chip color="error" variant="tonal">
                エラー {{ preview.errorCount }}
              </v-chip>
            </div>
            <p class="text-medium-emphasis">
              取り込み対象: {{ preview.totalRows }} 件。追加・更新予定
              {{ previewChangeCount }} 件のうち、先頭
              {{ preview.previewRows.length }} 件を表示しています。
            </p>
          </section>

          <div
            v-for="warning in formattedWarnings"
            :key="warning.id"
            class="table-import-message table-import-message-warning"
            role="alert"
          >
            <v-icon icon="mdi-alert-circle" size="small" />
            <div class="table-import-message-body">
              <span>{{ warning.message }}</span>
              <small v-if="warning.rowSummary">
                対象行: {{ warning.rowSummary }}
              </small>
              <small>{{ warning.action }}</small>
              <details
                v-if="warning.details.length > 0"
                class="table-import-message-details"
              >
                <summary>詳細</summary>
                <ul>
                  <li v-for="detail in warning.details" :key="detail">
                    {{ detail }}
                  </li>
                </ul>
              </details>
            </div>
          </div>

          <div
            v-for="error in formattedErrors"
            :key="error.id"
            class="table-import-message table-import-message-error"
            role="alert"
          >
            <v-icon icon="mdi-alert-circle" size="small" />
            <div class="table-import-message-body">
              <span>{{ error.message }}</span>
              <small v-if="error.rowSummary">
                対象行: {{ error.rowSummary }}
              </small>
              <small>{{ error.action }}</small>
              <details
                v-if="error.details.length > 0"
                class="table-import-message-details"
              >
                <summary>詳細</summary>
                <ul>
                  <li v-for="detail in error.details" :key="detail">
                    {{ detail }}
                  </li>
                </ul>
              </details>
            </div>
          </div>

          <section v-if="preview" class="table-import-section">
            <h3>プレビュー</h3>
            <v-alert
              v-if="preview.previewRows.length === 0"
              type="info"
              variant="tonal"
              density="compact"
            >
              追加・更新予定の行はありません。
            </v-alert>
            <div v-else class="table-import-preview-table-wrap">
              <v-table density="compact" class="table-import-preview-table">
                <thead>
                  <tr>
                    <th
                      v-for="mapping in preview.columnMappings"
                      :key="mapping.targetColumnName"
                    >
                      {{ mapping.targetDisplayName }}
                    </th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="(row, index) in preview.previewRows" :key="index">
                    <td
                      v-for="mapping in preview.columnMappings"
                      :key="mapping.targetColumnName"
                    >
                      {{ row[mapping.targetColumnName] }}
                    </td>
                  </tr>
                </tbody>
              </v-table>
            </div>
          </section>
        </template>
      </v-card-text>

      <v-card-actions>
        <v-spacer />
        <v-btn variant="text" @click="emit('update:modelValue', false)">
          キャンセル
        </v-btn>
        <v-btn
          color="primary"
          variant="flat"
          :disabled="!canImport"
          :loading="busy"
          @click="handleImport"
        >
          インポート
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.table-import-dialog {
  display: grid;
  gap: 1rem;
}

.table-import-section {
  display: grid;
  gap: 0.75rem;
}

.table-import-section-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.table-import-mapping-grid {
  display: grid;
  gap: 0.5rem;
}

.table-import-mapping-row {
  display: grid;
  grid-template-columns: minmax(11rem, 0.9fr) minmax(0, 1.1fr);
  gap: 0.75rem;
  align-items: center;
}

.table-import-mapping-row > div:first-child {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  min-width: 0;
}

.table-import-metrics {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.table-import-alert {
  align-items: flex-start;
  line-height: 1.45;
  min-height: auto;
  overflow-wrap: anywhere;
  padding: 0.75rem 1rem;
  white-space: normal;
}

.table-import-alert :deep(.v-alert__content) {
  align-self: flex-start;
  overflow: visible;
  overflow-wrap: anywhere;
  line-height: 1.45;
  white-space: normal;
}

.table-import-alert :deep(.v-alert__prepend) {
  align-self: flex-start;
  margin-top: 0.1rem;
}

.table-import-alert :deep(.v-alert__prepend > .v-icon) {
  flex: 0 0 auto;
}

.table-import-message {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 0.65rem;
  align-items: flex-start;
  border-radius: 6px;
  padding: 0.75rem 1rem;
  line-height: 1.45;
  overflow-wrap: anywhere;
  white-space: normal;
}

.table-import-message .v-icon {
  margin-top: 0.1rem;
}

.table-import-message-body {
  display: grid;
  gap: 0.2rem;
  min-width: 0;
}

.table-import-message-body small {
  color: rgb(var(--v-theme-on-surface));
}

.table-import-message-details {
  margin-top: 0.15rem;
  color: rgb(var(--v-theme-on-surface));
  font-size: 0.82rem;
}

.table-import-message-details summary {
  cursor: pointer;
  font-weight: 700;
}

.table-import-message-details ul {
  margin: 0.35rem 0 0;
  padding-left: 1rem;
}

.table-import-message-warning {
  background: rgba(var(--v-theme-warning), 0.14);
  color: rgb(var(--v-theme-on-surface));
}

.table-import-message-warning .v-icon {
  color: rgb(var(--v-theme-warning));
}

.table-import-message-error {
  background: rgba(var(--v-theme-error), 0.14);
  color: rgb(var(--v-theme-error));
}

.table-import-message-error .v-icon {
  color: rgb(var(--v-theme-error));
}

.table-import-preview-table-wrap {
  max-height: 280px;
  overflow: auto;
}

.table-import-preview-table {
  min-width: max-content;
}

.table-import-preview-table :deep(th),
.table-import-preview-table :deep(td) {
  max-width: 14rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
