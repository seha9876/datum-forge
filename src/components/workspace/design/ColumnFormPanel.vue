<script setup lang="ts">
import { computed, ref } from "vue";

import type { AddColumnPayload, AppBootstrap, FieldType } from "../../../types";

type FormRef = {
  validate: () => Promise<{ valid: boolean }>;
  resetValidation: () => void;
};

type ValidationRule = (value: unknown) => true | string;

const props = defineProps<{
  bootstrap: AppBootstrap | null;
  columnForm: AddColumnPayload;
  fieldTypes: FieldType[];
  fieldTypeLabel: (fieldType: FieldType) => string;
  onSubmitColumn: () => Promise<void>;
}>();

const formRef = ref<FormRef | null>(null);
const fieldTypeItems = computed(() =>
  props.fieldTypes.map((fieldType) => ({
    title: props.fieldTypeLabel(fieldType),
    value: fieldType
  }))
);

function isRequiredValueEmpty(value: unknown) {
  return (
    value === null ||
    value === undefined ||
    (typeof value === "string" && value.trim() === "")
  );
}

function requiredRule(label: string): ValidationRule {
  return (value: unknown) =>
    isRequiredValueEmpty(value) ? `${label} は必須です` : true;
}

function physicalNameRule(value: unknown) {
  if (typeof value !== "string") {
    return "物理名は文字列で入力してください";
  }

  return /^[A-Za-z][A-Za-z0-9_]*$/.test(value)
    ? true
    : "半角英字で始め、英数字と _ のみ使用できます";
}

async function handleSubmitColumn() {
  const validation = await formRef.value?.validate();
  if (!validation?.valid) {
    return;
  }

  await props.onSubmitColumn();
  formRef.value?.resetValidation();
}
</script>

<template>
  <v-form
    ref="formRef"
    class="column-form-stack"
    validate-on="submit lazy"
    @submit.prevent="handleSubmitColumn"
  >
    <!-- 追加するカラムの基本情報を入力するフォームです。 -->
    <v-text-field
      v-model="props.columnForm.columnName"
      label="物理名"
      placeholder="name"
      variant="outlined"
      density="comfortable"
      hint="半角英字で始め、英数字と _ のみ使用できます"
      persistent-hint
      :rules="[requiredRule('物理名'), physicalNameRule]"
      hide-details="auto"
      class="mb-4"
    />
    <v-text-field
      v-model="props.columnForm.displayName"
      label="表示名"
      placeholder="名前"
      variant="outlined"
      density="comfortable"
      :rules="[requiredRule('表示名')]"
      hide-details="auto"
      class="mb-4"
    />
    <v-select
      v-model="props.columnForm.fieldType"
      :items="fieldTypeItems"
      item-title="title"
      item-value="value"
      label="型"
      variant="outlined"
      density="comfortable"
      :rules="[requiredRule('型')]"
      hide-details="auto"
      class="mb-4"
    />

    <!-- 型に応じて追加設定が必要な項目だけを条件表示します。 -->
    <v-select
      v-if="props.columnForm.fieldType === 'single_select'"
      v-model="props.columnForm.selectOptionGroupId"
      :items="props.bootstrap?.optionGroups ?? []"
      item-title="name"
      item-value="id"
      label="単一選択グループ"
      variant="outlined"
      density="comfortable"
      :rules="[requiredRule('単一選択グループ')]"
      hide-details="auto"
      class="mb-4"
    />
    <v-select
      v-if="props.columnForm.fieldType === 'reference'"
      v-model="props.columnForm.refTableId"
      :items="props.bootstrap?.tables ?? []"
      item-title="displayName"
      item-value="id"
      label="参照テーブル"
      variant="outlined"
      density="comfortable"
      :rules="[requiredRule('参照テーブル')]"
      hide-details="auto"
      class="mb-4"
    />

    <!-- 必須入力の有無を切り替える設定です。 -->
    <div class="switch-grid">
      <v-switch
        v-model="props.columnForm.isRequired"
        label="必須にする"
        color="primary"
        hide-details
      />
    </div>

    <!-- カラム追加を確定する送信ボタンです。 -->
    <div class="toolbar mt-4">
      <span />
      <v-btn color="primary" type="submit">追加</v-btn>
    </div>
  </v-form>
</template>
