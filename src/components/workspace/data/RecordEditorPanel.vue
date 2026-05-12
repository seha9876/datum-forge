<script setup lang="ts">
import { ref } from "vue";

import type {
  AppBootstrap,
  AppColumn,
  FieldType,
  ReferenceChoice
} from "../../../types";

type FormRef = {
  validate: () => Promise<{ valid: boolean }>;
  resetValidation: () => void;
};

type ValidationRule = (value: unknown) => true | string;

/** レコード編集パネルで再利用する固定文言です。 */
const LABELS = {
  heading: "レコード編集",
  help: "フィールドタイプごとに入力値を切り替えて登録します。",
  cancel: "キャンセル",
  unselected: "未選択",
  noChoices: "候補がありません",
  selectReference: "参照先を選択",
  update: "更新",
  create: "登録"
} as const;

const props = defineProps<{
  bootstrap: AppBootstrap | null;
  columns: AppColumn[];
  editingRecordId: number | null;
  inputType: (fieldType: FieldType) => string;
  recordValues: Record<string, unknown>;
  referenceChoices: (column: AppColumn) => ReferenceChoice[];
  onCancelRecord: () => void;
  onSubmitRecord: () => Promise<boolean>;
}>();

const formRef = ref<FormRef | null>(null);

/** single_select カラムに紐づく選択肢グループを取得します。 */
function singleSelectOptions(column: AppColumn) {
  return (
    props.bootstrap?.optionGroups.find(
      (group) => group.id === column.selectOptionGroupId
    )?.options ?? []
  );
}

/** Vuetify の autocomplete が扱いやすい title/value 形式に変換します。 */
function singleSelectItems(column: AppColumn) {
  return singleSelectOptions(column).map((option) => ({
    title: option.label,
    value: option.optionNo
  }));
}

/** 参照カラム用の候補一覧を title/value 形式に変換します。 */
function referenceSelectItems(column: AppColumn) {
  return props.referenceChoices(column).map((choice) => ({
    title: choice.label,
    value: choice.id
  }));
}

function fieldPlaceholder(column: AppColumn) {
  if (column.fieldType === "single_select") {
    return LABELS.unselected;
  }

  if (column.fieldType === "reference") {
    return LABELS.selectReference;
  }

  return column.columnName;
}

function isRequiredValueEmpty(value: unknown) {
  return (
    value === null ||
    value === undefined ||
    (typeof value === "string" && value.trim() === "")
  );
}

function requiredRules(column: AppColumn): ValidationRule[] {
  if (!column.isRequired) {
    return [];
  }

  return [
    (value: unknown) =>
      isRequiredValueEmpty(value) ? `${column.displayName} は必須です` : true
  ];
}

async function handleSubmitRecord() {
  const validation = await formRef.value?.validate();
  if (!validation?.valid) {
    return false;
  }

  return props.onSubmitRecord();
}

function handleCancelRecord() {
  formRef.value?.resetValidation();
  props.onCancelRecord();
}
</script>

<template>
  <!-- レコード追加・編集フォームです。カラムの型に合わせて入力部品を切り替えます。 -->
  <v-form
    ref="formRef"
    class="column-form-stack"
    validate-on="submit lazy"
    @submit.prevent="handleSubmitRecord"
  >
    <!-- フォームの見出しです。 -->
    <div class="section-header">
      <div>
        <h2>{{ LABELS.heading }}</h2>
        <p class="help-text">{{ LABELS.help }}</p>
      </div>
    </div>

    <!-- id 以外の各カラムを、入力フィールドとして繰り返し描画します。 -->
    <div
      v-for="column in props.columns.filter((item) => item.columnName !== 'id')"
      :key="column.id"
      class="record-field"
    >
      <div class="record-field-label">
        <span>{{ column.displayName }}</span>
        <v-chip
          v-if="column.isRequired"
          color="error"
          size="x-small"
          variant="tonal"
        >
          必須
        </v-chip>
      </div>

      <!-- single_select は事前登録された選択肢から選びます。 -->
      <v-autocomplete
        v-if="column.fieldType === 'single_select'"
        v-model="props.recordValues[column.columnName]"
        :items="singleSelectItems(column)"
        item-title="title"
        item-value="value"
        variant="outlined"
        density="comfortable"
        clearable
        :rules="requiredRules(column)"
        hide-details="auto"
        :no-data-text="LABELS.noChoices"
        :placeholder="fieldPlaceholder(column)"
        class="record-reference-autocomplete"
      />

      <!-- reference は別テーブルのレコード候補から選びます。 -->
      <v-autocomplete
        v-else-if="column.fieldType === 'reference'"
        v-model="props.recordValues[column.columnName]"
        :items="referenceSelectItems(column)"
        item-title="title"
        item-value="value"
        variant="outlined"
        density="comfortable"
        clearable
        :rules="requiredRules(column)"
        hide-details="auto"
        :no-data-text="LABELS.noChoices"
        :placeholder="fieldPlaceholder(column)"
        class="record-reference-autocomplete"
      />

      <!-- boolean はチェックボックスで true / false を入力します。 -->
      <v-checkbox
        v-else-if="column.fieldType === 'boolean'"
        v-model="props.recordValues[column.columnName]"
        color="primary"
        density="comfortable"
        :rules="requiredRules(column)"
        hide-details="auto"
        :label="column.displayName"
      />

      <!-- それ以外の型は Vuetify のテキスト入力で値を受け取ります。 -->
      <v-text-field
        v-else
        v-model="props.recordValues[column.columnName]"
        density="comfortable"
        :rules="requiredRules(column)"
        hide-details="auto"
        :placeholder="fieldPlaceholder(column)"
        :type="props.inputType(column.fieldType)"
        variant="outlined"
      />
    </div>

    <!-- 編集キャンセルと保存実行のボタンです。 -->
    <div class="row-actions">
      <v-btn variant="text" @click="handleCancelRecord">
        {{ LABELS.cancel }}
      </v-btn>
      <v-btn color="primary" type="submit" variant="flat">
        {{ props.editingRecordId ? LABELS.update : LABELS.create }}
      </v-btn>
    </div>
  </v-form>
</template>
