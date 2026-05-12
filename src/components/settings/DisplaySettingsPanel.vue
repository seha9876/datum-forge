<script setup lang="ts">
import { useDisplaySettings } from "../../composables/useDisplaySettings";

const {
  errorMessage,
  isToastVisible,
  loading,
  showRecordIdsInNavigation,
  toastMessage,
  updateRecordIdVisibility
} = useDisplaySettings();
</script>

<template>
  <!-- 表示カテゴリでは、画面の見え方や情報量に関する設定だけを扱います。 -->
  <section class="settings-section">
    <div class="settings-section-heading">
      <div>
        <h2>表示</h2>
        <p>画面に表示する情報量や見え方を調整します。</p>
      </div>
    </div>

    <v-alert
      v-if="errorMessage"
      type="error"
      variant="tonal"
      density="comfortable"
    >
      {{ errorMessage }}
    </v-alert>

    <div class="settings-form-grid">
      <v-card
        color="surface"
        variant="elevated"
        elevation="1"
        rounded="lg"
        border
        class="settings-operation pa-4"
      >
        <div class="settings-operation-heading">
          <h3>閲覧モードの目次</h3>
          <p>
            カスタム目次で、レコード名の前にレコードIDを表示するかを切り替えます。
          </p>
        </div>

        <v-switch
          color="primary"
          density="comfortable"
          hide-details
          inset
          :disabled="loading"
          :model-value="showRecordIdsInNavigation"
          label="目次にレコードIDを表示"
          @update:model-value="updateRecordIdVisibility"
        />
      </v-card>
    </div>

    <v-snackbar v-model="isToastVisible" color="success" timeout="2400">
      {{ toastMessage }}
    </v-snackbar>
  </section>
</template>
