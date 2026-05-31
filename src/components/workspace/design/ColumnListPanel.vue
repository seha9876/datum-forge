<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { VueDraggable } from "vue-draggable-plus";

import { useConfirmDialog } from "../../../composables/useConfirmDialog";

import type { AppColumn, TableDetail } from "../../../types";

type FormRef = {
  validate: () => Promise<{ valid: boolean }>;
  resetValidation: () => void;
};

type ValidationRule = (value: unknown) => true | string;

const props = defineProps<{
  columns: AppColumn[];
  editingColumn: {
    id: number | null;
    columnName: string;
    displayName: string;
    isRequired: boolean;
  };
  fieldTypeLabel: (fieldType: AppColumn["fieldType"]) => string;
  fieldTypeMeta: (column: AppColumn) => string;
  selectedTable: TableDetail;
  onCancelEdit: () => void;
  onDeleteColumn: (columnId: number) => Promise<void>;
  onReorderColumns: (columns: AppColumn[]) => Promise<void>;
  onStartEdit: (column: AppColumn) => void;
  onSubmitEdit: () => Promise<void>;
  onUpdateLabelColumn: (labelColumnId: number | null) => Promise<void>;
}>();

const confirmDialog = useConfirmDialog();
const editFormRef = ref<FormRef | null>(null);

/** ID カラムを除いたドラッグ対象カラム一覧です。 */
const draggableColumns = ref<AppColumn[]>([]);

watch(
  () => props.columns,
  (columns) => {
    // 固定の ID カラムだけはドラッグ対象から除外します。
    draggableColumns.value = columns.filter(
      (column) => column.columnName !== "id"
    );
  },
  { immediate: true }
);

/** 並び替え対象から除外して先頭固定にする ID カラムです。 */
const idColumn = computed(
  () => props.columns.find((column) => column.columnName === "id") ?? null
);

/** 主表示カラムの選択肢を Vuetify の select 用形式にします。 */
const labelColumnItems = computed(() => [
  { title: "未設定", value: null },
  ...props.columns
    .filter((column) => column.columnName !== "id")
    .map((column) => ({
      title: `${column.displayName} (${column.columnName})`,
      value: column.id
    }))
]);

/**
 * 主表示カラムの変更値を受け取り、未設定も含めて保存します。
 *
 * @param labelColumnId 選択された主表示カラム ID
 */
function handleLabelColumnChange(labelColumnId: number | null) {
  void props.onUpdateLabelColumn(labelColumnId);
}

/**
 * Vuetify の rules で空欄を判定するため、文字列以外の null/undefined もまとめて扱います。
 */
function isRequiredValueEmpty(value: unknown) {
  return (
    value === null ||
    value === undefined ||
    (typeof value === "string" && value.trim() === "")
  );
}

/**
 * 表示名や物理名など、編集時に空欄へできない項目の共通ルールです。
 */
function requiredRule(label: string): ValidationRule {
  return (value: unknown) =>
    isRequiredValueEmpty(value) ? `${label} は必須です` : true;
}

/**
 * 物理名は SQLite の列名として使うため、バックエンドの識別子ルールと同じ条件で保存前に止めます。
 */
function physicalNameRule(value: unknown) {
  if (typeof value !== "string") {
    return "物理名は文字列で入力してください";
  }

  return /^[A-Za-z][A-Za-z0-9_]*$/.test(value)
    ? true
    : "半角英字で始め、英数字と _ のみ使用できます";
}

/**
 * 編集行の入力を検証してから、親コンポーネントの保存処理へ進みます。
 */
async function handleSubmitEdit() {
  const validation = await editFormRef.value?.validate();
  if (!validation?.valid) {
    return;
  }

  await props.onSubmitEdit();
  editFormRef.value?.resetValidation();
}

/**
 * 削除確認を行ったうえでカラムを削除します。
 *
 * @param column 削除対象カラム
 */
async function handleDeleteColumn(column: AppColumn) {
  const confirmed = await confirmDialog.open({
    title: "カラムの削除",
    message: `カラム「${column.displayName} (${column.columnName})」を削除しますか？`,
    confirmText: "削除",
    color: "error"
  });
  if (!confirmed) {
    return;
  }

  if (props.editingColumn.id === column.id) {
    props.onCancelEdit();
  }

  await props.onDeleteColumn(column.id);
}

/**
 * ID カラムを先頭に戻した状態で並び替え結果を保存します。
 */
async function handleDragEnd() {
  const orderedColumns = [
    ...(idColumn.value ? [idColumn.value] : []),
    ...draggableColumns.value
  ];
  await props.onReorderColumns(orderedColumns);
}
</script>

