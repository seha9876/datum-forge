<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { VueDraggable } from "vue-draggable-plus";

import { useConfirmDialog } from "../../../composables/useConfirmDialog";

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

type TemplateSlotContextMenuTarget = {
  cardId: number;
  slotId: number;
} | null;

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

const confirmDialog = useConfirmDialog();
const draggableSlotItems = ref<TemplateSlotListItem[]>([]);
const slotChipRailRef = ref<HTMLElement | null>(null);
const contextMenuOpen = ref(false);
const contextMenuTarget = ref<[number, number]>([0, 0]);
const contextMenuSlotKey = ref<TemplateSlotContextMenuTarget>(null);

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

function chipTitle(slot: TemplateSlotListItem) {
  return `${slot.autoName} / ${slot.statusLabel}`;
}

function chipShortLabel(index: number) {
  return String(index + 1);
}

function handleSlotChipWheel(event: WheelEvent) {
  const rail = slotChipRailRef.value;
  if (!rail) {
    return;
  }

  const maxScrollLeft = rail.scrollWidth - rail.clientWidth;
  if (maxScrollLeft <= 0) {
    return;
  }

  const delta =
    Math.abs(event.deltaX) > 0
      ? event.deltaX
      : Math.abs(event.deltaY) > 0
        ? event.deltaY
        : 0;
  if (delta === 0) {
    return;
  }

  const nextScrollLeft = Math.min(
    maxScrollLeft,
    Math.max(0, rail.scrollLeft + delta)
  );
  if (nextScrollLeft === rail.scrollLeft) {
    return;
  }

  event.preventDefault();
  rail.scrollLeft = nextScrollLeft;
}

function openSlotContextMenu(event: MouseEvent, slot: TemplateSlotListItem) {
  event.preventDefault();
  contextMenuSlotKey.value = {
    cardId: slot.cardId,
    slotId: slot.slotId
  };
  contextMenuTarget.value = [event.clientX, event.clientY];
  contextMenuOpen.value = true;
}

function closeSlotContextMenu() {
  contextMenuOpen.value = false;
}

async function confirmRemoveSlotFromMenu() {
  const target = contextMenuSlotKey.value;
  if (!target) {
    return;
  }

  const slotIndex = draggableSlotItems.value.findIndex(
    (slot) => slot.cardId === target.cardId && slot.slotId === target.slotId
  );
  if (slotIndex < 0) {
    closeSlotContextMenu();
    return;
  }

  const slot = draggableSlotItems.value[slotIndex];
  const confirmed = await confirmDialog.open({
    title: "表示スロットを削除",
    message: `${slot.autoName} を削除します。この操作は元に戻せません。`,
    confirmText: "削除する",
    color: "warning"
  });
  if (!confirmed) {
    return;
  }

  props.removeTemplateSlot(target.cardId, slotIndex);
  closeSlotContextMenu();
}
</script>

<template>
  <aside class="view-style-inspector" @pointerdown.stop @click.stop>
    <div v-if="isTemplateMode" class="view-template-preview-bindings">
      <div class="view-slot-card-summary">
        <strong>{{ templateSlotTargetCardLabel || "カード未選択" }}</strong>
        <span>スロット {{ templateSlotCount }}件</span>
      </div>

      <div class="view-template-preview-binding-heading">
        <strong>表示スロット</strong>
      </div>

      <div class="view-template-preview-binding-body">
        <div class="view-slot-chip-strip">
          <div class="view-slot-chip-strip-fade start" />
          <div
            ref="slotChipRailRef"
            class="view-slot-chip-rail"
            @wheel="handleSlotChipWheel"
          >
            <VueDraggable
              v-model="draggableSlotItems"
              class="view-slot-chip-drag-list"
              item-key="slotId"
              tag="div"
              ghost-class="view-slot-chip-ghost"
              chosen-class="view-slot-chip-chosen"
              drag-class="view-slot-chip-drag"
              :animation="0"
              :force-fallback="true"
              :fallback-on-body="true"
              @end="handleTemplateSlotDragEnd"
            >
              <button
                v-for="(slot, slotIndex) in draggableSlotItems"
                :key="slot.slotId"
                type="button"
                class="view-slot-chip"
                :class="{
                  selected:
                    selectedTemplateSlotKey === `${slot.cardId}:${slot.slotId}`,
                  bound: slot.isBound,
                  unbound: !slot.isBound
                }"
                :title="chipTitle(slot)"
                @click="selectTemplateSlot(slot.cardId, slot.slotId)"
                @contextmenu="openSlotContextMenu($event, slot)"
              >
                <span class="view-slot-chip-index">
                  {{ chipShortLabel(slotIndex) }}
                </span>
              </button>
            </VueDraggable>
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
          <div class="view-slot-chip-strip-fade end" />
        </div>
        <p v-if="templateSlotItems.length === 0" class="view-empty-hint mb-0">
          まだスロットはありません。カードを選択して追加してください。
        </p>

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

    <v-menu
      v-model="contextMenuOpen"
      :target="contextMenuTarget"
      location="bottom start"
    >
      <v-list density="compact" min-width="160">
        <v-list-item
          prepend-icon="mdi-delete-outline"
          title="スロットを削除"
          @click="confirmRemoveSlotFromMenu"
        />
      </v-list>
    </v-menu>
  </aside>
</template>
