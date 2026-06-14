<script setup lang="ts">
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  watch
} from "vue";

import { useConfirmDialog } from "../../../composables/useConfirmDialog";

import { isKnownCardPresetId, listCardPresets } from "./cardPresets/registry";
import { useFreeLayoutStyleEditing } from "./useFreeLayoutStyleEditing";
import ViewFreeLayoutBindingPanel from "./ViewFreeLayoutBindingPanel.vue";
import {
  boxFromPoints,
  clampViewportScale,
  DEFAULT_CARD_STYLE,
  DRAG_THRESHOLD,
  intersects,
  MIN_CARD_HEIGHT,
  MIN_CARD_WIDTH,
  RESIZE_HANDLES,
  VIEWPORT_ZOOM_STEP,
  WORLD_HEIGHT,
  WORLD_WIDTH
} from "./ViewFreeLayoutCanvas.helpers";
import ViewFreeLayoutStyleInspector from "./ViewFreeLayoutStyleInspector.vue";
import ViewFreeLayoutToolbar from "./ViewFreeLayoutToolbar.vue";

import type {
  CanvasElement,
  InteractionState,
  KeyboardLikeEvent,
  LayoutStyleKey,
  PanDrag,
  PointerLikeEvent,
  PointerTarget,
  ResizeDirection,
  SelectionBox,
  SelectionDrag,
  WheelLikeEvent
} from "./ViewFreeLayoutCanvas.helpers";
import type {
  AppColumn,
  TableDetail,
  TemplatePreviewRecordSelection,
  ViewLayoutAutoHeightBehavior,
  ViewLayoutCardItem,
  ViewLayoutCardColumnBinding,
  ViewLayoutSlotDisplayFormat,
  ViewLayoutTemplate,
  ViewLayoutTemplateCard,
  ViewLayoutTemplateCardSlot,
  ViewSelection,
  ViewTableSection
} from "../../../types";
import type { CSSProperties } from "vue";

type TemplateSlotListItem = {
  autoName: string;
  cardId: number;
  isBound: boolean;
  label: string;
  slotId: number;
  statusLabel: string;
};

type SelectedTemplateSlotKey = {
  cardId: number;
  slotId: number;
};

type RenderedViewLayoutCardItem = ViewLayoutCardItem & {
  renderBaseHeight: number;
  renderBodyMode: "normal" | "scaleToFit" | "scroll" | "truncate";
  renderContentScale: number;
  renderContentViewportHeight: number | null;
  renderNaturalHeight: number | null;
};

type SlotFontWeightValue = "inherit" | "normal" | "bold";
type SlotTextAlignValue = "inherit" | "left" | "center" | "right";
type FieldEntry = {
  key: string;
  label: string;
  slot: ViewLayoutTemplateCardSlot | null;
  value: string;
};
const props = withDefaults(
  defineProps<{
    detail: TableDetail | null;
    activeTemplateId?: number | null;
    activeTemplateName?: string | null;
    editorMode?: "record" | "template";
    folderActiveTemplateId?: number | null;
    folderLayoutTemplates?: ViewLayoutTemplate[];
    layoutItems: ViewLayoutCardItem[];
    saving: boolean;
    selectedItem: ViewSelection | null;
    tableSections?: ViewTableSection[];
    templateCards?: ViewLayoutTemplateCard[];
    templateName?: string;
    templatePreviewBindings?: ViewLayoutCardColumnBinding[];
    templatePreviewDetail?: TableDetail | null;
    templatePreviewLoading?: boolean;
    templatePreviewSelectedItem?: ViewSelection | null;
  }>(),
  {
    activeTemplateId: null,
    activeTemplateName: null,
    editorMode: "record",
    folderActiveTemplateId: null,
    folderLayoutTemplates: () => [],
    tableSections: () => [],
    templateCards: () => [],
    templateName: "",
    templatePreviewBindings: () => [],
    templatePreviewDetail: null,
    templatePreviewLoading: false,
    templatePreviewSelectedItem: null
  }
);

const emit = defineEmits<{
  "assign-record-template": [number, number];
  "clear-template-preview": [];
  "clear-record-template": [number];
  "reset-card-override": [number];
  "reset-record-overrides": [];
  "save-card-column-bindings": [ViewLayoutCardColumnBinding[]];
  "save-record-overrides": [ViewLayoutCardItem[]];
  "save-template-cards": [ViewLayoutTemplateCard[]];
  "select-template-preview-record": [TemplatePreviewRecordSelection];
}>();
const panelRef = ref<CanvasElement | null>(null);
const canvasRef = ref<CanvasElement | null>(null);
const editMode = ref(false);
const draftLayouts = ref<ViewLayoutCardItem[]>([]);
const interaction = ref<InteractionState | null>(null);
const selectedCardIds = ref<number[]>([]);
const selectionBox = ref<SelectionBox | null>(null);
const selectionDrag = ref<SelectionDrag | null>(null);
const viewportScale = ref(1);
const viewportOffsetX = ref(0);
const viewportOffsetY = ref(0);
const panDrag = ref<PanDrag | null>(null);
const isSpacePressed = ref(false);
const isBindingEditorOpen = ref(false);
const bindingDraft = ref<Record<number, Array<number | null>>>({});
const selectedBindingCardId = ref<number | null>(null);
const selectedTemplateSlotKey = ref<SelectedTemplateSlotKey | null>(null);
const cardContentMeasureElements = new Map<number, HTMLElement>();
const measuredCardContentHeights = ref<Record<number, number>>({});
const templatePreviewMenuOpen = ref(false);
const viewportPercent = computed(() => Math.round(viewportScale.value * 100));
const isTemplateMode = computed(() => props.editorMode === "template");
const isTemplatePreviewActive = computed(
  () =>
    isTemplateMode.value &&
    props.templatePreviewSelectedItem?.type === "tableRecord" &&
    props.templatePreviewDetail !== null
);

const hasRecordOverrides = computed(
  () =>
    !isTemplateMode.value &&
    draftLayouts.value.some((layout) => layout.hasOverride)
);
const worldLayerStyle = computed<CSSProperties>(() => ({
  height: `${WORLD_HEIGHT}px`,
  transform: `translate(${viewportOffsetX.value}px, ${viewportOffsetY.value}px) scale(${viewportScale.value})`,
  width: `${WORLD_WIDTH}px`
}));
const activeDetail = computed(() =>
  isTemplatePreviewActive.value ? props.templatePreviewDetail : props.detail
);
const activeSelectedItem = computed(() =>
  isTemplatePreviewActive.value
    ? props.templatePreviewSelectedItem
    : props.selectedItem
);
const currentRecord = computed(() => {
  const selected = activeSelectedItem.value;
  const detail = activeDetail.value;
  if (selected?.type !== "tableRecord" || !detail) {
    return null;
  }

  return (
    detail.records.find((record) => record.id === selected.recordId) ?? null
  );
});
const displayColumns = computed(() =>
  (activeDetail.value?.columns ?? []).filter(
    (column) => column.columnName !== "id"
  )
);
function isBoundToCurrentTable(item: ViewLayoutCardItem) {
  if (item.slots.length > 0) {
    return true;
  }
  return item.columns.some((binding) =>
    displayColumns.value.some((column) => column.id === binding.columnId)
  );
}

