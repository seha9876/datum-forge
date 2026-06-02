<script setup lang="ts">
import type {
  LayoutStyleKey,
  LayoutStyleValue
} from "./ViewFreeLayoutCanvas.helpers";
import type {
  ViewLayoutCardItem,
  ViewLayoutTemplateCard
} from "../../../types";

defineProps<{
  addTemplatePreviewBindingSlot: (cardId: number) => void;
  applyBackgroundColorMode: (mode: "color" | "transparent") => void;
  applyFontWeightFromCheckbox: (value: boolean | null) => void;
  applyNumberStyleFromInput: (key: LayoutStyleKey, event: unknown) => void;
  applyPresetId: (presetId: string | null) => void;
  applySelectedStyle: (key: LayoutStyleKey, value: LayoutStyleValue) => void;
  applyShowLabelFromCheckbox: (value: boolean | null) => void;
  applyStyleFromInput: (key: LayoutStyleKey, event: unknown) => void;
  backgroundColorDisabled: boolean;
  backgroundColorDisabledReason: string;
  backgroundColorInputValue: () => string;
  bindingColumnItems: Array<{ title: string; value: number }>;
  cardBindingLabel: (layout: ViewLayoutCardItem, index: number) => string;
  cardPresetItems: Array<{ title: string; value: string | null }>;
  draftLayouts: ViewLayoutCardItem[];
  hasRecordOverrides: boolean;
  isTemplateMode: boolean;
  isTemplatePreviewActive: boolean;
  isTemplatePreviewBindingsOpen: boolean;
  isTransparentBackgroundSelected: () => boolean;
  moveTemplatePreviewBindingSlotDown: (cardId: number, index: number) => void;
  moveTemplatePreviewBindingSlotUp: (cardId: number, index: number) => void;
  removeTemplatePreviewBindingSlot: (cardId: number, index: number) => void;
  resetRecordOverrides: () => void;
  resetSelectedCardOverride: () => void;
  resetSelectedStyle: () => void;
  selectedCardHasOverride: boolean;
  selectedLayouts: ViewLayoutCardItem[];
  selectedPresetId: string | null | "";
  setTemplatePreviewBinding: (
    cardId: number,
    index: number,
    columnId: unknown
  ) => void;
  styleBooleanInputValue: (key: LayoutStyleKey) => boolean;
  styleInputValue: (key: LayoutStyleKey) => string;
  styleInspectorValues: Record<LayoutStyleKey, LayoutStyleValue | "">;
  styleNumberInputValue: (key: LayoutStyleKey) => number | "";
  templateCards: ViewLayoutTemplateCard[];
  templatePreviewBindingDraft: Record<number, Array<number | null>>;
  templatePreviewBoundCount: number;
  themeColorInputValue: (tokenName: string) => string;
  toggleTemplatePreviewBindings: () => void;
}>();
</script>

