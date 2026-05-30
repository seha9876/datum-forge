<script setup lang="ts">
import { open, save } from "@tauri-apps/plugin-dialog";
import { ref } from "vue";

import { useConfirmDialog } from "../composables/useConfirmDialog";

import type {
  AppBootstrap,
  AppTableSummary,
  ImportTableCsvMode
} from "../types";

/** CSVインポートで最後に使った方式をブラウザ内へ保存するためのキーです。 */
const CSV_IMPORT_MODE_STORAGE_KEY = "datum-forge:csv-import-mode";
/** 初回インポート時に使う方式です。既存IDを壊しにくい安全寄りの方式にしています。 */
const DEFAULT_CSV_IMPORT_MODE: ImportTableCsvMode = "skipExistingPrimaryKeys";

/** サブメニューに出すインポート方式の内部ID一覧です。表示文言は下の関数で作ります。 */
const CSV_IMPORT_MODE_ITEMS: Array<{
  mode: ImportTableCsvMode;
  label: string;
}> = [
  {
    mode: "skipExistingPrimaryKeys",
    label: "既にあるプライマリーキー以外をインポート"
  },
  {
    mode: "appendIgnoringPrimaryKeys",
    label: "全てのデータをインポート"
  },
  {
    mode: "upsertByPrimaryKey",
    label: "ID重複時はインポート側で置き換え"
  }
];

const LABELS = {
  createTable: "テーブルを作成",
  tableList: "テーブル一覧",
  createNew: "新規作成",
  emptyHint: "まだテーブルがありません。まずは「新規作成」から始めましょう。",
  tableActions: "テーブル操作",
  deleteTable: "削除"
} as const;

const props = defineProps<{
  bootstrap: AppBootstrap | null;
  rail: boolean;
  selectedTableId: number | null;
  onDeleteTable: (tableId: number) => Promise<void>;
  onExportTableCsv: (tableId: number, outputPath: string) => Promise<void>;
  onImportTableCsv: (
    tableId: number,
    inputPath: string,
    mode: ImportTableCsvMode
  ) => Promise<void>;
  onLoadTable: (tableId: number) => Promise<void>;
  onOpenCreateDialog: () => void;
}>();

const confirmDialog = useConfirmDialog();
/** 通常クリック時に使う、現在選択中のCSVインポート方式です。 */
const selectedImportMode = ref<ImportTableCsvMode>(loadCsvImportMode());

/** localStorageから読んだ文字列が、実際にサポートしている方式か確認します。 */
function isImportMode(value: string | null): value is ImportTableCsvMode {
  return CSV_IMPORT_MODE_ITEMS.some((item) => item.mode === value);
}

/** 前回選んだインポート方式を読み込みます。壊れた値なら初期方式に戻します。 */
function loadCsvImportMode() {
  const savedMode = window.localStorage.getItem(CSV_IMPORT_MODE_STORAGE_KEY);
  return isImportMode(savedMode) ? savedMode : DEFAULT_CSV_IMPORT_MODE;
}

/** インポート方式を画面状態とlocalStorageの両方へ保存します。 */
function saveCsvImportMode(mode: ImportTableCsvMode) {
  selectedImportMode.value = mode;
  window.localStorage.setItem(CSV_IMPORT_MODE_STORAGE_KEY, mode);
}

/** サブメニューで太字相当に見える、短い方式名を返します。 */
function csvImportModeLabel(mode: ImportTableCsvMode) {
  switch (mode) {
    case "skipExistingPrimaryKeys":
      return "新しいIDの行だけ追加";
    case "appendIgnoringPrimaryKeys":
      return "すべて新しい行として追加";
    case "upsertByPrimaryKey":
      return "同じIDの行は上書き";
  }
}

/** 方式名だけでは分かりにくい挙動を、1行説明として返します。 */
function csvImportModeDescription(mode: ImportTableCsvMode) {
  switch (mode) {
    case "skipExistingPrimaryKeys":
      return "CSVのIDが既にある行はスキップします";
    case "appendIgnoringPrimaryKeys":
      return "CSVのIDは使わず、新しいIDで追加します";
    case "upsertByPrimaryKey":
      return "既存IDは更新し、ないIDは追加します";
  }
}

