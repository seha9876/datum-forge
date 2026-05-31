<script setup lang="ts">
import { computed, ref, watch } from "vue";

import type {
  AppTableSummary,
  ExcelColumnMappingPayload,
  ImportTableCsvMode,
  ImportTableCsvResult,
  InspectExcelTablesResult,
  PreviewExcelTableImportResult
} from "../types";

const props = defineProps<{
  modelValue: boolean;
  table: AppTableSummary | null;
  inputPath: string;
  inspectResult: InspectExcelTablesResult | null;
  mode: ImportTableCsvMode;
  onPreview: (
    tableId: number,
    inputPath: string,
    excelTableName: string,
    mode: ImportTableCsvMode,
    columnMapping: ExcelColumnMappingPayload[]
  ) => Promise<PreviewExcelTableImportResult>;
  onImport: (
    tableId: number,
    inputPath: string,
    excelTableName: string,
    mode: ImportTableCsvMode,
    columnMapping: ExcelColumnMappingPayload[]
  ) => Promise<ImportTableCsvResult>;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: boolean];
  imported: [result: ImportTableCsvResult];
  error: [error: unknown];
}>();

const selectedExcelTableName = ref("");
const columnMapping = ref<ExcelColumnMappingPayload[]>([]);
const preview = ref<PreviewExcelTableImportResult | null>(null);
const busy = ref(false);

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
const canImport = computed(
  () => Boolean(preview.value) && (preview.value?.errors.length ?? 0) === 0
);

watch(
  () => props.inspectResult,
  (result) => {
    if (!result) {
      selectedExcelTableName.value = "";
      columnMapping.value = [];
      preview.value = null;
      return;
    }
    selectedExcelTableName.value =
      result.suggestedTableName ?? result.tables[0]?.name ?? "";
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
  if (!props.table || !selectedExcelTableName.value || busy.value) {
    return;
  }

  busy.value = true;
  try {
    const nextPreview = await props.onPreview(
      props.table.id,
      props.inputPath,
      selectedExcelTableName.value,
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
  if (!props.table || !selectedExcelTableName.value || !canImport.value) {
    return;
  }

  busy.value = true;
  try {
    const result = await props.onImport(
      props.table.id,
      props.inputPath,
      selectedExcelTableName.value,
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
      <v-card-title>Excelインポート</v-card-title>
      <v-card-subtitle v-if="table">
        {{ table.displayName }} に取り込みます
      </v-card-subtitle>

      <v-card-text class="excel-import-dialog">
        <v-alert v-if="!hasExcelTables" type="warning" variant="tonal">
          このExcelファイルには、Excelの「テーブル」として設定された範囲が見つかりません。Excelで範囲を選択し、挿入
          > テーブル でテーブル化してから再度選択してください。
        </v-alert>

        <template v-else>
          <v-select
            v-model="selectedExcelTableName"
            :items="excelTableItems"
            label="取り込むExcelテーブル"
            density="compact"
            variant="outlined"
            :disabled="busy"
            @update:model-value="handleExcelTableChange"
          />

          <v-alert
            v-if="inspectResult?.lastUsedTableName"
            type="info"
            variant="tonal"
            density="compact"
          >
            前回このDatum Forgeテーブルで使ったExcelテーブル:
            {{ inspectResult.lastUsedTableName }}
          </v-alert>

          <section v-if="preview" class="excel-import-section">
            <div class="excel-import-section-heading">
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
            <div class="excel-import-mapping-grid">
              <div
                v-for="mapping in preview.columnMappings"
                :key="mapping.targetColumnName"
                class="excel-import-mapping-row"
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
                  :items="selectedExcelTable?.columnNames ?? []"
                  label="Excel列"
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

          <section v-if="preview" class="excel-import-section">
            <h3>差分確認</h3>
            <div class="excel-import-metrics">
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
              取り込み予定件数: {{ preview.totalRows }} 件。先頭
              {{ preview.previewRows.length }} 件を表示しています。
            </p>
          </section>

          <v-alert
            v-for="warning in preview?.warnings ?? []"
            :key="warning"
            type="warning"
            variant="tonal"
            density="compact"
          >
            {{ warning }}
          </v-alert>

          <v-alert
            v-for="error in preview?.errors ?? []"
            :key="error"
            type="error"
            variant="tonal"
            density="compact"
          >
            {{ error }}
          </v-alert>

          <section v-if="preview" class="excel-import-section">
            <h3>プレビュー</h3>
            <v-table density="compact" class="excel-import-preview-table">
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
.excel-import-dialog {
  display: grid;
  gap: 1rem;
}

.excel-import-section {
  display: grid;
  gap: 0.75rem;
}

.excel-import-section-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.excel-import-mapping-grid {
  display: grid;
  gap: 0.5rem;
}

.excel-import-mapping-row {
  display: grid;
  grid-template-columns: minmax(11rem, 0.9fr) minmax(0, 1.1fr);
  gap: 0.75rem;
  align-items: center;
}

.excel-import-mapping-row > div:first-child {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  min-width: 0;
}

.excel-import-metrics {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.excel-import-preview-table {
  max-height: 280px;
  overflow: auto;
}
</style>