const visibleLayouts = computed(() =>
  draftLayouts.value.filter((item) => {
    if (isTemplateMode.value) {
      return item.visible || editMode.value;
    }

    return isBoundToCurrentTable(item) && (item.visible || editMode.value);
  })
);
const shouldMeasureAutoHeight = computed(() => {
  if (isBindingEditorVisible.value) {
    return false;
  }

  if (isTemplateMode.value) {
    return isTemplatePreviewActive.value && currentRecord.value !== null;
  }

  return currentRecord.value !== null;
});
const selectedLayouts = computed(() =>
  visibleLayouts.value.filter((layout) => isSelected(layout.cardId))
);
const templateSlotTargetLayout = computed(() =>
  isTemplateMode.value ? (selectedLayouts.value[0] ?? null) : null
);
const cardPresetItems = computed(() => [
  { title: "標準", value: null },
  ...listCardPresets().map((preset) => ({
    title: preset.label,
    value: preset.id
  }))
]);
const selectedPresetId = computed(() => {
  const [firstLayout, ...restLayouts] = selectedLayouts.value;
  if (!firstLayout) {
    return "";
  }

  const firstPresetId = firstLayout.presetId ?? null;
  const hasMixedValues = restLayouts.some(
    (layout) => (layout.presetId ?? null) !== firstPresetId
  );
  return hasMixedValues ? "" : firstPresetId;
});
const backgroundColorEditingDisabled = computed(
  () =>
    isTemplateMode.value &&
    selectedLayouts.value.some((layout) => !canOverrideBackgroundColor(layout))
);
const backgroundColorDisabledReason = computed(() =>
  backgroundColorEditingDisabled.value
    ? "このプリセットでは背景色を変更できません。"
    : ""
);
const bindableTemplateLayouts = computed(() =>
  isTemplateMode.value ? [] : draftLayouts.value.filter((item) => item.visible)
);
const hasUnboundTemplateForTable = computed(
  () =>
    !isTemplateMode.value &&
    bindableTemplateLayouts.value.length > 0 &&
    displayColumns.value.length > 0 &&
    bindableTemplateLayouts.value.every((layout) =>
      resolvedColumnIds(layout).every((columnId) => !columnId)
    )
);
const isBindingEditorVisible = computed(
  () => hasUnboundTemplateForTable.value || isBindingEditorOpen.value
);
const shouldRenderCardContent = computed(
  () => editMode.value || isTemplatePreviewActive.value
);
const renderedVisibleLayouts = computed<RenderedViewLayoutCardItem[]>(() =>
  buildRenderedLayouts(visibleLayouts.value)
);
const canvasLayouts = computed(() =>
  isBindingEditorVisible.value
    ? bindableTemplateLayouts.value
    : renderedVisibleLayouts.value
);
const bindingColumnItems = computed(() =>
  displayColumns.value.map((column) => ({
    title: column.displayName,
    value: column.id
  }))
);
const recordTemplateItems = computed(() =>
  props.folderLayoutTemplates
    .filter((template) => template.id !== props.folderActiveTemplateId)
    .map((template) => ({
      title: template.name,
      value: template.id
    }))
);
const recordTemplateSource = computed<"record" | "folder" | "unset">(() => {
  const selected = props.selectedItem;
  if (selected?.type !== "tableRecord") {
    return "unset";
  }
  if (selected.recordTemplateId) {
    return "record";
  }
  return props.folderActiveTemplateId && props.activeTemplateId
    ? "folder"
    : "unset";
});
const recordTemplateSourceLabel = computed(() => {
  if (recordTemplateSource.value === "record") {
    return "このデータ専用";
  }
  if (recordTemplateSource.value === "folder") {
    return "フォルダから継承中";
  }
  return "未設定";
});
const recordTemplateSourceChipLabel = computed(() => {
  if (recordTemplateSource.value === "folder" && props.activeTemplateName) {
    return `${recordTemplateSourceLabel.value}（${props.activeTemplateName}）`;
  }
  return recordTemplateSourceLabel.value;
});
const recordTemplateTooltipText = computed(() =>
  recordTemplateSource.value === "folder" && props.activeTemplateName
    ? props.activeTemplateName
    : ""
);
const recordTemplateSourceColor = computed(() => {
  if (recordTemplateSource.value === "record") {
    return "primary";
  }
  if (recordTemplateSource.value === "folder") {
    return "secondary";
  }
  return "warning";
});
const templatePreviewRecordItems = computed(() =>
  props.tableSections.flatMap((section) =>
    section.records.map((record) => ({
      title: `${section.displayName} / ${record.label}`,
      value: `${section.tableId}:${record.id}`,
      record: {
        tableId: section.tableId,
        tableName: section.tableName,
        tableDisplayName: section.displayName,
        recordId: record.id,
        recordLabel: record.label
      }
    }))
  )
);
const selectedTemplatePreviewRecordKey = computed(() => {
  const selected = props.templatePreviewSelectedItem;
  return selected?.type === "tableRecord"
    ? `${selected.tableId}:${selected.recordId}`
    : null;
});
const selectedRecordPanelKey = computed(() => {
  const selected = props.selectedItem;
  if (selected?.type !== "tableRecord") {
    return null;
  }
  if (selected.folderRecordId) {
    return `folder-record:${selected.folderRecordId}`;
  }
  return `record:${selected.tableId}:${selected.recordId}:${selected.folderId ?? ""}`;
});
const templatePreviewLabel = computed(() => {
  const selected = props.templatePreviewSelectedItem;
  if (selected?.type !== "tableRecord") {
    return "";
  }
  return `${selected.tableDisplayName} / ${selected.recordLabel}`;
});
const templateSlotItems = computed<TemplateSlotListItem[]>(() => {
  const layout = templateSlotTargetLayout.value;
  if (!layout) {
    return [];
  }

  const columnIds = resolvedColumnIds(layout);
  return sortedSlots(layout).map((slot, index) => {
    const columnId = columnIds[index] ?? null;
    const autoName = `スロット${index + 1}`;
    const boundLabel = columnId === null ? null : columnDisplayName(columnId);

    return {
      autoName,
      cardId: layout.cardId,
      isBound: columnId !== null,
      label: boundLabel ?? autoName,
      slotId: slot.slotId,
      statusLabel: columnId === null ? "未紐付け" : "紐付け済み"
    };
  });
});
const selectedTemplateSlotItem = computed(() => {
  const key = selectedTemplateSlotKey.value;
  if (!key) {
    return null;
  }

  return (
    templateSlotItems.value.find(
      (slot) => slot.cardId === key.cardId && slot.slotId === key.slotId
    ) ?? null
  );
});
const selectedTemplateSlot = computed<ViewLayoutTemplateCardSlot | null>(() => {
  const key = selectedTemplateSlotKey.value;
  const layout = templateSlotTargetLayout.value;
  if (!key || !layout || layout.cardId !== key.cardId) {
    return null;
  }

  return sortedSlots(layout).find((slot) => slot.slotId === key.slotId) ?? null;
});
const templateSlotTargetCardLabel = computed(() => {
  const layout = templateSlotTargetLayout.value;
  return layout ? cardBindingLabel(layout, 0) : "";
});
const selectedTemplateSlotDisplayFormat =
  computed<ViewLayoutSlotDisplayFormat | null>(() =>
    slotDisplayFormatValue(selectedTemplateSlot.value)
  );
const selectedTemplateSlotFontSizeInherited = computed(
  () => typeof selectedTemplateSlot.value?.fontSize !== "number"
);
const selectedTemplateSlotFontSize = computed<number | "">(() =>
  typeof selectedTemplateSlot.value?.fontSize === "number"
    ? selectedTemplateSlot.value.fontSize
    : ""
);
const selectedTemplateSlotFontSizePlaceholder = computed(() => {
  const layout = templateSlotTargetLayout.value;
  if (!layout) {
    return "継承";
  }
  return `継承 (${layoutStyleValue(layout, "fontSize")}px)`;
});
const selectedTemplateSlotTextColor = computed(() => {
  const layout = templateSlotTargetLayout.value;
  if (!layout) {
    return themeColorInputValue("--v-theme-on-surface");
  }

  const value = resolvedSlotTextColor(layout, selectedTemplateSlot.value);
  return typeof value === "string" && value
    ? value
    : themeColorInputValue("--v-theme-on-surface");
});
const selectedTemplateSlotTextColorInherited = computed(
  () => !selectedTemplateSlot.value?.textColor
);
const selectedTemplateSlotFontWeight = computed<SlotFontWeightValue>(() => {
  const value = slotFontWeightValue(selectedTemplateSlot.value);
  return value ?? "inherit";
});
const selectedTemplateSlotTextAlign = computed<SlotTextAlignValue>(() => {
  const value = slotTextAlignValue(selectedTemplateSlot.value);
  return value ?? "inherit";
});
const selectedTemplateSlotInheritedPreview = computed(() => {
  const layout = templateSlotTargetLayout.value;
  if (!layout) {
    return null;
  }

  const inheritedTextColor = layoutStyleValue(layout, "textColor");
  const resolvedTextColor =
    typeof inheritedTextColor === "string" && inheritedTextColor
      ? inheritedTextColor
      : themeColorInputValue("--v-theme-on-surface");
  const inheritedFontWeight = layoutStyleValue(layout, "fontWeight");
  const inheritedTextAlign = layoutStyleValue(layout, "textAlign");

  return {
    fontSizeLabel: `${layoutStyleValue(layout, "fontSize")}px`,
    fontWeightLabel: inheritedFontWeight === "bold" ? "太字" : "標準",
    textAlignLabel:
      inheritedTextAlign === "center"
        ? "中央"
        : inheritedTextAlign === "right"
          ? "右"
          : "左",
    textColorLabel:
      typeof inheritedTextColor === "string" && inheritedTextColor
        ? inheritedTextColor
        : "既定色",
    textColorValue: resolvedTextColor
  };
});
const selectedTemplateSlotFontWeightResolvedLabel = computed(() => {
  if (selectedTemplateSlotFontWeight.value === "inherit") {
    return `継承中: ${
      selectedTemplateSlotInheritedPreview.value?.fontWeightLabel ?? "標準"
    }`;
  }
  return selectedTemplateSlotFontWeight.value === "bold"
    ? "個別設定: 太字"
    : "個別設定: 標準";
});
const selectedTemplateSlotTextAlignResolvedLabel = computed(() => {
  if (selectedTemplateSlotTextAlign.value === "inherit") {
    return `継承中: ${
      selectedTemplateSlotInheritedPreview.value?.textAlignLabel ?? "左"
    }`;
  }
  return selectedTemplateSlotTextAlign.value === "center"
    ? "個別設定: 中央"
    : selectedTemplateSlotTextAlign.value === "right"
      ? "個別設定: 右"
      : "個別設定: 左";
});
const bindingDraftValues = computed(() =>
  Object.values(bindingDraft.value)
    .flat()
    .filter((columnId): columnId is number => columnId !== null)
);
const canSaveBindingDraft = computed(
  () => bindingDraftValues.value.length > 0 && !props.saving
);

const selectedCardHasOverride = computed(
  () =>
    selectedLayouts.value.length === 1 && selectedLayouts.value[0].hasOverride
);

watch(
  () => [props.layoutItems, props.templateCards, props.editorMode] as const,
  () => {
    draftLayouts.value = isTemplateMode.value
      ? props.templateCards.map(templateCardToLayout)
      : props.layoutItems.map((item) => ({ ...item }));
    resetBindingDraft();
    pruneSelection();
  },
  { immediate: true }
);
watch(
  () => selectedRecordPanelKey.value,
  () => {
    clearSelection();
    closeBindingEditor();
    selectedTemplateSlotKey.value = null;
    resetViewport();
  }
);
watch(
  isTemplateMode,
  (enabled) => {
    if (enabled) {
      editMode.value = true;
    }
  },
  { immediate: true }
);
watch(editMode, (enabled) => {
  if (!enabled) {
    clearSelection();
    interaction.value = null;
    selectionBox.value = null;
    selectionDrag.value = null;
    panDrag.value = null;
    isSpacePressed.value = false;
    selectedTemplateSlotKey.value = null;
    resetViewport();
  }
});
watch(
  [templateSlotTargetLayout, templateSlotItems],
  ([layout, slotItems]) => {
    if (!layout || slotItems.length === 0) {
      selectedTemplateSlotKey.value = null;
      return;
    }

    const selected = selectedTemplateSlotKey.value;
    if (
      selected &&
      selected.cardId === layout.cardId &&
      slotItems.some((slot) => slot.slotId === selected.slotId)
    ) {
      return;
    }

    const [firstSlot] = slotItems;
    selectedTemplateSlotKey.value = firstSlot
      ? { cardId: firstSlot.cardId, slotId: firstSlot.slotId }
      : null;
  },
  { immediate: true }
);
watch(isBindingEditorVisible, (visible) => {
  if (visible) {
    resetBindingDraft();
    editMode.value = false;
    selectedBindingCardId.value =
      selectedBindingCardId.value &&
      bindableTemplateLayouts.value.some(
        (layout) => layout.cardId === selectedBindingCardId.value
      )
        ? selectedBindingCardId.value
        : (bindableTemplateLayouts.value[0]?.cardId ?? null);
    // 注意: 拡大率の自動変更はユーザーの意図しない動作となるため行わない
    return;
  }

  selectedBindingCardId.value = null;
});
watch(
  [canvasLayouts, currentRecord, shouldMeasureAutoHeight, () => editMode.value],
  async () => {
    await nextTick();
    measureCardContentHeights();
  },
  { deep: true, immediate: true }
);
onMounted(() => {
  globalThis.addEventListener?.("keydown", handleKeyDown);
  globalThis.addEventListener?.("keyup", handleKeyUp);
});
onBeforeUnmount(() => {
  globalThis.removeEventListener?.("keydown", handleKeyDown);
  globalThis.removeEventListener?.("keyup", handleKeyUp);
});

