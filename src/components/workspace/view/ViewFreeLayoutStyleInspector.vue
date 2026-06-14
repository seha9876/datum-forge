<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { VueDraggable } from "vue-draggable-plus";

import { useConfirmDialog } from "../../../composables/useConfirmDialog";

import type {
  LayoutStyleKey,
  LayoutStyleValue
} from "./ViewFreeLayoutCanvas.helpers";
import type {
  ViewLayoutAutoHeightBehavior,
  ViewLayoutCardItem,
  ViewLayoutSlotDisplayFormat
} from "../../../types";

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

type SlotFontWeightValue = "inherit" | "normal" | "bold";
type SlotTextAlignValue = "inherit" | "left" | "center" | "right";
type SlotInheritedPreview = {
  fontSizeLabel: string;
  fontWeightLabel: string;
  textAlignLabel: string;
  textColorLabel: string;
  textColorValue: string;
};

const props = defineProps<{
  addTemplateSlot: (cardId: number) => void;
  applyAutoHeightEnabledFromCheckbox: (value: boolean | null) => void;
  applyBackgroundColorMode: (mode: "color" | "transparent") => void;
  applyFontWeightFromCheckbox: (value: boolean | null) => void;
  applyMaxAutoHeightBehavior: (
    value: ViewLayoutAutoHeightBehavior | null
  ) => void;
  applyMaxAutoHeightFromInput: (event: unknown) => void;
  applyNumberStyleFromInput: (key: LayoutStyleKey, event: unknown) => void;
  applyPresetId: (presetId: string | null) => void;
  applyPushDownSiblingsFromCheckbox: (value: boolean | null) => void;
  applySelectedStyle: (key: LayoutStyleKey, value: LayoutStyleValue) => void;
  applySelectedTemplateSlotDisplayFormat: (
    value: ViewLayoutSlotDisplayFormat | null
  ) => void;
  applySelectedTemplateSlotFontSizeFromInput: (event: unknown) => void;
  applySelectedTemplateSlotFontWeight: (value: SlotFontWeightValue) => void;
  applySelectedTemplateSlotTextAlign: (value: SlotTextAlignValue) => void;
  applySelectedTemplateSlotTextColorFromInput: (event: unknown) => void;
  applyShowLabelFromCheckbox: (value: boolean | null) => void;
  applyStyleFromInput: (key: LayoutStyleKey, event: unknown) => void;
  autoHeightEnabledValue: boolean;
  backgroundColorDisabled: boolean;
  backgroundColorDisabledReason: string;
  backgroundColorInputValue: () => string;
  cardPresetItems: Array<{ title: string; value: string | null }>;
  hasRecordOverrides: boolean;
  isTemplateMode: boolean;
  isTransparentBackgroundSelected: () => boolean;
  maxAutoHeightBehaviorValue: ViewLayoutAutoHeightBehavior | null;
  maxAutoHeightInputValue: number | "";
  pushDownSiblingsValue: boolean;
  removeTemplateSlot: (cardId: number, index: number) => void;
  reorderTemplateSlots: (cardId: number, orderedSlotIds: number[]) => void;
  resetRecordOverrides: () => void;
  resetSelectedCardOverride: () => void;
  resetSelectedStyle: () => void;
  selectedCardHasOverride: boolean;
  selectedLayouts: ViewLayoutCardItem[];
  selectedPresetId: string | null | "";
  selectedSlotItem: TemplateSlotListItem | null;
  selectedTemplateSlotDisplayFormatValue: ViewLayoutSlotDisplayFormat | null;
  selectedTemplateSlotFontSizeInherited: boolean;
  selectedTemplateSlotFontSizePlaceholder: string;
  selectedTemplateSlotFontSizeValue: number | "";
  selectedTemplateSlotInheritedPreview: SlotInheritedPreview | null;
  selectedTemplateSlotFontWeightValue: SlotFontWeightValue;
  selectedTemplateSlotFontWeightResolvedLabel: string;
  selectedTemplateSlotKey: string | null;
  selectedTemplateSlotTextAlignValue: SlotTextAlignValue;
  selectedTemplateSlotTextAlignResolvedLabel: string;
  selectedTemplateSlotTextColorInherited: boolean;
  selectedTemplateSlotTextColorValue: string;
  resetSelectedTemplateSlotTextColor: () => void;
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
const openStyleSections = ref<number[]>([0]);
const openSlotStyleSections = ref<number[]>([0]);
const slotDisplayFormatItems: Array<{
  title: string;
  value: ViewLayoutSlotDisplayFormat | null;
}> = [
  { title: "既定表示", value: null },
  { title: "標準", value: "plain" }
];

const slotDisplayFormatSelectItems = slotDisplayFormatItems.map((item) => ({
  ...item,
  title: item.value === "plain" ? "標準" : "既定表示"
}));

const heightBehaviorOptions: Array<{
  icon: string;
  label: string;
  tooltip: string;
  value: ViewLayoutAutoHeightBehavior;
}> = [
  {
    icon: "mdi-fit-to-page-outline",
    label: "縮小して収める",
    tooltip: "最大高さを超えたら文字を縮小して全体を表示",
    value: "scaleToFit"
  },
  {
    icon: "mdi-arrow-up-down",
    label: "内部スクロール",
    tooltip: "最大高さを超えたらカード内をスクロールして表示",
    value: "scroll"
  },
  {
    icon: "mdi-dots-horizontal",
    label: "末尾を省略",
    tooltip: "最大高さを超えたら末尾を省略して表示",
    value: "truncate"
  }
];

watch(
  () => props.templateSlotItems,
  (slotItems) => {
    draggableSlotItems.value = [...slotItems];
  },
  { immediate: true }
);

const canAddTemplateSlot = computed(() => props.selectedLayouts.length > 0);
const autoHeightSettingsDisabled = computed(
  () => !props.autoHeightEnabledValue
);
const hasTemplateSlots = computed(() => props.templateSlotItems.length > 0);
const templateSlotCount = computed(() => props.templateSlotItems.length);
const selectedSlotStatusMeta = computed(() =>
  props.selectedSlotItem?.isBound && props.selectedSlotItem.label
    ? `表示カラム: ${props.selectedSlotItem.label}`
    : "表示カラムはまだ紐付いていません"
);
const autoHeightModeSummary = computed(() =>
  props.autoHeightEnabledValue
    ? "内容量に合わせて高さを伸ばします"
    : "固定高さで表示します"
);
const autoHeightSettingHint = computed(() =>
  props.autoHeightEnabledValue
    ? "固定高さ向けの設定は無効です。"
    : "高さ自動拡張が OFF のときに使う設定です。"
);
const selectedHeightBehaviorLabel = computed(() => {
  const option = heightBehaviorOptions.find(
    (candidate) => candidate.value === props.maxAutoHeightBehaviorValue
  );
  return option?.label ?? "未選択";
});
const cardStyleHeading = computed(() =>
  props.isTemplateMode && hasTemplateSlots.value ? "カード設定" : "スタイル"
);
const cardStyleSectionHint = computed(() =>
  props.isTemplateMode && hasTemplateSlots.value
    ? "レイアウトと表示操作をここで設定します。"
    : ""
);
const commonFontSizeLabel = computed(() => {
  const value = props.styleNumberInputValue("fontSize");
  return typeof value === "number" ? `${value}px` : "未設定";
});
const commonTextColorValue = computed(() => {
  const value = props.styleInputValue("textColor");
  return value || props.themeColorInputValue("--v-theme-on-surface");
});
const commonTextColorLabel = computed(() => {
  const value = props.styleInputValue("textColor");
  return value || "既定色";
});
const commonFontWeightLabel = computed(() =>
  props.styleBooleanInputValue("fontWeight") ? "太字" : "標準"
);
const commonTextAlignLabel = computed(() => {
  const value = props.styleInspectorValues.textAlign;
  if (value === "center") {
    return "中央";
  }
  if (value === "right") {
    return "右";
  }
  return "左";
});
const commonStyleSummary = computed(
  () =>
    `${commonFontSizeLabel.value} / ${commonTextColorLabel.value} / ${commonFontWeightLabel.value} / ${commonTextAlignLabel.value}`
);
const slotOverrideSummary = computed(() => {
  const overrideLabels: string[] = [];
  if (props.selectedTemplateSlotDisplayFormatValue !== null) {
    overrideLabels.push("表示形式");
  }
  if (!props.selectedTemplateSlotFontSizeInherited) {
    overrideLabels.push("フォントサイズ");
  }
  if (!props.selectedTemplateSlotTextColorInherited) {
    overrideLabels.push("文字色");
  }
  if (props.selectedTemplateSlotFontWeightValue !== "inherit") {
    overrideLabels.push("太字");
  }
  if (props.selectedTemplateSlotTextAlignValue !== "inherit") {
    overrideLabels.push("揃え");
  }
  if (overrideLabels.length === 0) {
    return "個別上書きなし";
  }
  if (overrideLabels.length <= 2) {
    return `${overrideLabels.join(" / ")} を個別設定`;
  }
  return `${overrideLabels[0]} ほか ${overrideLabels.length - 1} 項目を個別設定`;
});

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
    <div class="view-style-inspector-heading">
      <strong>{{ cardStyleHeading }}</strong>
      <span>{{ selectedLayouts.length }}件</span>
    </div>

    <div class="view-style-inspector-scroll">
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
                      selectedTemplateSlotKey ===
                      `${slot.cardId}:${slot.slotId}`,
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

          <section v-if="hasTemplateSlots" class="view-slot-detail-panel">
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
              <div class="view-slot-style-panel">
                <v-expansion-panels
                  v-model="openSlotStyleSections"
                  multiple
                  variant="accordion"
                  flat
                  class="view-style-panels view-slot-style-panels inline-panel"
                >
                  <v-expansion-panel :value="0" rounded="0" elevation="0">
                    <v-expansion-panel-title class="view-style-panel-title">
                      <div class="expansion-title">
                        <strong>スロット個別表示</strong>
                        <small>{{ slotOverrideSummary }}</small>
                      </div>
                    </v-expansion-panel-title>
                    <v-expansion-panel-text class="view-style-panel-body">
                      <div class="view-style-panel-stack">
                        <div class="view-style-section-heading">
                          <span>
                            未設定の項目は「継承元:
                            カード共通スタイル」を使います。
                          </span>
                        </div>

                        <label class="view-style-control">
                          <span>表示形式</span>
                          <v-select
                            :items="slotDisplayFormatSelectItems"
                            :model-value="
                              selectedTemplateSlotDisplayFormatValue
                            "
                            item-title="title"
                            item-value="value"
                            variant="outlined"
                            density="compact"
                            hide-details
                            @update:model-value="
                              applySelectedTemplateSlotDisplayFormat
                            "
                          />
                          <small>未設定時は既定表示を使います。</small>
                        </label>

                        <div class="view-style-two-column">
                          <label class="view-style-control">
                            <span>フォントサイズ</span>
                            <input
                              :class="{
                                'view-style-input-inherited':
                                  selectedTemplateSlotFontSizeInherited
                              }"
                              type="number"
                              min="10"
                              max="48"
                              step="1"
                              :value="selectedTemplateSlotFontSizeValue"
                              :placeholder="
                                selectedTemplateSlotFontSizePlaceholder
                              "
                              @change="
                                applySelectedTemplateSlotFontSizeFromInput
                              "
                            />
                            <small v-if="selectedTemplateSlotFontSizeInherited">
                              共通設定を使っています。
                            </small>
                          </label>

                          <label
                            class="view-style-control view-background-control"
                          >
                            <span>文字色</span>
                            <div class="view-background-row">
                              <input
                                type="color"
                                :class="{
                                  muted: selectedTemplateSlotTextColorInherited
                                }"
                                :value="selectedTemplateSlotTextColorValue"
                                @input="
                                  applySelectedTemplateSlotTextColorFromInput
                                "
                              />
                              <button
                                type="button"
                                class="view-background-transparent-button"
                                :disabled="
                                  selectedTemplateSlotTextColorInherited
                                "
                                @click="resetSelectedTemplateSlotTextColor"
                              >
                                継承に戻す
                              </button>
                            </div>
                            <small
                              v-if="selectedTemplateSlotTextColorInherited"
                            >
                              継承中:
                              {{
                                selectedTemplateSlotInheritedPreview?.textColorLabel
                              }}
                            </small>
                          </label>
                        </div>

                        <div class="view-style-control">
                          <span>太字</span>
                          <v-btn-toggle
                            :model-value="selectedTemplateSlotFontWeightValue"
                            mandatory
                            divided
                            color="primary"
                            variant="tonal"
                            rounded="lg"
                            density="comfortable"
                            class="view-style-segment"
                            @update:model-value="
                              applySelectedTemplateSlotFontWeight
                            "
                          >
                            <v-btn value="inherit">継承</v-btn>
                            <v-btn value="normal">標準</v-btn>
                            <v-btn value="bold">太字</v-btn>
                          </v-btn-toggle>
                          <small>{{
                            selectedTemplateSlotFontWeightResolvedLabel
                          }}</small>
                        </div>

                        <div class="view-style-control">
                          <span>揃え</span>
                          <v-btn-toggle
                            :model-value="selectedTemplateSlotTextAlignValue"
                            mandatory
                            divided
                            color="primary"
                            variant="tonal"
                            rounded="lg"
                            density="comfortable"
                            class="view-style-segment"
                            @update:model-value="
                              applySelectedTemplateSlotTextAlign
                            "
                          >
                            <v-btn value="inherit">継承</v-btn>
                            <v-btn value="left">左</v-btn>
                            <v-btn value="center">中央</v-btn>
                            <v-btn value="right">右</v-btn>
                          </v-btn-toggle>
                          <small>{{
                            selectedTemplateSlotTextAlignResolvedLabel
                          }}</small>
                        </div>
                      </div>
                    </v-expansion-panel-text>
                  </v-expansion-panel>

                  <v-expansion-panel :value="1" rounded="0" elevation="0">
                    <v-expansion-panel-title class="view-style-panel-title">
                      <div class="expansion-title">
                        <strong>継承元: カード共通スタイル</strong>
                        <small>{{ commonStyleSummary }}</small>
                      </div>
                    </v-expansion-panel-title>
                    <v-expansion-panel-text class="view-style-panel-body">
                      <div class="view-style-panel-stack">
                        <div class="view-style-section-heading">
                          <span>
                            このカードの文字系既定値です。未設定スロットはここを使います。
                          </span>
                        </div>

                        <div class="view-style-two-column">
                          <label class="view-style-control">
                            <span>フォントサイズ</span>
                            <input
                              type="number"
                              min="10"
                              max="48"
                              step="1"
                              :value="styleNumberInputValue('fontSize')"
                              placeholder="未設定"
                              @change="
                                applyNumberStyleFromInput('fontSize', $event)
                              "
                            />
                          </label>

                          <label class="view-style-control">
                            <span>文字色</span>
                            <input
                              type="color"
                              :value="commonTextColorValue"
                              @input="applyStyleFromInput('textColor', $event)"
                            />
                          </label>
                        </div>

                        <div class="view-style-control">
                          <span>太字</span>
                          <v-btn-toggle
                            :model-value="styleBooleanInputValue('fontWeight')"
                            mandatory
                            divided
                            color="primary"
                            variant="tonal"
                            rounded="lg"
                            density="comfortable"
                            class="view-style-segment"
                            @update:model-value="applyFontWeightFromCheckbox"
                          >
                            <v-btn :value="false">標準</v-btn>
                            <v-btn :value="true">太字</v-btn>
                          </v-btn-toggle>
                          <small>現在: {{ commonFontWeightLabel }}</small>
                        </div>

                        <div class="view-style-control">
                          <span>揃え</span>
                          <v-btn-toggle
                            :model-value="styleInspectorValues.textAlign"
                            mandatory
                            divided
                            color="primary"
                            variant="tonal"
                            rounded="lg"
                            density="comfortable"
                            class="view-style-segment"
                            @update:model-value="
                              applySelectedStyle('textAlign', $event)
                            "
                          >
                            <v-btn value="left">左</v-btn>
                            <v-btn value="center">中央</v-btn>
                            <v-btn value="right">右</v-btn>
                          </v-btn-toggle>
                          <small>現在: {{ commonTextAlignLabel }}</small>
                        </div>
                      </div>
                    </v-expansion-panel-text>
                  </v-expansion-panel>
                </v-expansion-panels>
              </div>
            </template>
            <p v-else class="view-empty-hint mb-0">
              スロットを選択すると詳細設定を表示します。
            </p>
          </section>
        </div>
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

        <div v-if="cardStyleSectionHint" class="view-style-context-note">
          {{ cardStyleSectionHint }}
        </div>

        <v-expansion-panels
          v-model="openStyleSections"
          multiple
          variant="accordion"
          flat
          class="view-style-panels inline-panel"
        >
          <v-expansion-panel :value="0" rounded="0" elevation="0">
            <v-expansion-panel-title class="view-style-panel-title">
              <div class="expansion-title">
                <strong>レイアウト</strong>
                <small>{{ autoHeightModeSummary }}</small>
              </div>
            </v-expansion-panel-title>
            <v-expansion-panel-text class="view-style-panel-body">
              <div class="view-style-panel-stack">
                <v-switch
                  color="primary"
                  density="compact"
                  hide-details
                  :model-value="autoHeightEnabledValue"
                  label="高さ自動拡張"
                  @update:model-value="applyAutoHeightEnabledFromCheckbox"
                />

                <div class="view-style-section-heading">
                  <strong>固定高さで使う設定</strong>
                  <span>{{ autoHeightSettingHint }}</span>
                </div>

                <div
                  class="view-style-two-column view-style-two-column-layout view-style-two-column-behavior"
                >
                  <v-checkbox
                    class="view-style-check view-style-control view-style-cell-full"
                    color="primary"
                    density="compact"
                    hide-details
                    :disabled="autoHeightSettingsDisabled"
                    :model-value="pushDownSiblingsValue"
                    label="下方向へ押し出し"
                    @update:model-value="applyPushDownSiblingsFromCheckbox"
                  />

                  <label class="view-style-control view-style-behavior-input">
                    <span>最大高さ</span>
                    <input
                      type="number"
                      min="64"
                      step="1"
                      :disabled="autoHeightSettingsDisabled"
                      :value="maxAutoHeightInputValue"
                      placeholder="上限なし"
                      @change="applyMaxAutoHeightFromInput"
                    />
                  </label>

                  <div class="view-style-control view-style-behavior-cell">
                    <span>最大高さ超過時</span>
                    <v-btn-toggle
                      :disabled="autoHeightSettingsDisabled"
                      :model-value="maxAutoHeightBehaviorValue"
                      divided
                      mandatory
                      color="primary"
                      variant="tonal"
                      rounded="lg"
                      density="comfortable"
                      class="view-style-behavior-toggle"
                      @update:model-value="applyMaxAutoHeightBehavior"
                    >
                      <v-tooltip
                        v-for="option in heightBehaviorOptions"
                        :key="option.value"
                        :text="option.tooltip"
                        location="bottom"
                      >
                        <template #activator="{ props: tooltipProps }">
                          <v-btn
                            v-bind="tooltipProps"
                            :value="option.value"
                            class="view-style-behavior-button"
                          >
                            <v-icon :icon="option.icon" size="18" />
                          </v-btn>
                        </template>
                      </v-tooltip>
                    </v-btn-toggle>
                    <small>選択中: {{ selectedHeightBehaviorLabel }}</small>
                  </div>
                </div>

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
                        @change="
                          applyNumberStyleFromInput('paddingTop', $event)
                        "
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
                        @change="
                          applyNumberStyleFromInput('paddingRight', $event)
                        "
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
                        @change="
                          applyNumberStyleFromInput('paddingBottom', $event)
                        "
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
                        @change="
                          applyNumberStyleFromInput('paddingLeft', $event)
                        "
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
              </div>
            </v-expansion-panel-text>
          </v-expansion-panel>

          <v-expansion-panel
            v-if="!isTemplateMode"
            :value="1"
            rounded="0"
            elevation="0"
          >
            <v-expansion-panel-title class="view-style-panel-title">
              <div class="expansion-title">
                <strong>文字</strong>
                <small>色、サイズ、方向、揃え</small>
              </div>
            </v-expansion-panel-title>
            <v-expansion-panel-text class="view-style-panel-body">
              <div class="view-style-panel-stack">
                <div class="view-style-two-column">
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
                    class="view-style-segment"
                    @update:model-value="
                      applySelectedStyle('textDirection', $event)
                    "
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
                    class="view-style-segment"
                    @update:model-value="
                      applySelectedStyle('textAlign', $event)
                    "
                  >
                    <v-btn value="left">左</v-btn>
                    <v-btn value="center">中央</v-btn>
                    <v-btn value="right">右</v-btn>
                  </v-btn-toggle>
                </div>
              </div>
            </v-expansion-panel-text>
          </v-expansion-panel>

          <v-expansion-panel :value="2" rounded="0" elevation="0">
            <v-expansion-panel-title class="view-style-panel-title">
              <div class="expansion-title">
                <strong>表示 / 操作</strong>
                <small>背景、ラベル、リセット</small>
              </div>
            </v-expansion-panel-title>
            <v-expansion-panel-text class="view-style-panel-body">
              <div class="view-style-panel-stack">
                <div class="view-style-two-column">
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
                    <small
                      v-if="backgroundColorDisabled"
                      class="view-empty-hint"
                    >
                      {{ backgroundColorDisabledReason }}
                    </small>
                  </label>

                  <v-checkbox
                    class="view-style-check view-style-control"
                    color="primary"
                    density="compact"
                    hide-details
                    :model-value="styleBooleanInputValue('showLabel')"
                    label="カラム名を表示"
                    @update:model-value="applyShowLabelFromCheckbox"
                  />
                </div>

                <div v-if="!isTemplateMode" class="view-style-action-group">
                  <button
                    type="button"
                    class="view-style-reset-button"
                    :disabled="!selectedCardHasOverride"
                    @click="resetSelectedCardOverride"
                  >
                    このカードだけ元に戻す
                  </button>
                  <button
                    type="button"
                    class="view-style-reset-button"
                    :disabled="!hasRecordOverrides"
                    @click="resetRecordOverrides"
                  >
                    個別差分をすべて解除
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
            </v-expansion-panel-text>
          </v-expansion-panel>
        </v-expansion-panels>
      </div>
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