<template>
  <aside class="view-style-inspector" @pointerdown.stop @click.stop>
    <div
      v-if="isTemplateMode && isTemplatePreviewActive"
      class="view-template-preview-bindings"
      :class="{ open: isTemplatePreviewBindingsOpen }"
    >
      <div class="view-template-preview-binding-heading">
        <div>
          <strong>一時紐付け</strong>
          <span>
            {{ templatePreviewBoundCount }} / {{ templateCards.length }}
            カードで設定済み
          </span>
        </div>
        <v-tooltip
          :text="
            isTemplatePreviewBindingsOpen
              ? '一時紐付けを閉じる'
              : '一時紐付けを開く'
          "
          location="bottom"
        >
          <template #activator="{ props: tooltipProps }">
            <v-btn
              v-bind="tooltipProps"
              :icon="
                isTemplatePreviewBindingsOpen
                  ? 'mdi-chevron-down'
                  : 'mdi-chevron-right'
              "
              variant="text"
              size="small"
              :aria-expanded="isTemplatePreviewBindingsOpen"
              :aria-label="
                isTemplatePreviewBindingsOpen
                  ? '一時紐付けを閉じる'
                  : '一時紐付けを開く'
              "
              @click="toggleTemplatePreviewBindings"
            />
          </template>
        </v-tooltip>
      </div>
      <div
        v-if="isTemplatePreviewBindingsOpen"
        class="view-template-preview-binding-body"
      >
        <p class="view-empty-hint mb-0">
          プレビュー用の表示カラムです。カードごとに複数行を仮設定できます。
        </p>
        <div class="view-template-preview-binding-list">
          <div
            v-for="(layout, layoutIndex) in draftLayouts"
            :key="layout.cardId"
            class="view-template-preview-binding"
          >
            <div class="view-binding-card-header">
              <span>{{ cardBindingLabel(layout, layoutIndex) }}</span>
              <v-btn
                size="x-small"
                variant="text"
                prepend-icon="mdi-plus"
                @click="addTemplatePreviewBindingSlot(layout.cardId)"
              >
                行を追加
              </v-btn>
            </div>
            <div class="view-binding-slot-list">
              <div
                v-for="(columnId, bindingIndex) in templatePreviewBindingDraft[
                  layout.cardId
                ] ?? [null]"
                :key="`${layout.cardId}:${bindingIndex}`"
                class="view-binding-slot"
              >
                <div class="view-binding-slot-meta">
                  <span>{{ bindingIndex + 1 }} 行目</span>
                </div>
                <div class="view-binding-slot-controls">
                  <v-select
                    :items="bindingColumnItems"
                    :model-value="columnId ?? null"
                    item-title="title"
                    item-value="value"
                    label="表示カラム"
                    variant="outlined"
                    density="compact"
                    clearable
                    hide-details
                    @update:model-value="
                      setTemplatePreviewBinding(
                        layout.cardId,
                        bindingIndex,
                        $event
                      )
                    "
                  />
                  <div class="view-binding-slot-actions">
                    <v-btn
                      icon="mdi-chevron-up"
                      size="x-small"
                      variant="text"
                      :disabled="bindingIndex === 0"
                      @click="
                        moveTemplatePreviewBindingSlotUp(
                          layout.cardId,
                          bindingIndex
                        )
                      "
                    />
                    <v-btn
                      icon="mdi-chevron-down"
                      size="x-small"
                      variant="text"
                      :disabled="
                        bindingIndex >=
                        (templatePreviewBindingDraft[layout.cardId]?.length ??
                          1) -
                          1
                      "
                      @click="
                        moveTemplatePreviewBindingSlotDown(
                          layout.cardId,
                          bindingIndex
                        )
                      "
                    />
                    <v-btn
                      icon="mdi-close"
                      size="x-small"
                      variant="text"
                      :disabled="
                        (templatePreviewBindingDraft[layout.cardId]?.length ??
                          1) <= 1
                      "
                      @click="
                        removeTemplatePreviewBindingSlot(
                          layout.cardId,
                          bindingIndex
                        )
                      "
                    />
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
    <div class="view-style-inspector-heading">
      <strong>スタイル</strong>
      <span>{{ selectedLayouts.length }}枚</span>
    </div>

    <p v-if="selectedLayouts.length === 0" class="view-empty-hint">
      カードを選択してください。
    </p>
    <div v-else class="view-style-controls">
      <label v-if="isTemplateMode" class="view-style-control">
        <span>プリセット</span>
        <v-select
          :items="cardPresetItems"
          :model-value="selectedPresetId || null"
          item-title="title"
          item-value="value"
          label="カード見た目"
          variant="outlined"
          density="compact"
          hide-details
          @update:model-value="applyPresetId"
        />
      </label>
      <label class="view-style-control view-background-control">
        <span>背景色</span>
        <div class="view-background-row">
          <input
            type="color"
            :class="{ muted: isTransparentBackgroundSelected() }"
            :disabled="backgroundColorDisabled"
            :value="backgroundColorInputValue()"
            @input="applyStyleFromInput('backgroundColor', $event)"
          />
          <button
            type="button"
            class="view-background-transparent-button"
            :class="{ active: isTransparentBackgroundSelected() }"
            :disabled="backgroundColorDisabled"
            @click="applyBackgroundColorMode('transparent')"
          >
            透明
          </button>
        </div>
        <small v-if="backgroundColorDisabled" class="view-empty-hint">
          {{ backgroundColorDisabledReason }}
        </small>
      </label>
      <label class="view-style-control">
        <span>文字色</span>
        <input
          type="color"
          :value="
            styleInputValue('textColor') ||
            themeColorInputValue('--v-theme-on-surface')
          "
          @input="applyStyleFromInput('textColor', $event)"
        />
      </label>

      <label class="view-style-control">
        <span>文字サイズ</span>
        <input
          type="number"
          min="10"
          max="48"
          step="1"
          :value="styleNumberInputValue('fontSize')"
          placeholder="既定"
          @change="applyNumberStyleFromInput('fontSize', $event)"
        />
      </label>
      <div class="view-style-control view-padding-control">
        <span>余白</span>
        <div class="view-padding-grid">
          <label>
            <span>上</span>
            <input
              type="number"
              min="0"
              max="40"
              step="1"
              :value="styleNumberInputValue('paddingTop')"
              placeholder="既定"
              @change="applyNumberStyleFromInput('paddingTop', $event)"
            />
          </label>
          <label>
            <span>右</span>
            <input
              type="number"
              min="0"
              max="40"
              step="1"
              :value="styleNumberInputValue('paddingRight')"
              placeholder="既定"
              @change="applyNumberStyleFromInput('paddingRight', $event)"
            />
          </label>
          <label>
            <span>下</span>
            <input
              type="number"
              min="0"
              max="40"
              step="1"
              :value="styleNumberInputValue('paddingBottom')"
              placeholder="既定"
              @change="applyNumberStyleFromInput('paddingBottom', $event)"
            />
          </label>
          <label>
            <span>左</span>
            <input
              type="number"
              min="0"
              max="40"
              step="1"
              :value="styleNumberInputValue('paddingLeft')"
              placeholder="既定"
              @change="applyNumberStyleFromInput('paddingLeft', $event)"
            />
          </label>
        </div>
      </div>
      <label class="view-style-control">
        <span>角丸</span>
        <input
          type="number"
          min="0"
          max="40"
          step="1"
          :value="styleNumberInputValue('borderRadius')"
          placeholder="既定"
          @change="applyNumberStyleFromInput('borderRadius', $event)"
        />
      </label>

      <div class="view-style-control">
        <span>文字方向</span>
        <v-btn-toggle
          :model-value="styleInspectorValues.textDirection"
          mandatory
          divided
          color="primary"
          variant="tonal"
          rounded="lg"
          density="comfortable"
          class="view-style-segment d-flex ga-1 pa-1"
          @update:model-value="applySelectedStyle('textDirection', $event)"
        >
          <v-btn value="horizontal">横</v-btn>
          <v-btn value="vertical">縦</v-btn>
        </v-btn-toggle>
      </div>

      <div class="view-style-control">
        <span>文字揃え</span>
        <v-btn-toggle
          :model-value="styleInspectorValues.textAlign"
          mandatory
          divided
          color="primary"
          variant="tonal"
          rounded="lg"
          density="comfortable"
          class="view-style-segment d-flex ga-1 pa-1"
          @update:model-value="applySelectedStyle('textAlign', $event)"
        >
          <v-btn value="left">左</v-btn>
          <v-btn value="center">中央</v-btn>
          <v-btn value="right">右</v-btn>
        </v-btn-toggle>
      </div>
      <v-checkbox
        class="view-style-check"
        color="primary"
        density="compact"
        hide-details
        :model-value="styleBooleanInputValue('fontWeight')"
        label="太字"
        @update:model-value="applyFontWeightFromCheckbox"
      />

      <v-checkbox
        class="view-style-check"
        color="primary"
        density="compact"
        hide-details
        :model-value="styleBooleanInputValue('showLabel')"
        label="カラム名を表示"
        @update:model-value="applyShowLabelFromCheckbox"
      />
      <div v-if="!isTemplateMode" class="view-style-action-group">
        <button
          type="button"
          class="view-style-reset-button"
          :disabled="!selectedCardHasOverride"
          @click="resetSelectedCardOverride"
        >
          このカードだけ解除
        </button>
        <button
          type="button"
          class="view-style-reset-button"
          :disabled="!hasRecordOverrides"
          @click="resetRecordOverrides"
        >
          個別上書きを全解除
        </button>
      </div>
      <button
        type="button"
        class="view-style-reset-button"
        @click="resetSelectedStyle"
      >
        スタイルをリセット
      </button>
    </div>
  </aside>
</template>
