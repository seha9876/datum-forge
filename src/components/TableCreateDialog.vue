<script setup lang="ts">
const props = defineProps<{
  modelValue: boolean;
  tableForm: { tableName: string; displayName: string };
  onSubmitTable: () => Promise<void>;
}>();

const emit = defineEmits<{
  "update:modelValue": [boolean];
}>();

/**
 * テーブル作成を実行し、成功後にダイアログを閉じます。
 */
async function handleSubmit() {
  await props.onSubmitTable();
  emit("update:modelValue", false);
}
</script>

<template>
  <!-- テーブル新規作成フォームを表示するモーダルです。 -->
  <v-dialog
    :model-value="modelValue"
    max-width="520"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <v-card rounded="xl">
      <v-card-title>テーブル作成</v-card-title>
      <v-card-text>
        <!-- テーブルの物理名と表示名を入力します。 -->
        <v-text-field
          v-model="tableForm.tableName"
          label="テーブル名"
          placeholder="characters"
          variant="outlined"
          density="comfortable"
          class="mb-4"
        />
        <v-text-field
          v-model="tableForm.displayName"
          label="表示名"
          placeholder="キャラクター"
          variant="outlined"
          density="comfortable"
        />
      </v-card-text>
      <!-- ダイアログを閉じるか、テーブル作成を確定する操作群です。 -->
      <v-card-actions class="justify-end">
        <v-btn variant="text" @click="emit('update:modelValue', false)">
          キャンセル
        </v-btn>
        <v-btn color="primary" @click="handleSubmit">作成</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>