function columnById(columnId: number) {
  return displayColumns.value.find((column) => column.id === columnId) ?? null;
}

function normalizedBindingIds(columnIds: Array<number | null>) {
  return columnIds.length > 0 ? columnIds : [null];
}

function sortedSlots(layout: ViewLayoutCardItem | ViewLayoutTemplateCard) {
  return [...layout.slots].sort(
    (left, right) => left.sortOrder - right.sortOrder
  );
}

function slotDisplayFormatValue(
  slot: ViewLayoutTemplateCardSlot | null | undefined
): ViewLayoutSlotDisplayFormat | null {
  return slot?.displayFormat ?? null;
}

function slotFontWeightValue(
  slot: ViewLayoutTemplateCardSlot | null | undefined
): "normal" | "bold" | null {
  return slot?.fontWeight ?? null;
}

function slotTextAlignValue(
  slot: ViewLayoutTemplateCardSlot | null | undefined
): "left" | "center" | "right" | null {
  return slot?.textAlign ?? null;
}

function layoutColumnIds(layout: ViewLayoutCardItem) {
  return normalizedBindingIds(
    [...layout.columns]
      .sort((left, right) => left.sortOrder - right.sortOrder)
      .map((binding) => binding.columnId)
  );
}

function templatePreviewColumnIds(layout: ViewLayoutCardItem) {
  return normalizedBindingIds(
    props.templatePreviewBindings
      .filter((binding) => binding.cardId === layout.cardId)
      .sort((left, right) => left.sortOrder - right.sortOrder)
      .map((binding) => binding.columnId)
  );
}

function resolvedColumnIds(layout: ViewLayoutCardItem) {
  const columnIds =
    isTemplateMode.value && isTemplatePreviewActive.value
      ? templatePreviewColumnIds(layout)
      : layoutColumnIds(layout);
  const slotCount = sortedSlots(layout).length;

  if (slotCount > 0) {
    return Array.from(
      { length: slotCount },
      (_, index) => columnIds[index] ?? null
    );
  }

  return columnIds;
}

function fieldValue(column: AppColumn) {
  return currentRecord.value?.displayValues[column.columnName] || "未入力";
}

function fieldLabel(columnId: number) {
  return columnById(columnId)?.displayName ?? "";
}

function cardSummaryLabel(layout: ViewLayoutCardItem) {
  if (isTemplateMode.value) {
    const columnIds = resolvedColumnIds(layout).filter(
      (columnId): columnId is number => columnId !== null
    );
    if (isTemplatePreviewActive.value && columnIds.length > 0) {
      const firstLabel = fieldLabel(columnIds[0]);
      return columnIds.length > 1
        ? `${firstLabel} ほか${columnIds.length - 1}件`
        : firstLabel;
    }
    return layout.cardId > 0 ? `カード #${layout.cardId}` : "新規カード";
  }

  const columnIds = layoutColumnIds(layout).filter(
    (columnId): columnId is number => columnId !== null
  );
  if (columnIds.length === 0) {
    return "未紐付け";
  }
  const firstLabel = fieldLabel(columnIds[0]);
  return columnIds.length > 1
    ? `${firstLabel} ほか${columnIds.length - 1}件`
    : firstLabel;
}

function removeButtonLabel(layout: ViewLayoutCardItem) {
  if (isTemplateMode.value) {
    return `${cardSummaryLabel(layout)} を削除`;
  }

  return layout.visible
    ? `${cardSummaryLabel(layout)}を非表示にする`
    : `${cardSummaryLabel(layout)}を表示に戻す`;
}

function removeButtonIcon(layout: ViewLayoutCardItem) {
  if (isTemplateMode.value) {
    return "mdi-close";
  }

  return layout.visible ? "mdi-eye-off-outline" : "mdi-eye-outline";
}

function fieldValueByColumnId(columnId: number) {
  const column = columnById(columnId);
  return column ? fieldValue(column) : "";
}

function fieldEntriesByLayout(layout: ViewLayoutCardItem): FieldEntry[] {
  const slotCount = sortedSlots(layout).length;
  const columnIds = resolvedColumnIds(layout);

  if (slotCount === 0) {
    return columnIds
      .filter((columnId): columnId is number => columnId !== null)
      .map((columnId, index) => ({
        key: `${layout.cardId}:${columnId}:${index}`,
        label: fieldLabel(columnId),
        slot: null,
        value: fieldValueByColumnId(columnId)
      }));
  }

  return sortedSlots(layout).map((slot, index) => {
    const columnId = columnIds[index] ?? null;
    return {
      key: `${layout.cardId}:slot:${slot.slotId}`,
      label: columnId === null ? `スロット ${index + 1}` : fieldLabel(columnId),
      slot,
      value: columnId === null ? "未設定" : fieldValueByColumnId(columnId)
    };
  });
}

function resolvedSlotFontSize(
  layout: ViewLayoutCardItem,
  slot: ViewLayoutTemplateCardSlot | null
) {
  return slot?.fontSize ?? layoutStyleValue(layout, "fontSize");
}

function resolvedSlotTextColor(
  layout: ViewLayoutCardItem,
  slot: ViewLayoutTemplateCardSlot | null
) {
  return slot?.textColor ?? layoutStyleValue(layout, "textColor");
}

function resolvedSlotFontWeight(
  layout: ViewLayoutCardItem,
  slot: ViewLayoutTemplateCardSlot | null
) {
  return slotFontWeightValue(slot) ?? layoutStyleValue(layout, "fontWeight");
}

function resolvedSlotTextAlign(
  layout: ViewLayoutCardItem,
  slot: ViewLayoutTemplateCardSlot | null
) {
  return slotTextAlignValue(slot) ?? layoutStyleValue(layout, "textAlign");
}

function fieldEntryValueStyle(
  layout: ViewLayoutCardItem,
  entry: FieldEntry
): CSSProperties {
  if (slotDisplayFormatValue(entry.slot) === "plain") {
    // v1 keeps the default presentation; storing the value now allows future expansion.
  }

  const fontSize = resolvedSlotFontSize(layout, entry.slot);
  const textColor = resolvedSlotTextColor(layout, entry.slot);
  const fontWeight = resolvedSlotFontWeight(layout, entry.slot);
  const textAlign = resolvedSlotTextAlign(layout, entry.slot);

  return {
    color: textColor ? String(textColor) : undefined,
    fontSize: typeof fontSize === "number" ? `${fontSize}px` : undefined,
    fontWeight: typeof fontWeight === "string" ? fontWeight : undefined,
    textAlign:
      typeof textAlign === "string"
        ? (textAlign as CSSProperties["textAlign"])
        : undefined
  };
}

function cardBindingLabel(layout: ViewLayoutCardItem, index: number) {
  return layout.cardId > 0
    ? `カード #${layout.cardId}`
    : `新規カード ${index + 1}`;
}

function columnDisplayName(columnId: number | null) {
  if (columnId === null) {
    return "未紐付け";
  }
  return columnById(columnId)?.displayName ?? "未紐付け";
}

function setBindingDraft(
  cardId: number,
  index: number,
  columnId: number | null
) {
  selectBindingCard(cardId);
  const next = [...(bindingDraft.value[cardId] ?? [null])];
  next[index] = columnId;
  bindingDraft.value = {
    ...bindingDraft.value,
    [cardId]: normalizedBindingIds(next)
  };
}

function resetBindingDraft() {
  bindingDraft.value = Object.fromEntries(
    draftLayouts.value.map((layout) => [
      layout.cardId,
      resolvedColumnIds(layout)
    ])
  );
}

function selectTemplatePreviewRecord(value: unknown) {
  if (typeof value !== "string") {
    return;
  }
  const item = templatePreviewRecordItems.value.find(
    (candidate) => candidate.value === value
  );
  if (!item) {
    return;
  }

  emit("select-template-preview-record", item.record);
  templatePreviewMenuOpen.value = false;
}

function clearTemplatePreview() {
  emit("clear-template-preview");
}

function assignRecordTemplate(templateId: unknown) {
  const selected = props.selectedItem;
  if (selected?.type !== "tableRecord" || !selected.folderRecordId) {
    return;
  }
  if (
    typeof templateId !== "number" ||
    templateId === props.folderActiveTemplateId ||
    templateId === selected.recordTemplateId
  ) {
    return;
  }
  emit("assign-record-template", selected.folderRecordId, templateId);
}

function clearRecordTemplate() {
  const selected = props.selectedItem;
  if (
    selected?.type !== "tableRecord" ||
    !selected.folderRecordId ||
    !selected.recordTemplateId
  ) {
    return;
  }
  emit("clear-record-template", selected.folderRecordId);
}

/** 紐付けドラフトが元の値から変更されているかを判定します。 */
const hasUnsavedBindingChanges = computed(() => {
  if (!isBindingEditorVisible.value) return false;
  return draftLayouts.value.some(
    (layout) =>
      JSON.stringify(
        normalizedBindingIds(bindingDraft.value[layout.cardId] ?? [null])
      ) !== JSON.stringify(layoutColumnIds(layout))
  );
});

