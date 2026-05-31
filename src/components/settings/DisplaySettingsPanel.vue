<script setup lang="ts">
import { useDisplaySettings } from "../../composables/useDisplaySettings";

const {
  commonDurationSeconds,
  errorDurationSeconds,
  errorMessage,
  isToastVisible,
  loading,
  saveNotificationSettings,
  showRecordIdsInNavigation,
  successDurationSeconds,
  toastMessage,
  updateRecordIdVisibility,
  usePerKindDurations,
  warningDurationSeconds
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

      <v-card
        color="surface"
        variant="elevated"
        elevation="1"
        rounded="lg"
        border
        class="settings-operation pa-4"
      >
        <div class="settings-operation-heading">
          <h3>通知</h3>
          <p>
            Snackbar通知が自動で閉じるまでの秒数を設定します。0秒にすると自動で閉じません。
          </p>
        </div>

        <div class="settings-field">
          <span class="settings-label">通知の表示時間</span>
          <v-text-field
            v-model="commonDurationSeconds"
            type="number"
            min="0"
            max="60"
            step="1"
            suffix="秒"
            hide-details="auto"
            variant="outlined"
            density="comfortable"
            :disabled="loading || usePerKindDurations"
          />
          <p v-if="usePerKindDurations" class="settings-help-text">
            個別設定中は、下の成功・警告・エラーの秒数を使用します。
          </p>
        </div>

        <v-switch
          v-model="usePerKindDurations"
          color="primary"
          density="comfortable"
          hide-details
          inset
          :disabled="loading"
          label="種類ごとに個別設定する"
        />

        <v-expand-transition>
          <div v-if="usePerKindDurations" class="settings-form-grid">
            <div class="settings-field">
              <span class="settings-label">成功</span>
              <v-text-field
                v-model="successDurationSeconds"
                type="number"
                min="0"
                max="60"
                step="1"
                suffix="秒"
                hide-details="auto"
                variant="outlined"
                density="comfortable"
                :disabled="loading"
              />
            </div>

            <div class="settings-field">
              <span class="settings-label">警告</span>
              <v-text-field
                v-model="warningDurationSeconds"
                type="number"
                min="0"
                max="60"
                step="1"
                suffix="秒"
                hide-details="auto"
                variant="outlined"
                density="comfortable"
                :disabled="loading"
              />
            </div>

            <div class="settings-field">
              <span class="settings-label">エラー</span>
              <v-text-field
                v-model="errorDurationSeconds"
                type="number"
                min="0"
                max="60"
                step="1"
                suffix="秒"
                hide-details="auto"
                variant="outlined"
                density="comfortable"
                :disabled="loading"
              />
            </div>
          </div>
        </v-expand-transition>

        <div class="settings-actions">
          <v-btn
            color="primary"
            prepend-icon="mdi-content-save-outline"
            :disabled="loading"
            :loading="loading"
            @click="saveNotificationSettings"
          >
            通知設定を保存
          </v-btn>
        </div>
      </v-card>
    </div>

    <v-snackbar v-model="isToastVisible" color="success" timeout="2400">
      {{ toastMessage }}
    </v-snackbar>
  </section>
</template>