/** レール表示の丸アイコンへ入れる、テーブル名の先頭1文字を返します。 */
function tableInitial(name: string) {
  return name.slice(0, 1).toUpperCase();
}

/** CSVエクスポートの保存ダイアログに出す初期ファイル名を作ります。 */
function csvDefaultFileName(table: AppTableSummary) {
  const today = new Date();
  const yyyy = String(today.getFullYear());
  const mm = String(today.getMonth() + 1).padStart(2, "0");
  const dd = String(today.getDate()).padStart(2, "0");
  const safeDisplayName = table.displayName
    .trim()
    .split("")
    .map((char) =>
      char.charCodeAt(0) < 32 || /[<>:"/\\|?*]/.test(char) ? "_" : char
    )
    .join("")
    .replace(/[. ]+$/g, "");

  return `${safeDisplayName || table.tableName}_${yyyy}${mm}${dd}.csv`;
}

/** ユーザーが拡張子なしで保存した場合でもCSVファイルになるよう補います。 */
function ensureCsvExtension(path: string) {
  return /\.csv$/i.test(path) ? path : `${path}.csv`;
}

/** 保存先を選んでから、Rust側のCSVエクスポート処理へ渡します。 */
async function handleExportTableCsv(table: AppTableSummary) {
  const outputPath = await save({
    defaultPath: csvDefaultFileName(table),
    filters: [
      {
        name: "CSV",
        extensions: ["csv"]
      }
    ]
  });

  if (typeof outputPath !== "string") {
    return;
  }

  await props.onExportTableCsv(table.id, ensureCsvExtension(outputPath));
}

/** CSVファイルを選んでから、選択中の方式でRust側のインポート処理へ渡します。 */
async function handleImportTableCsv(
  table: AppTableSummary,
  mode = selectedImportMode.value
) {
  saveCsvImportMode(mode);
  const inputPath = await open({
    multiple: false,
    filters: [
      {
        name: "CSV",
        extensions: ["csv"]
      }
    ]
  });

  if (typeof inputPath !== "string") {
    return;
  }

  await props.onImportTableCsv(table.id, inputPath, mode);
}

async function handleDeleteTable(table: AppTableSummary) {
  const confirmed = await confirmDialog.open({
    title: "テーブルの削除",
    message: `テーブル「${table.displayName} (${table.tableName})」を削除します。テーブル内のレコード、カラム、閲覧ナビ配置、レイアウト差分も削除され、元に戻せません。削除しますか？`,
    confirmText: "削除",
    color: "error"
  });
  if (!confirmed) {
    return;
  }

  try {
    await props.onDeleteTable(table.id);
  } catch {
    // 詳細なエラーはアプリ上部のエラー表示に委ねます。
  }
}
</script>

<template>
  <div class="sidebar-content" :class="{ rail }">
    <template v-if="rail">
      <div class="sidebar-rail-shell">
        <div class="sidebar-rail-actions">
          <v-tooltip :text="LABELS.createTable" location="right">
            <template #activator="{ props: tooltipProps }">
              <v-btn
                v-bind="tooltipProps"
                icon="mdi-table-plus"
                variant="tonal"
                color="primary"
                :aria-label="LABELS.createTable"
                @click="onOpenCreateDialog"
              />
            </template>
          </v-tooltip>
        </div>

        <div class="sidebar-scroll-section rail">
          <v-list
            class="sidebar-list sidebar-list-rail"
            nav
            density="comfortable"
          >
            <v-tooltip
              v-for="table in props.bootstrap?.tables ?? []"
              :key="table.id"
              :text="`${table.displayName} (${table.tableName})`"
              location="right"
            >
              <template #activator="{ props: tooltipProps }">
                <v-list-item
                  v-bind="tooltipProps"
                  :active="table.id === selectedTableId"
                  rounded="xl"
                  class="sidebar-table-item rail"
                  @click="onLoadTable(table.id)"
                >
                  <template #prepend>
                    <v-avatar size="36" color="primary" variant="tonal">
                      {{ tableInitial(table.displayName) }}
                    </v-avatar>
                  </template>
                </v-list-item>
              </template>
            </v-tooltip>
          </v-list>
        </div>
      </div>
    </template>

    <template v-else>
      <div class="sidebar-scroll-section">
        <v-card
          class="sidebar-card sidebar-list-card"
          color="surface"
          variant="elevated"
          border
          rounded="xl"
          elevation="2"
        >
          <div class="sidebar-list-head">
            <span class="sidebar-section-title">{{ LABELS.tableList }}</span>
            <div class="sidebar-list-head-actions">
              <v-chip
                size="small"
                color="primary"
                variant="tonal"
                class="sidebar-count-chip"
              >
                {{ props.bootstrap?.tables.length ?? 0 }}
              </v-chip>
            </div>
          </div>

          <div class="sidebar-list-actions">
            <v-btn
              prepend-icon="mdi-table-plus"
              variant="tonal"
              color="primary"
              block
              class="sidebar-create-btn"
              @click="onOpenCreateDialog"
            >
              {{ LABELS.createNew }}
            </v-btn>
          </div>

          <v-list class="sidebar-list" nav density="comfortable" lines="two">
            <v-list-item
              v-for="table in props.bootstrap?.tables ?? []"
              :key="table.id"
              :active="table.id === selectedTableId"
              rounded="xl"
              class="sidebar-table-item"
              @click="onLoadTable(table.id)"
            >
              <template #prepend>
                <v-avatar size="36" color="primary" variant="tonal">
                  {{ tableInitial(table.displayName) }}
                </v-avatar>
              </template>
              <v-list-item-title class="sidebar-table-title">
                {{ table.displayName }}
              </v-list-item-title>
              <v-list-item-subtitle class="sidebar-table-subtitle">
                {{ table.tableName }}
              </v-list-item-subtitle>
              <template #append>
                <v-menu location="bottom end">
                  <template #activator="{ props: menuProps }">
                    <v-tooltip :text="LABELS.tableActions" location="bottom">
                      <template #activator="{ props: tooltipProps }">
                        <v-btn
                          v-bind="{ ...menuProps, ...tooltipProps }"
                          icon="mdi-dots-vertical"
                          variant="text"
                          density="comfortable"
                          :aria-label="LABELS.tableActions"
                          @click.stop
                        />
                      </template>
                    </v-tooltip>
                  </template>
                  <v-list density="compact">
                    <v-menu
                      open-on-hover
                      location="end"
                      transition="fade-transition"
                    >
                      <template #activator="{ props: importMenuProps }">
                        <v-list-item
                          v-bind="importMenuProps"
                          prepend-icon="mdi-file-import-outline"
                          append-icon="mdi-chevron-right"
                          @click.stop="handleImportTableCsv(table)"
                        >
                          <v-list-item-title> CSVインポート </v-list-item-title>
                        </v-list-item>
                      </template>
                      <v-list density="compact" min-width="280">
                        <v-list-item
                          v-for="item in CSV_IMPORT_MODE_ITEMS"
                          :key="item.mode"
                          :prepend-icon="
                            selectedImportMode === item.mode
                              ? 'mdi-check'
                              : 'mdi-blank'
                          "
                          @click.stop="handleImportTableCsv(table, item.mode)"
                        >
                          <v-list-item-title>
                            {{ csvImportModeLabel(item.mode) }}
                          </v-list-item-title>
                          <v-list-item-subtitle>
                            {{ csvImportModeDescription(item.mode) }}
                          </v-list-item-subtitle>
                        </v-list-item>
                      </v-list>
                    </v-menu>
                    <v-list-item
                      prepend-icon="mdi-file-export-outline"
                      @click.stop="handleExportTableCsv(table)"
                    >
                      <v-list-item-title> CSVエクスポート </v-list-item-title>
                    </v-list-item>
                    <v-list-item
                      base-color="error"
                      prepend-icon="mdi-delete-outline"
                      @click.stop="handleDeleteTable(table)"
                    >
                      <v-list-item-title>
                        {{ LABELS.deleteTable }}
                      </v-list-item-title>
                    </v-list-item>
                  </v-list>
                </v-menu>
              </template>
            </v-list-item>
          </v-list>

          <p
            v-if="(props.bootstrap?.tables.length ?? 0) === 0"
            class="sidebar-empty-hint"
          >
            {{ LABELS.emptyHint }}
          </p>
        </v-card>
      </div>
    </template>
  </div>
</template>