const confirmDialog = useConfirmDialog();

function openBindingEditor() {
  resetBindingDraft();
  isBindingEditorOpen.value = true;
}

function closeBindingEditor() {
  isBindingEditorOpen.value = false;
  selectedBindingCardId.value = null;
}

function addBindingSlot(cardId: number) {
  bindingDraft.value = {
    ...bindingDraft.value,
    [cardId]: [...(bindingDraft.value[cardId] ?? [null]), null]
  };
  selectBindingCard(cardId);
}

function removeBindingSlot(cardId: number, index: number) {
  const current = [...(bindingDraft.value[cardId] ?? [null])];
  if (current.length <= 1) {
    bindingDraft.value = {
      ...bindingDraft.value,
      [cardId]: [null]
    };
    return;
  }
  current.splice(index, 1);
  bindingDraft.value = {
    ...bindingDraft.value,
    [cardId]: normalizedBindingIds(current)
  };
}

function moveBindingSlot(cardId: number, index: number, direction: -1 | 1) {
  const current = [...(bindingDraft.value[cardId] ?? [null])];
  const nextIndex = index + direction;
  if (nextIndex < 0 || nextIndex >= current.length) {
    return;
  }
  const [moved] = current.splice(index, 1);
  current.splice(nextIndex, 0, moved);
  bindingDraft.value = {
    ...bindingDraft.value,
    [cardId]: current
  };
}

function nextTemplateSlotId(layout: ViewLayoutCardItem) {
  return Math.min(0, ...layout.slots.map((slot) => slot.slotId)) - 1;
}

function addTemplateSlot(cardId: number) {
  const layout = draftLayouts.value.find((item) => item.cardId === cardId);
  if (!layout) {
    return;
  }

  const slotId = nextTemplateSlotId(layout);

  updateLayout(
    {
      ...layout,
      slots: [
        ...sortedSlots(layout),
        {
          displayFormat: null,
          fontSize: null,
          fontWeight: null,
          slotId,
          sortOrder: layout.slots.length,
          textAlign: null,
          textColor: null
        }
      ]
    },
    true
  );
  selectedTemplateSlotKey.value = { cardId, slotId };
  setSingleSelection(cardId);
}

function removeTemplateSlot(cardId: number, index: number) {
  const layout = draftLayouts.value.find((item) => item.cardId === cardId);
  if (!layout) {
    return;
  }

  const nextSlots = sortedSlots(layout)
    .filter((_, slotIndex) => slotIndex !== index)
    .map((slot, slotIndex) => ({
      ...slot,
      sortOrder: slotIndex
    }));

  updateLayout(
    {
      ...layout,
      slots: nextSlots
    },
    true
  );
}

function reorderTemplateSlots(cardId: number, orderedSlotIds: number[]) {
  const layout = draftLayouts.value.find((item) => item.cardId === cardId);
  if (!layout || orderedSlotIds.length !== layout.slots.length) {
    return;
  }

  const slotMap = new Map(layout.slots.map((slot) => [slot.slotId, slot]));
  const nextSlots = orderedSlotIds
    .map((slotId, slotIndex) => {
      const slot = slotMap.get(slotId);
      return slot
        ? {
            ...slot,
            sortOrder: slotIndex
          }
        : null;
    })
    .filter((slot): slot is NonNullable<typeof slot> => slot !== null);

  if (nextSlots.length !== layout.slots.length) {
    return;
  }

  updateLayout(
    {
      ...layout,
      slots: nextSlots
    },
    true
  );
}

/**
 * 紐付け設定パネルの開閉を切り替えます。
 * 未保存の変更がある場合は、破棄確認ダイアログを表示します。
 */
async function toggleBindingEditor() {
  // パネルが開いている場合は閉じる動作を優先します。
  if (isBindingEditorOpen.value) {
    if (hasUnsavedBindingChanges.value) {
      const confirmed = await confirmDialog.open({
        title: "紐付けの変更を破棄",
        message: "紐付けの変更が保存されていません。変更を破棄して閉じますか？",
        confirmText: "破棄して閉じる",
        color: "warning"
      });
      if (!confirmed) return;
    }
    closeBindingEditor();
    return;
  }

  // パネルが閉じている場合は開きます。
  openBindingEditor();
}

function selectBindingCard(cardId: number) {
  selectedBindingCardId.value = cardId;
}

function isBindingCardSelected(cardId: number) {
  return selectedBindingCardId.value === cardId;
}

function isBindingSlotCountLocked(layout: ViewLayoutCardItem) {
  return layout.slots.length > 0;
}

function selectTemplateSlot(cardId: number, slotId: number) {
  selectedTemplateSlotKey.value = { cardId, slotId };
  setSingleSelection(cardId);
}

function updateSelectedTemplateSlot(
  updater: (slot: ViewLayoutTemplateCardSlot) => ViewLayoutTemplateCardSlot
) {
  const key = selectedTemplateSlotKey.value;
  const layout = templateSlotTargetLayout.value;
  if (!key || !layout || layout.cardId !== key.cardId) {
    return;
  }

  const nextSlots = sortedSlots(layout).map((slot) =>
    slot.slotId === key.slotId ? updater(slot) : slot
  );
  updateLayout(
    {
      ...layout,
      slots: nextSlots
    },
    true
  );
}

function applySelectedTemplateSlotDisplayFormat(
  value: ViewLayoutSlotDisplayFormat | null
) {
  updateSelectedTemplateSlot((slot) => ({
    ...slot,
    displayFormat: value
  }));
}

function applySelectedTemplateSlotFontSizeFromInput(event: unknown) {
  const rawValue =
    (event as { target?: { value?: string } }).target?.value ?? "";
  if (!rawValue) {
    updateSelectedTemplateSlot((slot) => ({
      ...slot,
      fontSize: null
    }));
    return;
  }

  const value = Number(rawValue);
  if (!Number.isFinite(value)) {
    return;
  }

  updateSelectedTemplateSlot((slot) => ({
    ...slot,
    fontSize: value
  }));
}

function applySelectedTemplateSlotTextColorFromInput(event: unknown) {
  const value = (event as { target?: { value?: string } }).target?.value ?? "";
  if (!value) {
    return;
  }

  updateSelectedTemplateSlot((slot) => ({
    ...slot,
    textColor: value
  }));
}

function resetSelectedTemplateSlotTextColor() {
  updateSelectedTemplateSlot((slot) => ({
    ...slot,
    textColor: null
  }));
}

function applySelectedTemplateSlotFontWeight(value: SlotFontWeightValue) {
  updateSelectedTemplateSlot((slot) => ({
    ...slot,
    fontWeight: value === "inherit" ? null : value
  }));
}

function applySelectedTemplateSlotTextAlign(value: SlotTextAlignValue) {
  updateSelectedTemplateSlot((slot) => ({
    ...slot,
    textAlign: value === "inherit" ? null : value
  }));
}

function saveBindingDraft() {
  if (!canSaveBindingDraft.value) {
    return;
  }

  emit(
    "save-card-column-bindings",
    Object.entries(bindingDraft.value)
      .flatMap(([cardId, columnIds]) =>
        normalizedBindingIds(columnIds).map((columnId, sortOrder) => ({
          cardId: Number(cardId),
          columnId,
          sortOrder
        }))
      )
      .filter(
        (binding): binding is ViewLayoutCardColumnBinding =>
          Number.isFinite(binding.cardId) && binding.columnId !== null
      )
  );
  if (!hasUnboundTemplateForTable.value) {
    closeBindingEditor();
  }
}

function shouldShowLabel(layout: ViewLayoutCardItem) {
  return layoutStyleValue(layout, "showLabel") !== false;
}

function baseLayoutForCard(cardId: number, fallback: ViewLayoutCardItem) {
  return (
    visibleLayouts.value.find((item) => item.cardId === cardId) ?? fallback
  );
}

function normalizedPresetId(layout: { presetId?: string | null }) {
  return isKnownCardPresetId(layout.presetId)
    ? (layout.presetId ?? null)
    : null;
}

function applyPresetId(value: string | null) {
  if (!isTemplateMode.value || selectedLayouts.value.length === 0) {
    return;
  }

  const presetId =
    typeof value === "string" && isKnownCardPresetId(value) ? value : null;
  updateLayouts(
    selectedLayouts.value.map((layout) => ({
      ...layout,
      presetId
    })),
    true
  );
}

function canvasBounds() {
  return {
    height: WORLD_HEIGHT,
    width: WORLD_WIDTH
  };
}

function canvasPoint(event: PointerLikeEvent) {
  const rect = canvasRef.value?.getBoundingClientRect();
  if (!rect) {
    return { x: 0, y: 0 };
  }
  return {
    x:
      (event.clientX - rect.left - viewportOffsetX.value) / viewportScale.value,
    y: (event.clientY - rect.top - viewportOffsetY.value) / viewportScale.value
  };
}

function viewportPoint(event: PointerLikeEvent | WheelLikeEvent) {
  const rect = canvasRef.value?.getBoundingClientRect();
  if (!rect) {
    return { x: 0, y: 0 };
  }

  return {
    x: event.clientX - rect.left,
    y: event.clientY - rect.top
  };
}

function resetViewport() {
  viewportScale.value = 1;
  viewportOffsetX.value = 0;
  viewportOffsetY.value = 0;
}

function zoomAt(scale: number, viewportX: number, viewportY: number) {
  const nextScale = clampViewportScale(scale);
  const currentScale = viewportScale.value;
  if (nextScale === currentScale) {
    return;
  }

  const worldX = (viewportX - viewportOffsetX.value) / currentScale;
  const worldY = (viewportY - viewportOffsetY.value) / currentScale;
  viewportScale.value = nextScale;
  viewportOffsetX.value = viewportX - worldX * nextScale;
  viewportOffsetY.value = viewportY - worldY * nextScale;
}

function zoomFromCenter(delta: number) {
  const canvas = canvasRef.value;
  const centerX = canvas ? canvas.clientWidth / 2 : 0;
  const centerY = canvas ? canvas.clientHeight / 2 : 0;
  zoomAt(viewportScale.value + delta, centerX, centerY);
}

