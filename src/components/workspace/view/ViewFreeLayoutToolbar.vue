<script setup lang="ts">
import { VIEWPORT_ZOOM_STEP } from "./ViewFreeLayoutCanvas.helpers";

import type { TemplatePreviewRecordSelection } from "../../../types";

defineProps<{
  canEditBindings: boolean;
  editMode: boolean;
  isBindingEditorVisible: boolean;
  isTemplateMode: boolean;
  isTemplatePreviewActive: boolean;
  saving: boolean;
  selectedTemplatePreviewRecordKey: string | null;
  templateName: string;
  templatePreviewLabel: string;
  templatePreviewLoading: boolean;
  templatePreviewMenuOpen: boolean;
  templatePreviewRecordItems: Array<{
    title: string;
    value: string;
    record: TemplatePreviewRecordSelection;
  }>;
  viewportPercent: number;
}>();

const emit = defineEmits<{
  "add-template-card": [];
  "clear-template-preview": [];
  "fit-viewport": [];
  "reset-viewport": [];
  "select-template-preview-record": [string | null];
  "toggle-binding-editor": [];
  "update:edit-mode": [boolean | null];
  "update:template-preview-menu-open": [boolean];
  "zoom-from-center": [number];
}>();
</script>

<template>
  <div class="section-header">
    <div>
      <template v-if="isTemplateMode">
        <h2>テンプレート編集</h2>
        <p class="help-text">
          {{ templateName }} のカード配置と見た目を編集します。
        </p>
      </template>
      <template v-else>
        <h2>自由配置ビュー</h2>
        <p class="help-text">
          このレコードだけの表示位置や見た目を調整できます。
        </p>
      </template>
    </div>
    <div class="view-layout-toolbar">
      <div class="view-viewport-toolbar" aria-label="キャンバス表示操作">
        <v-tooltip text="縮小" location="bottom">
          <template #activator="{ props: tooltipProps }">
            <button
              v-bind="tooltipProps"
              type="button"
              @click="emit('zoom-from-center', -VIEWPORT_ZOOM_STEP)"
            >
              -
            </button>
          </template>
        </v-tooltip>
        <v-tooltip text="ズームをリセット" location="bottom">
          <template #activator="{ props: tooltipProps }">
            <button
              v-bind="tooltipProps"
              type="button"
              @click="emit('reset-viewport')"
            >
              {{ viewportPercent }}%
            </button>
          </template>
        </v-tooltip>
        <v-tooltip text="拡大" location="bottom">
          <template #activator="{ props: tooltipProps }">
            <button
              v-bind="tooltipProps"
              type="button"
              @click="emit('zoom-from-center', VIEWPORT_ZOOM_STEP)"
            >
              +
            </button>
          </template>
        </v-tooltip>
        <v-tooltip text="全体表示" location="bottom">
          <template #activator="{ props: tooltipProps }">
            <button
              v-bind="tooltipProps"
              type="button"
              @click="emit('fit-viewport')"
            >
              全体表示
            </button>
          </template>
        </v-tooltip>
      </div>
      <span class="view-layout-save-state">
        {{ saving ? "保存中..." : "保存済み" }}
      </span>
      <v-menu
        v-if="isTemplateMode"
        :model-value="templatePreviewMenuOpen"
        :close-on-content-click="false"
        location="bottom end"
        @update:model-value="emit('update:template-preview-menu-open', $event)"
      >
        <template #activator="{ props: menuProps }">
          <v-btn
            v-bind="menuProps"
            prepend-icon="mdi-eye-outline"
            color="primary"
            variant="tonal"
            size="small"
            :loading="templatePreviewLoading"
          >
            プレビュー
          </v-btn>
        </template>
        <v-card class="view-template-preview-menu" rounded="lg">
          <v-card-title>プレビューするデータ</v-card-title>
          <v-card-text>
            <v-autocomplete
              :items="templatePreviewRecordItems"
              :model-value="selectedTemplatePreviewRecordKey"
              item-title="title"
              item-value="value"
              label="テーブル / レコード"
              variant="outlined"
              density="comfortable"
              hide-details
              :disabled="templatePreviewLoading"
              @update:model-value="
                emit('select-template-preview-record', $event)
              "
            />
          </v-card-text>
        </v-card>
      </v-menu>
      <v-chip
        v-if="isTemplatePreviewActive"
        class="view-template-preview-chip"
        color="primary"
        variant="tonal"
        closable
        @click:close="emit('clear-template-preview')"
      >
        プレビュー中: {{ templatePreviewLabel }}
      </v-chip>
      <v-btn
        v-if="canEditBindings"
        prepend-icon="mdi-card-bulleted-settings-outline"
        color="primary"
        :variant="isBindingEditorVisible ? 'flat' : 'tonal'"
        size="small"
        @click="emit('toggle-binding-editor')"
      >
        テンプレート設定
      </v-btn>
      <v-btn
        v-if="isTemplateMode"
        prepend-icon="mdi-plus"
        color="primary"
        variant="tonal"
        size="small"
        @click="emit('add-template-card')"
      >
        カード追加
      </v-btn>
      <v-switch
        :model-value="editMode"
        :disabled="isBindingEditorVisible"
        hide-details
        density="compact"
        color="primary"
        label="編集"
        @update:model-value="emit('update:edit-mode', $event)"
      />
    </div>
  </div>
</template>
