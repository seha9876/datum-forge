<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { VueDraggable } from "vue-draggable-plus";

import type {
  LayoutStyleKey,
  LayoutStyleValue
} from "./ViewFreeLayoutCanvas.helpers";
import type { ViewLayoutCardItem } from "../../../types";

type TemplateSlotListItem = {
  autoName: string;
  cardId: number;
  isBound: boolean;
  label: string;
  slotId: number;
  statusLabel: string;
};

const props = defineProps<{
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
  cardPresetItems: Array<{ title: string; value: string | null }>;
  hasRecordOverrides: boolean;
  isTemplateMode: boolean;
  isTransparentBackgroundSelected: () => boolean;
  removeTemplateSlot: (cardId: number, index: number) => void;
  reorderTemplateSlots: (cardId: number, orderedSlotIds: number[]) => void;
  resetRecordOverrides: () => void;
  resetSelectedCardOverride: () => void;
  resetSelectedStyle: () => void;
  selectedCardHasOverride: boolean;
  selectedLayouts: ViewLayoutCardItem[];
  selectedPresetId: string | null | "";
  selectedSlotItem: TemplateSlotListItem | null;
  selectedTemplateSlotKey: string | null;
  selectTemplateSlot: (cardId: number, slotId: number) => void;
  styleBooleanInputValue: (key: LayoutStyleKey) => boolean;
  styleInputValue: (key: LayoutStyleKey) => string;
  styleInspectorValues: Record<LayoutStyleKey, LayoutStyleValue | "">;
  styleNumberInputValue: (key: LayoutStyleKey) => number | "";
  templateSlotItems: TemplateSlotListItem[];
  templateSlotTargetCardLabel: string;
  themeColorInputValue: (tokenName: string) => string;
}>();

const draggableSlotItems = ref<TemplateSlotListItem[]>([]);
const isTemplateSlotSectionExpanded = ref(false);

watch(
  () => props.templateSlotItems,
  (slotItems) => {
    draggableSlotItems.value = [...slotItems];
  },
  { immediate: true }
);

const canAddTemplateSlot = computed(() => props.selectedLayouts.length > 0);
const templateSlotCount = computed(() => props.templateSlotItems.length);
const selectedSlotStatusMeta = computed(() =>
  props.selectedSlotItem?.isBound && props.selectedSlotItem.label
    ? `表示カラム: ${props.selectedSlotItem.label}`
    : "表示カラムはまだ紐付いていません"
);

function handleTemplateSlotDragEnd() {
  const [firstSlot] = draggableSlotItems.value;
  if (!firstSlot) {
    return;
  }

  props.reorderTemplateSlots(
    firstSlot.cardId,
    draggableSlotItems.value.map((slot) => slot.slotId)
  );
}

function addTemplateSlotFromSelection() {
  const [firstLayout] = props.selectedLayouts;
  if (!firstLayout) {
    return;
  }

  props.addTemplateSlot(firstLayout.cardId);
}

function toggleTemplateSlotSection() {
  isTemplateSlotSectionExpanded.value = !isTemplateSlotSectionExpanded.value;
}

function chipTitle(slot: TemplateSlotListItem) {
  return `${slot.autoName} / ${slot.statusLabel}`;
}

function chipShortLabel(index: number) {
  return String(index + 1);
}
</script>