function handleCanvasWheel(event: WheelLikeEvent) {
  if (!event.ctrlKey) {
    return;
  }

  event.preventDefault();
  const point = viewportPoint(event);
  const direction = event.deltaY > 0 ? -1 : 1;
  zoomAt(
    viewportScale.value + direction * VIEWPORT_ZOOM_STEP,
    point.x,
    point.y
  );
}

function clampLayout(item: ViewLayoutCardItem): ViewLayoutCardItem {
  const bounds = canvasBounds();
  const width = Math.max(MIN_CARD_WIDTH, Math.min(item.width, bounds.width));
  const height = Math.max(
    MIN_CARD_HEIGHT,
    Math.min(item.height, bounds.height)
  );

  return {
    ...item,
    height,
    width,
    x: Math.max(0, Math.min(item.x, Math.max(0, bounds.width - width))),
    y: Math.max(0, Math.min(item.y, Math.max(0, bounds.height - height)))
  };
}

function updateLayouts(items: ViewLayoutCardItem[], save: boolean) {
  const nextItems = items.map((item) =>
    clampLayout({
      ...item,
      hasOverride: isTemplateMode.value ? false : true
    })
  );
  const byCardId = new Map(
    nextItems.map((item) => [item.cardId, item] as const)
  );

  draftLayouts.value = draftLayouts.value.map(
    (layout) => byCardId.get(layout.cardId) ?? layout
  );

  for (const item of nextItems) {
    if (!draftLayouts.value.some((layout) => layout.cardId === item.cardId)) {
      draftLayouts.value = [...draftLayouts.value, item];
    }
  }

  if (save) {
    emitDraftLayouts();
  }
}

function updateLayout(item: ViewLayoutCardItem, save: boolean) {
  updateLayouts([item], save);
}

function sharedLayoutProperty<T>(
  selector: (layout: ViewLayoutCardItem) => T
): T | "" {
  const [firstLayout, ...restLayouts] = selectedLayouts.value;
  if (!firstLayout) {
    return "";
  }

  const firstValue = selector(firstLayout);
  const hasMixedValues = restLayouts.some(
    (layout) => selector(layout) !== firstValue
  );
  return hasMixedValues ? "" : firstValue;
}

const autoHeightEnabledInspectorValue = computed(
  () => sharedLayoutProperty((layout) => layout.autoHeightEnabled) === true
);
const pushDownSiblingsInspectorValue = computed(
  () => sharedLayoutProperty((layout) => layout.pushDownSiblings) === true
);
const maxAutoHeightInspectorValue = computed<number | "">(() => {
  const value = sharedLayoutProperty((layout) => layout.maxAutoHeight ?? null);
  return typeof value === "number" ? value : "";
});
const maxAutoHeightBehaviorInspectorValue =
  computed<ViewLayoutAutoHeightBehavior | null>(() => {
    const value = sharedLayoutProperty(
      (layout) => layout.maxAutoHeightBehavior
    );
    return value === "" ? null : value;
  });

function updateSelectedLayouts(
  updater: (layout: ViewLayoutCardItem) => ViewLayoutCardItem
) {
  if (selectedLayouts.value.length === 0) {
    return;
  }

  updateLayouts(selectedLayouts.value.map(updater), true);
}

function applyAutoHeightEnabledFromCheckbox(value: boolean | null) {
  updateSelectedLayouts((layout) => ({
    ...layout,
    autoHeightEnabled: value === true
  }));
}

function applyPushDownSiblingsFromCheckbox(value: boolean | null) {
  updateSelectedLayouts((layout) => ({
    ...layout,
    pushDownSiblings: value === true
  }));
}

function applyMaxAutoHeightFromInput(event: unknown) {
  const value = Number(
    (event as { target?: { value?: string } }).target?.value
  );
  updateSelectedLayouts((layout) => ({
    ...layout,
    maxAutoHeight: Number.isFinite(value) && value > 0 ? value : null
  }));
}

function applyMaxAutoHeightBehavior(
  value: ViewLayoutAutoHeightBehavior | null
) {
  if (!value) {
    return;
  }

  updateSelectedLayouts((layout) => ({
    ...layout,
    maxAutoHeightBehavior: value
  }));
}

function setCardContentMeasureElement(cardId: number, element: unknown) {
  if (element instanceof HTMLElement) {
    cardContentMeasureElements.set(cardId, element);
    return;
  }

  cardContentMeasureElements.delete(cardId);
  if (cardId in measuredCardContentHeights.value) {
    const next = { ...measuredCardContentHeights.value };
    delete next[cardId];
    measuredCardContentHeights.value = next;
  }
}

function measureCardContentHeights() {
  const nextEntries = [...cardContentMeasureElements.entries()].map(
    ([cardId, element]) => [cardId, Math.ceil(element.scrollHeight)] as const
  );
  const nextHeights = Object.fromEntries(nextEntries);
  if (
    JSON.stringify(nextHeights) ===
    JSON.stringify(measuredCardContentHeights.value)
  ) {
    return;
  }
  measuredCardContentHeights.value = nextHeights;
}

function horizontalRangesOverlap(
  left: Pick<ViewLayoutCardItem, "x" | "width">,
  right: Pick<ViewLayoutCardItem, "x" | "width">
) {
  return left.x < right.x + right.width && left.x + left.width > right.x;
}

function resolvedPaddingValue(
  layout: ViewLayoutCardItem,
  key: "paddingTop" | "paddingBottom"
) {
  const sideValue = layout[key];
  if (typeof sideValue === "number") {
    return sideValue;
  }
  if (typeof layout.padding === "number") {
    return layout.padding;
  }
  return DEFAULT_CARD_STYLE[key];
}

function naturalCardHeight(layout: ViewLayoutCardItem) {
  if (!shouldMeasureAutoHeight.value || !layout.autoHeightEnabled) {
    return null;
  }

  const contentHeight = measuredCardContentHeights.value[layout.cardId];
  if (!Number.isFinite(contentHeight)) {
    return null;
  }

  const paddingTop = resolvedPaddingValue(layout, "paddingTop");
  const paddingBottom = resolvedPaddingValue(layout, "paddingBottom");
  return contentHeight + paddingTop + paddingBottom;
}

function buildRenderedLayouts(
  layouts: ViewLayoutCardItem[]
): RenderedViewLayoutCardItem[] {
  const originalLayoutMetrics = new Map(
    layouts.map((layout) => [
      layout.cardId,
      {
        bottom: layout.y + layout.height,
        height: layout.height,
        y: layout.y
      }
    ])
  );

  const rendered = layouts.map((layout) => {
    const baseHeight = layout.height;
    const naturalHeight = naturalCardHeight(layout);
    const expandedHeight =
      layout.autoHeightEnabled && naturalHeight !== null
        ? Math.max(baseHeight, naturalHeight)
        : baseHeight;
    const maxAutoHeight =
      typeof layout.maxAutoHeight === "number" && layout.maxAutoHeight > 0
        ? Math.max(baseHeight, layout.maxAutoHeight)
        : null;
    const displayHeight =
      maxAutoHeight === null
        ? expandedHeight
        : Math.min(expandedHeight, maxAutoHeight);
    const bodyViewportHeight = Math.max(
      0,
      displayHeight -
        resolvedPaddingValue(layout, "paddingTop") -
        resolvedPaddingValue(layout, "paddingBottom")
    );
    const measuredContentHeight =
      measuredCardContentHeights.value[layout.cardId];
    const isOverflowing =
      layout.autoHeightEnabled &&
      maxAutoHeight !== null &&
      expandedHeight > maxAutoHeight &&
      Number.isFinite(measuredContentHeight) &&
      measuredContentHeight > bodyViewportHeight;
    const renderBodyMode = isOverflowing
      ? layout.maxAutoHeightBehavior
      : "normal";
    const renderContentScale =
      renderBodyMode === "scaleToFit" &&
      Number.isFinite(measuredContentHeight) &&
      measuredContentHeight > 0
        ? Math.min(1, bodyViewportHeight / measuredContentHeight)
        : 1;

    return {
      ...layout,
      height: displayHeight,
      renderBaseHeight: baseHeight,
      renderBodyMode,
      renderContentScale,
      renderContentViewportHeight:
        renderBodyMode === "normal" ? null : bodyViewportHeight,
      renderNaturalHeight: naturalHeight
    } satisfies RenderedViewLayoutCardItem;
  });

  const ordered = [...rendered].sort(
    (left, right) =>
      left.y - right.y || left.x - right.x || left.cardId - right.cardId
  );

  for (let index = 0; index < ordered.length; index += 1) {
    const current = ordered[index];
    if (
      !current.autoHeightEnabled ||
      !current.pushDownSiblings ||
      current.height <= current.renderBaseHeight
    ) {
      continue;
    }

    const currentBottom = current.y + current.height;
    for (
      let followerIndex = index + 1;
      followerIndex < ordered.length;
      followerIndex += 1
    ) {
      const follower = ordered[followerIndex];
      if (!horizontalRangesOverlap(current, follower)) {
        continue;
      }
      if (follower.y >= currentBottom || follower.y < current.y) {
        continue;
      }

      const originalCurrentMetrics = originalLayoutMetrics.get(current.cardId);
      const originalFollowerMetrics = originalLayoutMetrics.get(
        follower.cardId
      );
      const originalCurrentBottom = originalCurrentMetrics
        ? originalCurrentMetrics.bottom
        : current.y + current.renderBaseHeight;
      const originalGap = originalFollowerMetrics
        ? originalFollowerMetrics.y - originalCurrentBottom
        : follower.y - (current.y + current.renderBaseHeight);
      const preservedGap = Math.max(0, originalGap);

      follower.y = currentBottom + preservedGap;
    }
  }

  return rendered;
}

function cardContentBodyClass(
  layout: ViewLayoutCardItem | RenderedViewLayoutCardItem
) {
  const mode = "renderBodyMode" in layout ? layout.renderBodyMode : "normal";
  return {
    "is-scrollable": mode === "scroll",
    "is-truncated": mode === "truncate"
  };
}

