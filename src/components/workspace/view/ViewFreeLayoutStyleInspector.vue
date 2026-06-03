<script setup lang="ts">
import type {
  LayoutStyleKey,
  LayoutStyleValue
} from "./ViewFreeLayoutCanvas.helpers";
import type { ViewLayoutCardItem } from "../../../types";

defineProps<{
  addTemplateSlot: (cardId: number) => void;
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
  cardBindingLabel: (layout: ViewLayoutCardItem, index: number) => string;
  cardPresetItems: Array<{ title: string; value: string | null }>;
  hasRecordOverrides: boolean;
  isTemplateMode: boolean;
  isTransparentBackgroundSelected: () => boolean;
  moveTemplateSlotDown: (cardId: number, index: number) => void;
  moveTemplateSlotUp: (cardId: number, index: number) => void;
  removeTemplateSlot: (cardId: number, index: number) => void;
  resetRecordOverrides: () => void;
  resetSelectedCardOverride: () => void;
  resetSelectedStyle: () => void;
  selectedCardHasOverride: boolean;
  selectedLayouts: ViewLayoutCardItem[];
  selectedPresetId: string | null | "";
  styleBooleanInputValue: (key: LayoutStyleKey) => boolean;
  styleInputValue: (key: LayoutStyleKey) => string;
  styleInspectorValues: Record<LayoutStyleKey, LayoutStyleValue | "">;
  styleNumberInputValue: (key: LayoutStyleKey) => number | "";
  themeColorInputValue: (tokenName: string) => string;
}>();
</script>

<template>
  <aside class="view-style-inspector" @pointerdown.stop @click.stop>
    <div v-if="isTemplateMode" class="view-template-preview-bindings open">
      <div class="view-template-preview-binding-heading">
        <div>
          <strong>表示スロット</strong>
          <span>カード内の表示枠だけを先に定義します</span>
        </div>
      </div>
      <div class="view-template-preview-binding-body">
        <p class="view-empty-hint mb-0">
          実カラムはまだ選ばず、表示スロット数と順序だけをテンプレートに保存します。
        </p>
        <div class="view-template-preview-binding-list">
          <div
            v-for="(layout, layoutIndex) in selectedLayouts"
            :key="layout.cardId"
            class="view-template-preview-binding"
          >
            <div class="view-binding-card-header">
              <span>{{ cardBindingLabel(layout, layoutIndex) }}</span>
              <v-btn
                size="x-small"
                variant="text"
                prepend-icon="mdi-plus"
                @click="addTemplateSlot(layout.cardId)"
              >
                スロット追加
              </v-btn>
            </div>
            <div class="view-binding-slot-list">
              <div
                v-for="(slot, slotIndex) in layout.slots"
                :key="`${layout.cardId}:${slot.slotId}`"
                class="view-binding-slot"
              >
                <div class="view-binding-slot-meta">
                  <span>スロット {{ slotIndex + 1 }}</span>
                  <small>未紐付け</small>
                </div>
                <div class="view-binding-slot-controls">
                  <div class="view-binding-slot-actions">
                    <v-btn
                      icon="mdi-chevron-up"
                      size="x-small"
                      variant="text"
                      :disabled="slotIndex === 0"
                      @click="moveTemplateSlotUp(layout.cardId, slotIndex)"
                    />
                    <v-btn
                      icon="mdi-chevron-down"
                      size="x-small"
                      variant="text"
                      :disabled="slotIndex >= layout.slots.length - 1"
                      @click="moveTemplateSlotDown(layout.cardId, slotIndex)"
                    />
                    <v-btn
                      icon="mdi-close"
                      size="x-small"
                      variant="text"
                      @click="removeTemplateSlot(layout.cardId, slotIndex)"
                    />
                  </div>
                </div>
              </div>
              <p v-if="layout.slots.length === 0" class="view-empty-hint mb-0">
                まだスロットはありません。
              </p>
            </div>
          </div>
          <p v-if="selectedLayouts.length === 0" class="view-empty-hint mb-0">
            スロットを編集するカードを選択してください。
          </p>
        </div>
      </div>
    </div>

    <div class="view-style-inspector-heading">
      <strong>スタイル</strong>
      <span>{{ selectedLayouts.length }}件</span>
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
          placeholder="混在"
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
              placeholder="混在"
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
              placeholder="混在"
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
              placeholder="混在"
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
              placeholder="混在"
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
          placeholder="混在"
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
          個別差分を全解除
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