<template>
  <v-card
    tag="section"
    color="surface"
    variant="elevated"
    rounded="xl"
    elevation="2"
    border
    class="pa-4"
  >
    <!-- カラム一覧パネルの見出しです。 -->
    <div class="section-heading">
      <div class="section-header">
        <div>
          <h2>カラム一覧</h2>
          <p class="help-text">
            ドラッグ&amp;ドロップで表示順を変更できます。並びと項目の関連情報もここで確認できます。
          </p>
        </div>
        <v-chip size="small" color="primary" variant="tonal">
          カラム {{ draggableColumns.length }}
        </v-chip>
      </div>
    </div>

    <!-- テーブルの主表示カラムを選択する設定欄です。 -->
    <v-select
      density="comfortable"
      hide-details
      :items="labelColumnItems"
      label="主表示カラム"
      :model-value="selectedTable.table.labelColumnId ?? null"
      variant="outlined"
      @update:model-value="handleLabelColumnChange"
    />

    <!-- カラム一覧の列見出しです。 -->
    <div class="column-list-head">
      <span>並び順</span>
      <span>カラム</span>
      <span>型</span>
      <span>設定</span>
      <span>操作</span>
    </div>

    <!-- ID カラムは並び替え対象外として固定表示します。 -->
    <div v-if="idColumn" class="column-row fixed">
      <div class="drag-handle disabled">::</div>
      <div class="column-main">
        <strong>{{ idColumn.displayName }}</strong>
        <small>{{ idColumn.columnName }}</small>
      </div>
      <div>{{ fieldTypeLabel(idColumn.fieldType) }}</div>
      <div class="column-meta">
        <v-chip
          v-if="selectedTable.table.labelColumnId === idColumn.id"
          size="small"
          color="primary"
          variant="tonal"
        >
          主表示
        </v-chip>
      </div>
      <div />
    </div>

    <!-- ID 以外のカラムをドラッグで並び替えできる一覧です。 -->
    <VueDraggable
      v-model="draggableColumns"
      class="column-list"
      handle=".drag-handle"
      item-key="id"
      ghost-class="option-card-ghost"
      chosen-class="option-card-chosen"
      drag-class="option-card-drag"
      :animation="0"
      :force-fallback="true"
      :fallback-on-body="true"
      @end="handleDragEnd"
    >
      <div
        v-for="column in draggableColumns"
        :key="column.id"
        class="column-row"
      >
        <template v-if="editingColumn.id === column.id">
          <!-- 編集中の行だけは入力フォームに切り替えます。 -->
          <v-tooltip text="並び順を変更" location="bottom">
            <template #activator="{ props: tooltipProps }">
              <button
                v-bind="tooltipProps"
                class="drag-handle"
                type="button"
                aria-label="並び順を変更"
              >
                ::
              </button>
            </template>
          </v-tooltip>
          <v-form
            ref="editFormRef"
            class="column-main"
            validate-on="submit lazy"
            @submit.prevent="handleSubmitEdit"
          >
            <v-text-field
              v-model="editingColumn.displayName"
              density="compact"
              hide-details="auto"
              label="表示名"
              :rules="[requiredRule('表示名')]"
              variant="outlined"
            />
            <v-text-field
              v-model="editingColumn.columnName"
              density="compact"
              hide-details="auto"
              hint="半角英字で始め、英数字と _ のみ使用できます"
              label="物理名"
              placeholder="name"
              :rules="[requiredRule('物理名'), physicalNameRule]"
              variant="outlined"
            />
          </v-form>
          <div>{{ fieldTypeLabel(column.fieldType) }}</div>
          <div class="column-meta">
            <span>{{ fieldTypeMeta(column) }}</span>
            <v-switch
              v-model="editingColumn.isRequired"
              color="primary"
              density="compact"
              hide-details
              label="必須"
            />
          </div>
          <div class="row-actions">
            <v-btn
              color="primary"
              density="comfortable"
              size="small"
              @click="handleSubmitEdit"
              >保存</v-btn
            >
            <v-btn
              density="comfortable"
              size="small"
              variant="text"
              @click="onCancelEdit"
              >キャンセル</v-btn
            >
          </div>
        </template>

        <template v-else>
          <!-- 通常時は表示用レイアウトと操作ボタンを表示します。 -->
          <v-tooltip text="並び順を変更" location="bottom">
            <template #activator="{ props: tooltipProps }">
              <button
                v-bind="tooltipProps"
                class="drag-handle"
                type="button"
                aria-label="並び順を変更"
              >
                ::
              </button>
            </template>
          </v-tooltip>
          <div class="column-main">
            <strong>{{ column.displayName }}</strong>
            <small>{{ column.columnName }}</small>
          </div>
          <div>{{ fieldTypeLabel(column.fieldType) }}</div>
          <div class="column-meta">
            <span>{{ fieldTypeMeta(column) }}</span>
            <v-chip
              v-if="selectedTable.table.labelColumnId === column.id"
              size="small"
              color="primary"
              variant="tonal"
            >
              主表示
            </v-chip>
            <v-chip
              v-if="column.isRequired"
              size="small"
              color="error"
              variant="tonal"
            >
              必須
            </v-chip>
          </div>
          <div class="row-actions">
            <v-btn
              density="comfortable"
              size="small"
              variant="tonal"
              @click="onStartEdit(column)"
              >編集</v-btn
            >
            <v-btn
              color="error"
              density="comfortable"
              size="small"
              variant="tonal"
              @click="handleDeleteColumn(column)"
            >
              削除
            </v-btn>
          </div>
        </template>
      </div>
    </VueDraggable>
  </v-card>
</template>