function cardContentBodyStyle(
  layout: ViewLayoutCardItem | RenderedViewLayoutCardItem
): CSSProperties {
  return {
    maxHeight:
      "renderContentViewportHeight" in layout &&
      layout.renderContentViewportHeight !== null
        ? `${layout.renderContentViewportHeight}px`
        : undefined
  };
}

function cardContentInnerStyle(
  layout: ViewLayoutCardItem | RenderedViewLayoutCardItem
): CSSProperties {
  if (!("renderBodyMode" in layout) || layout.renderBodyMode !== "scaleToFit") {
    return {};
  }

  return {
    transform: `scale(${layout.renderContentScale})`,
    transformOrigin: "top left",
    width:
      layout.renderContentScale > 0
        ? `${100 / layout.renderContentScale}%`
        : "100%"
  };
}

const {
  applyBackgroundColorMode,
  applyFontWeightFromCheckbox,
  applyNumberStyleFromInput,
  applySelectedStyle,
  applyShowLabelFromCheckbox,
  applyStyleFromInput,
  backgroundColorInputValue,
  cardContentStyle,
  cardStyle,
  canOverrideBackgroundColor,
  hasTransparentBackground,
  isTransparentBackgroundSelected,
  layoutStyleValue,
  resetSelectedStyle,
  styleBooleanInputValue,
  styleInputValue,
  styleInspectorValues,
  styleNumberInputValue,
  themeColorInputValue
} = useFreeLayoutStyleEditing(selectedLayouts, updateLayouts);

function applyBackgroundColorModeWithPolicy(mode: "color" | "transparent") {
  if (backgroundColorEditingDisabled.value) {
    return;
  }

  applyBackgroundColorMode(mode);
}

function applyStyleFromInputWithPolicy(key: LayoutStyleKey, event: unknown) {
  if (key === "backgroundColor" && backgroundColorEditingDisabled.value) {
    return;
  }

  applyStyleFromInput(key, event);
}

function pointerTarget(event: PointerLikeEvent) {
  return event.currentTarget as PointerTarget | null;
}

function isSelected(cardId: number) {
  return selectedCardIds.value.includes(cardId);
}

function clearSelection() {
  selectedCardIds.value = [];
}

function pruneSelection() {
  const visibleCardIds = new Set(
    draftLayouts.value
      .filter((item) =>
        isTemplateMode.value
          ? item.visible || editMode.value
          : isBoundToCurrentTable(item) && (item.visible || editMode.value)
      )
      .map((item) => item.cardId)
  );
  selectedCardIds.value = selectedCardIds.value.filter((cardId) =>
    visibleCardIds.has(cardId)
  );
}

function setSingleSelection(cardId: number) {
  selectedCardIds.value = [cardId];
}

function setSelection(cardIds: number[]) {
  const validIds = new Set(canvasLayouts.value.map((layout) => layout.cardId));
  selectedCardIds.value = [...new Set(cardIds)].filter((cardId) =>
    validIds.has(cardId)
  );
}

function toggleSelection(cardId: number) {
  selectedCardIds.value = isSelected(cardId)
    ? selectedCardIds.value.filter((id) => id !== cardId)
    : [...selectedCardIds.value, cardId];
}

function handleKeyDown(event: KeyboardLikeEvent) {
  if (event.key === "Escape") {
    clearSelection();
    selectionBox.value = null;
    selectionDrag.value = null;
  }
  if (event.key === " " && !event.repeat) {
    event.preventDefault?.();
    isSpacePressed.value = true;
  }
}

function handleKeyUp(event: KeyboardLikeEvent) {
  if (event.key === " ") {
    isSpacePressed.value = false;
    panDrag.value = null;
  }
}

function isPastDragThreshold(dx: number, dy: number) {
  return Math.hypot(dx, dy) > DRAG_THRESHOLD;
}

function startPan(event: PointerLikeEvent) {
  event.preventDefault();
  event.stopPropagation();
  pointerTarget(event)?.setPointerCapture?.(event.pointerId);
  panDrag.value = {
    startOffsetX: viewportOffsetX.value,
    startOffsetY: viewportOffsetY.value,
    startX: event.clientX,
    startY: event.clientY
  };
}

function movePan(event: PointerLikeEvent) {
  const current = panDrag.value;
  if (!current) {
    return;
  }

  viewportOffsetX.value = current.startOffsetX + event.clientX - current.startX;
  viewportOffsetY.value = current.startOffsetY + event.clientY - current.startY;
}

function endPan() {
  panDrag.value = null;
}

function shouldStartPan(event: PointerLikeEvent) {
  return event.button === 1 || (isSpacePressed.value && event.button === 0);
}

function fitViewportToContent() {
  const canvas = canvasRef.value;
  const layouts = canvasLayouts.value;
  if (!canvas || layouts.length === 0) {
    resetViewport();
    return;
  }

  const padding = 48;
  const minX = Math.min(...layouts.map((layout) => layout.x));
  const minY = Math.min(...layouts.map((layout) => layout.y));
  const maxX = Math.max(...layouts.map((layout) => layout.x + layout.width));
  const maxY = Math.max(...layouts.map((layout) => layout.y + layout.height));
  const contentWidth = Math.max(1, maxX - minX);
  const contentHeight = Math.max(1, maxY - minY);
  const nextScale = clampViewportScale(
    Math.min(
      (canvas.clientWidth - padding * 2) / contentWidth,
      (canvas.clientHeight - padding * 2) / contentHeight
    )
  );

  viewportScale.value = nextScale;
  viewportOffsetX.value =
    (canvas.clientWidth - contentWidth * nextScale) / 2 - minX * nextScale;
  viewportOffsetY.value =
    (canvas.clientHeight - contentHeight * nextScale) / 2 - minY * nextScale;
}

function startCardPointer(
  event: PointerLikeEvent,
  cardId: number,
  layout: ViewLayoutCardItem
) {
  if (shouldStartPan(event)) {
    startPan(event);
    return;
  }

  if (!editMode.value) {
    return;
  }

  event.preventDefault();
  event.stopPropagation();
  pointerTarget(event)?.setPointerCapture?.(event.pointerId);

  const originalSelection = [...selectedCardIds.value];
  if (!isSelected(cardId)) {
    setSingleSelection(cardId);
  }

  const movingIds = isSelected(cardId) ? selectedCardIds.value : [cardId];
  const origins = visibleLayouts.value
    .filter((item) => movingIds.includes(item.cardId))
    .map((item) => ({ ...item }));

  interaction.value = {
    cardId,
    cardIds: origins.length > 0 ? origins.map((item) => item.cardId) : [cardId],
    moved: false,
    originLayouts: origins.length > 0 ? origins : [{ ...layout }],
    originalSelection,
    shiftKey: event.shiftKey,
    startX: event.clientX,
    startY: event.clientY,
    type: "move"
  };
}

function startResize(
  event: PointerLikeEvent,
  cardId: number,
  layout: ViewLayoutCardItem,
  direction: ResizeDirection
) {
  if (shouldStartPan(event)) {
    startPan(event);
    return;
  }

  if (!editMode.value) {
    return;
  }

  event.preventDefault();
  event.stopPropagation();
  pointerTarget(event)?.setPointerCapture?.(event.pointerId);
  setSingleSelection(cardId);
  interaction.value = {
    cardId,
    direction,
    moved: false,
    origin: { ...layout },
    startX: event.clientX,
    startY: event.clientY,
    type: "resize"
  };
}

function movePointer(event: PointerLikeEvent) {
  const current = interaction.value;
  if (!current) {
    return;
  }

  const dx = (event.clientX - current.startX) / viewportScale.value;
  const dy = (event.clientY - current.startY) / viewportScale.value;
  if (!current.moved && !isPastDragThreshold(dx, dy)) {
    return;
  }
  current.moved = true;

  if (current.type === "move") {
    updateLayouts(
      current.originLayouts.map((origin) => ({
        ...origin,
        x: origin.x + dx,
        y: origin.y + dy
      })),
      false
    );
    return;
  }

  let nextX = current.origin.x;
  let nextY = current.origin.y;
  let nextWidth = current.origin.width;
  let nextHeight = current.origin.height;
  if (current.direction.includes("e")) {
    nextWidth = Math.max(MIN_CARD_WIDTH, current.origin.width + dx);
  }
  if (current.direction.includes("s")) {
    nextHeight = Math.max(MIN_CARD_HEIGHT, current.origin.height + dy);
  }
  if (current.direction.includes("w")) {
    nextWidth = Math.max(MIN_CARD_WIDTH, current.origin.width - dx);
    nextX = current.origin.x + current.origin.width - nextWidth;
  }
  if (current.direction.includes("n")) {
    nextHeight = Math.max(MIN_CARD_HEIGHT, current.origin.height - dy);
    nextY = current.origin.y + current.origin.height - nextHeight;
  }

  updateLayout(
    {
      ...current.origin,
      height: nextHeight,
      width: nextWidth,
      x: nextX,
      y: nextY
    },
    false
  );
}

function endPointer() {
  const current = interaction.value;
  if (!current) {
    return;
  }

  interaction.value = null;

  if (current.type === "move" && !current.moved) {
    selectedCardIds.value = current.originalSelection;
    if (current.shiftKey) {
      toggleSelection(current.cardId);
    } else {
      setSingleSelection(current.cardId);
    }
    return;
  }

  if (current.moved) {
    emitDraftLayouts();
  }
}

function idsInSelectionBox(box: SelectionBox) {
  return canvasLayouts.value
    .filter((layout) =>
      intersects(box, {
        height: layout.height,
        width: layout.width,
        x: layout.x,
        y: layout.y
      })
    )
    .map((layout) => layout.cardId);
}

function applySelectionBox(box: SelectionBox, drag: SelectionDrag) {
  const boxIds = idsInSelectionBox(box);
  setSelection(drag.additive ? [...drag.initialSelection, ...boxIds] : boxIds);
}