<template>
  <aside class="view-style-inspector" @pointerdown.stop @click.stop>
    <div
      v-if="isTemplateMode"
      class="view-template-preview-bindings"
      :class="{ open: isTemplateSlotSectionExpanded }"
    >
      <div class="view-slot-card-summary">
        <strong>{{ templateSlotTargetCardLabel || "カード未選択" }}</strong>
        <span>スロット {{ templateSlotCount }}件</span>
      </div>

      <div class="view-template-preview-binding-heading">
        <button
          type="button"
          class="view-slot-section-toggle"
          :aria-expanded="isTemplateSlotSectionExpanded"
          @click="toggleTemplateSlotSection"
        >
          <span class="view-slot-section-toggle-copy">
            <strong>表示スロット</strong>
          </span>
          <span class="view-slot-section-toggle-action">
            {{ isTemplateSlotSectionExpanded ? "一覧を閉じる" : "一覧を開く" }}
            <v-icon
              :icon="
                isTemplateSlotSectionExpanded
                  ? 'mdi-chevron-up'
                  : 'mdi-chevron-down'
              "
              size="18"
            />
          </span>
        </button>
      </div>

      <div class="view-template-preview-binding-body">
        <div
          v-if="!isTemplateSlotSectionExpanded"
          class="view-slot-chip-rail-wrap"
        >
          <div
            v-if="templateSlotItems.length > 0"
            class="view-slot-chip-rail"
            role="listbox"
            aria-label="表示スロット一覧"
          >
            <button
              v-for="(slot, slotIndex) in templateSlotItems"
              :key="slot.slotId"
              type="button"
              class="view-slot-chip"
              :class="{
                selected:
                  selectedTemplateSlotKey === `${slot.cardId}:${slot.slotId}`
              }"
              :title="chipTitle(slot)"
              @click="selectTemplateSlot(slot.cardId, slot.slotId)"
            >
              <span class="view-slot-chip-index">{{
                chipShortLabel(slotIndex)
              }}</span>
              <span
                class="view-slot-chip-badge"
                :class="{ bound: slot.isBound }"
              >
                {{ slot.isBound ? "済" : "未" }}
              </span>
            </button>
            <button
              type="button"
              class="view-slot-chip add-chip"
              :disabled="!canAddTemplateSlot"
              aria-label="スロット追加"
              @click="addTemplateSlotFromSelection"
            >
              <v-icon icon="mdi-plus" size="16" />
            </button>
          </div>
          <div v-else class="view-slot-chip-empty">
            <span>まだスロットはありません。</span>
            <v-btn
              size="small"
              variant="text"
              prepend-icon="mdi-plus"
              :disabled="!canAddTemplateSlot"
              @click="addTemplateSlotFromSelection"
            >
              スロット追加
            </v-btn>
          </div>
        </div>

        <div v-else class="view-slot-panel-open">
          <div class="view-slot-panel-actions">
            <v-btn
              size="small"
              variant="tonal"
              prepend-icon="mdi-plus"
              :disabled="!canAddTemplateSlot"
              @click="addTemplateSlotFromSelection"
            >
              スロット追加
            </v-btn>
          </div>
          <VueDraggable
            v-if="draggableSlotItems.length > 0"
            v-model="draggableSlotItems"
            class="view-slot-compact-list"
            handle=".view-slot-row-handle"
            item-key="slotId"
            ghost-class="view-slot-row-ghost"
            chosen-class="view-slot-row-chosen"
            drag-class="view-slot-row-drag"
            :animation="0"
            :force-fallback="true"
            :fallback-on-body="true"
            @end="handleTemplateSlotDragEnd"
          >
            <button
              v-for="(slot, slotIndex) in draggableSlotItems"
              :key="slot.slotId"
              type="button"
              class="view-slot-row"
              :class="{
                selected:
                  selectedTemplateSlotKey === `${slot.cardId}:${slot.slotId}`
              }"
              @click="selectTemplateSlot(slot.cardId, slot.slotId)"
            >
              <span
                class="view-slot-row-handle"
                aria-label="スロット順を変更"
                title="ドラッグで順序変更"
              >
                <v-icon icon="mdi-drag-vertical" size="16" />
              </span>
              <span class="view-slot-row-main">
                <strong :title="slot.autoName">{{ slot.autoName }}</strong>
                <small :title="slot.label">{{ slot.label }}</small>
              </span>
              <v-chip
                size="x-small"
                variant="tonal"
                :color="slot.isBound ? 'primary' : 'default'"
                class="view-slot-row-status"
              >
                {{ slot.statusLabel }}
              </v-chip>
              <v-btn
                icon="mdi-close"
                size="x-small"
                variant="text"
                :aria-label="`${slot.autoName} を削除`"
                @click.stop="removeTemplateSlot(slot.cardId, slotIndex)"
              />
            </button>
          </VueDraggable>
          <p v-else class="view-empty-hint mb-0">
            まだスロットはありません。カードを選択して追加してください。
          </p>
        </div>

        <section class="view-slot-detail-panel">
          <div class="view-slot-detail-heading">
            <strong>選択中スロット設定</strong>
            <span>{{
              selectedSlotItem ? selectedSlotItem.autoName : "未選択"
            }}</span>
          </div>

          <template v-if="selectedSlotItem">
            <div class="view-slot-detail-grid">
              <div class="view-slot-detail-row">
                <span>表示名</span>
                <strong>{{ selectedSlotItem.autoName }}</strong>
              </div>
              <div class="view-slot-detail-row">
                <span>紐付け状態</span>
                <strong>{{ selectedSlotItem.statusLabel }}</strong>
              </div>
              <div class="view-slot-detail-row wide">
                <span>メモ</span>
                <small>{{ selectedSlotStatusMeta }}</small>
              </div>
            </div>
            <div class="view-slot-future-list">
              <div class="view-slot-future-item">
                <span>表示形式</span>
                <small>今後追加予定</small>
              </div>
              <div class="view-slot-future-item">
                <span>フォントサイズ</span>
                <small>今後追加予定</small>
              </div>
              <div class="view-slot-future-item">
                <span>文字色</span>
                <small>今後追加予定</small>
              </div>
              <div class="view-slot-future-item">
                <span>太字</span>
                <small>今後追加予定</small>
              </div>
              <div class="view-slot-future-item">
                <span>揃え</span>
                <small>今後追加予定</small>
              </div>
            </div>
          </template>
          <p v-else class="view-empty-hint mb-0">
            スロットを選択すると詳細設定を表示します。
          </p>
        </section>
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
          placeholder="未設定"
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
              placeholder="未設定"
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
              placeholder="未設定"
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
              placeholder="未設定"
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
              placeholder="未設定"
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
          placeholder="未設定"
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
          個別調整を全解除
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