function startSelectionDrag(event: PointerLikeEvent) {
  if (shouldStartPan(event)) {
    startPan(event);
    return;
  }

  if (!editMode.value || panDrag.value) {
    return;
  }

  event.preventDefault();
  pointerTarget(event)?.setPointerCapture?.(event.pointerId);
  const point = canvasPoint(event);
  selectionDrag.value = {
    additive: event.shiftKey,
    initialSelection: [...selectedCardIds.value],
    moved: false,
    startX: point.x,
    startY: point.y
  };
  selectionBox.value = null;
}

function moveSelectionDrag(event: PointerLikeEvent) {
  const drag = selectionDrag.value;
  if (!drag) {
    return;
  }

  const point = canvasPoint(event);
  const dx = point.x - drag.startX;
  const dy = point.y - drag.startY;
  if (!drag.moved && !isPastDragThreshold(dx, dy)) {
    return;
  }

  drag.moved = true;
  const box = boxFromPoints(drag.startX, drag.startY, point.x, point.y);
  selectionBox.value = box;
  applySelectionBox(box, drag);
}

function endSelectionDrag() {
  const drag = selectionDrag.value;
  if (!drag) {
    return;
  }

  if (!drag.moved) {
    clearSelection();
  }

  selectionDrag.value = null;
  selectionBox.value = null;
}

function removeOrHideCard(cardId: number) {
  const current = draftLayouts.value.find((item) => item.cardId === cardId);
  if (!current) {
    return;
  }

  if (isTemplateMode.value) {
    draftLayouts.value = draftLayouts.value.filter(
      (item) => item.cardId !== cardId
    );
    selectedCardIds.value = selectedCardIds.value.filter(
      (selectedCardId) => selectedCardId !== cardId
    );
    emitDraftLayouts();
    return;
  }

  selectedCardIds.value = selectedCardIds.value.filter(
    (selectedCardId) => selectedCardId !== cardId
  );
  updateLayout({ ...current, visible: !current.visible }, true);
}

function resetSelectedCardOverride() {
  const [cardId] = selectedCardIds.value;
  if (!cardId) {
    return;
  }
  emit("reset-card-override", cardId);
}

function resetRecordOverrides() {
  emit("reset-record-overrides");
}

function addTemplateCard() {
  const minCardId = Math.min(
    0,
    ...draftLayouts.value.map((item) => item.cardId)
  );
  const cardId = minCardId - 1;
  const offset = draftLayouts.value.length * 24;
  const next: ViewLayoutCardItem = {
    tableId: 0,
    cardId,
    columns: [],
    slots: [],
    presetId: null,
    label: null,
    x: 120 + offset,
    y: 120 + offset,
    width: 220,
    height: 120,
    visible: true,
    backgroundColor: null,
    textColor: null,
    fontSize: null,
    textDirection: null,
    fontWeight: null,
    textAlign: null,
    padding: null,
    paddingTop: null,
    paddingRight: null,
    paddingBottom: null,
    paddingLeft: null,
    borderRadius: null,
    showLabel: null,
    autoHeightEnabled: false,
    pushDownSiblings: false,
    maxAutoHeight: null,
    maxAutoHeightBehavior: "scaleToFit",
    hasOverride: false
  };
  draftLayouts.value = [...draftLayouts.value, next];
  setSingleSelection(cardId);
  emitDraftLayouts();
}

function emitDraftLayouts() {
  if (isTemplateMode.value) {
    emit("save-template-cards", draftLayouts.value.map(layoutToTemplateCard));
    return;
  }
  emit("save-record-overrides", draftLayouts.value);
}

function templateCardToLayout(
  card: ViewLayoutTemplateCard
): ViewLayoutCardItem {
  return {
    tableId: 0,
    cardId: card.cardId,
    columns: [],
    slots: sortedSlots(card),
    presetId: normalizedPresetId(card),
    label: null,
    x: card.x,
    y: card.y,
    width: card.width,
    height: card.height,
    visible: card.visible,
    backgroundColor: card.backgroundColor,
    textColor: card.textColor,
    fontSize: card.fontSize,
    textDirection: card.textDirection,
    fontWeight: card.fontWeight,
    textAlign: card.textAlign,
    padding: card.padding,
    paddingTop: card.paddingTop,
    paddingRight: card.paddingRight,
    paddingBottom: card.paddingBottom,
    paddingLeft: card.paddingLeft,
    borderRadius: card.borderRadius,
    showLabel: card.showLabel,
    autoHeightEnabled: card.autoHeightEnabled,
    pushDownSiblings: card.pushDownSiblings,
    maxAutoHeight: card.maxAutoHeight ?? null,
    maxAutoHeightBehavior: card.maxAutoHeightBehavior,
    hasOverride: false
  };
}

function layoutToTemplateCard(
  layout: ViewLayoutCardItem
): ViewLayoutTemplateCard {
  return {
    cardId: layout.cardId,
    slots: sortedSlots(layout).map((slot, index) => ({
      displayFormat: slot.displayFormat ?? null,
      fontSize: slot.fontSize ?? null,
      fontWeight: slot.fontWeight ?? null,
      slotId: slot.slotId,
      sortOrder: index,
      textAlign: slot.textAlign ?? null,
      textColor: slot.textColor ?? null
    })),
    presetId: normalizedPresetId(layout),
    label: null,
    x: layout.x,
    y: layout.y,
    width: layout.width,
    height: layout.height,
    visible: layout.visible,
    backgroundColor: layout.backgroundColor,
    textColor: layout.textColor,
    fontSize: layout.fontSize,
    textDirection: layout.textDirection,
    fontWeight: layout.fontWeight,
    textAlign: layout.textAlign,
    padding: layout.padding,
    paddingTop: layout.paddingTop,
    paddingRight: layout.paddingRight,
    paddingBottom: layout.paddingBottom,
    paddingLeft: layout.paddingLeft,
    borderRadius: layout.borderRadius,
    showLabel: layout.showLabel,
    autoHeightEnabled: layout.autoHeightEnabled,
    pushDownSiblings: layout.pushDownSiblings,
    maxAutoHeight: layout.maxAutoHeight ?? null,
    maxAutoHeightBehavior: layout.maxAutoHeightBehavior
  };
}
</script>

<template>
  <section ref="panelRef" class="view-free-layout-panel">
    <ViewFreeLayoutToolbar
      :can-edit-bindings="
        !isTemplateMode &&
        displayColumns.length > 0 &&
        !hasUnboundTemplateForTable
      "
      :edit-mode="editMode"
      :is-binding-editor-visible="isBindingEditorVisible"
      :is-template-mode="isTemplateMode"
      :is-template-preview-active="isTemplatePreviewActive"
      :saving="saving"
      :selected-template-preview-record-key="selectedTemplatePreviewRecordKey"
      :template-name="templateName"
      :template-preview-label="templatePreviewLabel"
      :template-preview-loading="templatePreviewLoading"
      :template-preview-menu-open="templatePreviewMenuOpen"
      :template-preview-record-items="templatePreviewRecordItems"
      :viewport-percent="viewportPercent"
      @add-template-card="addTemplateCard"
      @clear-template-preview="clearTemplatePreview"
      @fit-viewport="fitViewportToContent"
      @reset-viewport="resetViewport"
      @select-template-preview-record="selectTemplatePreviewRecord"
      @toggle-binding-editor="toggleBindingEditor"
      @update:edit-mode="editMode = Boolean($event)"
      @update:template-preview-menu-open="templatePreviewMenuOpen = $event"
      @zoom-from-center="zoomFromCenter"
    />
    <div class="view-layout-workspace">
      <div
        ref="canvasRef"
        class="view-free-canvas"
        :class="{
          editing: editMode,
          panning: Boolean(panDrag),
          'pan-ready': isSpacePressed
        }"
        @pointerdown="startSelectionDrag"
        @pointermove="
          moveSelectionDrag($event);
          movePan($event);
        "
        @pointerup="
          endSelectionDrag();
          endPan();
        "
        @pointercancel="
          endSelectionDrag();
          endPan();
        "
        @wheel="handleCanvasWheel"
      >
        <div class="view-free-world" :style="worldLayerStyle">
          <div
            v-for="(layout, index) in canvasLayouts"
            :key="layout.cardId"
            class="view-field-card"
            :class="{
              editing: editMode,
              ghost: isBindingEditorVisible,
              hidden:
                !isTemplateMode &&
                editMode &&
                !isBindingEditorVisible &&
                !layout.visible,
              overridden: layout.hasOverride,
              selected: isBindingEditorVisible
                ? isBindingCardSelected(layout.cardId)
                : isSelected(layout.cardId),
              transparent:
                hasTransparentBackground(layout) &&
                !editMode &&
                !isBindingEditorVisible
            }"
            :data-card-preset="normalizedPresetId(layout) ?? undefined"
            :style="cardStyle(layout)"
            @pointerdown.stop="
              isBindingEditorVisible
                ? selectBindingCard(layout.cardId)
                : startCardPointer(
                    $event,
                    layout.cardId,
                    baseLayoutForCard(layout.cardId, layout)
                  )
            "
            @pointermove="
              movePointer($event);
              movePan($event);
            "
            @pointerup="
              endPointer();
              endPan();
            "
            @pointercancel="
              endPointer();
              endPan();
            "
          >
            <template v-if="isBindingEditorVisible">
              <span class="view-binding-card-number">{{ index + 1 }}</span>
              <div class="view-field-card-content">
                <div class="view-field-card-header">
                  <span>{{ cardBindingLabel(layout, index) }}</span>
                </div>
                <div class="view-field-card-stack">
                  <p
                    v-for="(columnId, bindingIndex) in bindingDraft[
                      layout.cardId
                    ] ?? [null]"
                    :key="`${layout.cardId}:${bindingIndex}`"
                    class="view-field-card-value"
                  >
                    {{ columnDisplayName(columnId ?? null) }}
                  </p>
                </div>
              </div>
            </template>
            <template
              v-else-if="
                shouldRenderCardContent ||
                (!isTemplateMode &&
                  (layout.slots.length > 0 || layout.columns.length > 0))
              "
            >
              <v-tooltip
                v-if="editMode"
                :text="removeButtonLabel(layout)"
                location="bottom"
              >
                <template #activator="{ props: tooltipProps }">
                  <button
                    v-bind="tooltipProps"
                    type="button"
                    class="view-field-remove-button"
                    :class="{ 'toggle-visibility': !isTemplateMode }"
                    :aria-label="removeButtonLabel(layout)"
                    @pointerdown.stop.prevent
                    @click.stop="removeOrHideCard(layout.cardId)"
                  >
                    <v-icon size="13" :icon="removeButtonIcon(layout)" />
                  </button>
                </template>
              </v-tooltip>
              <span
                v-if="!isTemplateMode && !layout.visible"
                class="view-field-hidden-chip"
              >
                非表示
              </span>
              <span
                v-else-if="layout.hasOverride"
                class="view-field-override-chip"
              >
                個別差分
              </span>
              <div
                class="view-field-card-content"
                :class="{ 'hide-label': !shouldShowLabel(layout) }"
                :style="cardContentStyle(layout)"
              >
                <div
                  class="view-field-card-body"
                  :class="cardContentBodyClass(layout)"
                  :style="cardContentBodyStyle(layout)"
                >
                  <div
                    :ref="
                      (element) =>
                        setCardContentMeasureElement(layout.cardId, element)
                    "
                    class="view-field-card-body-inner"
                    :style="cardContentInnerStyle(layout)"
                  >
                    <div class="view-field-card-stack">
                      <div
                        v-for="entry in fieldEntriesByLayout(layout)"
                        :key="entry.key"
                        class="view-field-card-row"
                      >
                        <div
                          v-if="shouldShowLabel(layout)"
                          class="view-field-card-header"
                        >
                          <span>{{ entry.label }}</span>
                        </div>
                        <p
                          class="view-field-card-value"
                          :style="fieldEntryValueStyle(layout, entry)"
                        >
                          {{ entry.value }}
                        </p>
                      </div>
                      <p
                        v-if="
                          isTemplateMode &&
                          !isTemplatePreviewActive &&
                          fieldEntriesByLayout(layout).length === 0
                        "
                        class="view-field-card-value"
                      >
                        カード枠
                      </p>
                      <p
                        v-else-if="
                          isTemplateMode &&
                          isTemplatePreviewActive &&
                          fieldEntriesByLayout(layout).length === 0
                        "
                        class="view-field-card-value"
                      >
                        一時紐付けなし
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            </template>
            <template v-if="editMode && isSelected(layout.cardId)">
              <v-tooltip
                v-for="handle in RESIZE_HANDLES"
                :key="handle.direction"
                :text="`${cardSummaryLabel(layout)}を${handle.label}`"
                location="bottom"
              >
                <template #activator="{ props: tooltipProps }">
                  <button
                    v-bind="tooltipProps"
                    type="button"
                    class="view-field-resize-handle"
                    :class="`view-field-resize-handle-${handle.direction}`"
                    :aria-label="`${cardSummaryLabel(layout)}を${handle.label}`"
                    @pointerdown.stop="
                      startResize(
                        $event,
                        layout.cardId,
                        baseLayoutForCard(layout.cardId, layout),
                        handle.direction
                      )
                    "
                    @pointermove="
                      movePointer($event);
                      movePan($event);
                    "
                    @pointerup="
                      endPointer();
                      endPan();
                    "
                    @pointercancel="
                      endPointer();
                      endPan();
                    "
                  />
                </template>
              </v-tooltip>
            </template>
          </div>
          <div
            v-if="selectionBox && !isBindingEditorVisible"
            class="view-selection-box"
            :style="{
              transform: `translate(${selectionBox.x}px, ${selectionBox.y}px)`,
              width: `${selectionBox.width}px`,
              height: `${selectionBox.height}px`
            }"
          />
        </div>
        <div
          v-if="!isBindingEditorVisible && visibleLayouts.length === 0"
          class="view-free-canvas-empty"
        >
          <h3>表示できる項目がありません</h3>
          <p>テンプレート編集でカード枠を追加してください。</p>
        </div>
      </div>
      <ViewFreeLayoutBindingPanel
        v-if="isBindingEditorVisible"
        :bindable-template-layouts="bindableTemplateLayouts"
        :binding-column-items="bindingColumnItems"
        :binding-draft="bindingDraft"
        :can-save-binding-draft="canSaveBindingDraft"
        :card-binding-label="cardBindingLabel"
        :column-display-name="columnDisplayName"
        :has-unbound-template-for-table="hasUnboundTemplateForTable"
        :is-binding-card-selected="isBindingCardSelected"
        :is-binding-slot-count-locked="isBindingSlotCountLocked"
        :record-template-items="recordTemplateItems"
        :record-template-source-chip-label="recordTemplateSourceChipLabel"
        :record-template-source-color="recordTemplateSourceColor"
        :record-template-tooltip-text="recordTemplateTooltipText"
        :saving="saving"
        :selected-item="selectedItem"
        @assign-record-template="assignRecordTemplate"
        @clear-record-template="clearRecordTemplate"
        @close-binding-editor="closeBindingEditor"
        @save-binding-draft="saveBindingDraft"
        @add-binding-slot="addBindingSlot"
        @remove-binding-slot="removeBindingSlot"
        @move-binding-slot-up="
          (cardId, index) => moveBindingSlot(cardId, index, -1)
        "
        @move-binding-slot-down="
          (cardId, index) => moveBindingSlot(cardId, index, 1)
        "
        @select-binding-card="selectBindingCard"
        @set-binding-draft="setBindingDraft"
      />
      <ViewFreeLayoutStyleInspector
        v-else-if="editMode"
        :add-template-slot="addTemplateSlot"
        :apply-background-color-mode="applyBackgroundColorModeWithPolicy"
        :apply-auto-height-enabled-from-checkbox="
          applyAutoHeightEnabledFromCheckbox
        "
        :apply-font-weight-from-checkbox="applyFontWeightFromCheckbox"
        :apply-max-auto-height-behavior="applyMaxAutoHeightBehavior"
        :apply-max-auto-height-from-input="applyMaxAutoHeightFromInput"
        :apply-number-style-from-input="applyNumberStyleFromInput"
        :apply-preset-id="applyPresetId"
        :apply-push-down-siblings-from-checkbox="
          applyPushDownSiblingsFromCheckbox
        "
        :apply-selected-style="applySelectedStyle"
        :apply-selected-template-slot-display-format="
          applySelectedTemplateSlotDisplayFormat
        "
        :apply-selected-template-slot-font-size-from-input="
          applySelectedTemplateSlotFontSizeFromInput
        "
        :apply-selected-template-slot-font-weight="
          applySelectedTemplateSlotFontWeight
        "
        :apply-selected-template-slot-text-align="
          applySelectedTemplateSlotTextAlign
        "
        :apply-selected-template-slot-text-color-from-input="
          applySelectedTemplateSlotTextColorFromInput
        "
        :apply-show-label-from-checkbox="applyShowLabelFromCheckbox"
        :apply-style-from-input="applyStyleFromInputWithPolicy"
        :auto-height-enabled-value="autoHeightEnabledInspectorValue"
        :background-color-disabled="backgroundColorEditingDisabled"
        :background-color-disabled-reason="backgroundColorDisabledReason"
        :background-color-input-value="backgroundColorInputValue"
        :card-preset-items="cardPresetItems"
        :has-record-overrides="hasRecordOverrides"
        :is-template-mode="isTemplateMode"
        :is-transparent-background-selected="isTransparentBackgroundSelected"
        :max-auto-height-behavior-value="maxAutoHeightBehaviorInspectorValue"
        :max-auto-height-input-value="maxAutoHeightInspectorValue"
        :push-down-siblings-value="pushDownSiblingsInspectorValue"
        :remove-template-slot="removeTemplateSlot"
        :reorder-template-slots="reorderTemplateSlots"
        :reset-record-overrides="resetRecordOverrides"
        :reset-selected-card-override="resetSelectedCardOverride"
        :reset-selected-style="resetSelectedStyle"
        :selected-card-has-override="selectedCardHasOverride"
        :selected-layouts="selectedLayouts"
        :selected-preset-id="selectedPresetId"
        :selected-slot-item="selectedTemplateSlotItem"
        :selected-template-slot-display-format-value="
          selectedTemplateSlotDisplayFormat
        "
        :selected-template-slot-font-size-inherited="
          selectedTemplateSlotFontSizeInherited
        "
        :selected-template-slot-font-size-placeholder="
          selectedTemplateSlotFontSizePlaceholder
        "
        :selected-template-slot-font-size-value="selectedTemplateSlotFontSize"
        :selected-template-slot-inherited-preview="
          selectedTemplateSlotInheritedPreview
        "
        :selected-template-slot-font-weight-value="
          selectedTemplateSlotFontWeight
        "
        :selected-template-slot-font-weight-resolved-label="
          selectedTemplateSlotFontWeightResolvedLabel
        "
        :selected-template-slot-key="
          selectedTemplateSlotKey
            ? `${selectedTemplateSlotKey.cardId}:${selectedTemplateSlotKey.slotId}`
            : null
        "
        :selected-template-slot-text-align-value="selectedTemplateSlotTextAlign"
        :selected-template-slot-text-align-resolved-label="
          selectedTemplateSlotTextAlignResolvedLabel
        "
        :selected-template-slot-text-color-inherited="
          selectedTemplateSlotTextColorInherited
        "
        :selected-template-slot-text-color-value="selectedTemplateSlotTextColor"
        :reset-selected-template-slot-text-color="
          resetSelectedTemplateSlotTextColor
        "
        :select-template-slot="selectTemplateSlot"
        :style-boolean-input-value="styleBooleanInputValue"
        :style-input-value="styleInputValue"
        :style-inspector-values="styleInspectorValues"
        :style-number-input-value="styleNumberInputValue"
        :template-slot-items="templateSlotItems"
        :template-slot-target-card-label="templateSlotTargetCardLabel"
        :theme-color-input-value="themeColorInputValue"
      />
    </div>
  </section>
</template>
