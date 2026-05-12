<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";

import type {
  AppColumn,
  TableDetail,
  TemplatePreviewRecordSelection,
  ViewLayoutCardItem,
  ViewLayoutCardColumnBinding,
  ViewLayoutTemplateCard,
  ViewSelection,
  ViewTableSection
} from "../../../types";
import type { CSSProperties } from "vue";
type RectLike = {
  bottom: number;
  left: number;
  right: number;
  top: number;
};
type CanvasElement = {
  clientHeight: number;
  clientWidth: number;
  getBoundingClientRect: () => RectLike;
};
type PointerTarget = {
  setPointerCapture?: (pointerId: number) => void;
};
type PointerLikeEvent = {
  button?: number;
  clientX: number;
  clientY: number;
  ctrlKey?: boolean;
  currentTarget: object | null;
  pointerId: number;
  preventDefault: () => void;
  shiftKey: boolean;
  stopPropagation: () => void;
};
type KeyboardLikeEvent = {
  key: string;
  preventDefault?: () => void;
  repeat?: boolean;
};
type WheelLikeEvent = {
  clientX: number;
  clientY: number;
  ctrlKey: boolean;
  deltaY: number;
  preventDefault: () => void;
};
type InputLikeEvent = {
  target: {
    checked?: boolean;
    value: string;
  } | null;
};
type MoveInteraction = {
  cardId: number;
  cardIds: number[];
  moved: boolean;
  originLayouts: ViewLayoutCardItem[];
  originalSelection: number[];
  shiftKey: boolean;
  startX: number;
  startY: number;
  type: "move";
};
type ResizeDirection = "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw";
type ResizeInteraction = {
  cardId: number;
  direction: ResizeDirection;
  moved: boolean;
  origin: ViewLayoutCardItem;
  startX: number;
  startY: number;
  type: "resize";
};
type InteractionState = MoveInteraction | ResizeInteraction;
type SelectionBox = {
  height: number;
  width: number;
  x: number;
  y: number;
};
type SelectionDrag = {
  additive: boolean;
  initialSelection: number[];
  moved: boolean;
  startX: number;
  startY: number;
};
type PanDrag = {
  startOffsetX: number;
  startOffsetY: number;
  startX: number;
  startY: number;
};
type LayoutStyleKey =
  | "backgroundColor"
  | "textColor"
  | "fontSize"
  | "textDirection"
  | "fontWeight"
  | "textAlign"
  | "padding"
  | "paddingTop"
  | "paddingRight"
  | "paddingBottom"
  | "paddingLeft"
  | "borderRadius"
  | "showLabel";

type LayoutStyleValue = boolean | string | number | null;
const MIN_CARD_WIDTH = 120;
const MIN_CARD_HEIGHT = 64;
const DRAG_THRESHOLD = 4;
const WORLD_WIDTH = 2400;
const WORLD_HEIGHT = 1600;
const MIN_VIEWPORT_SCALE = 0.25;
const MAX_VIEWPORT_SCALE = 3;
const VIEWPORT_ZOOM_STEP = 0.15;
const RESIZE_HANDLES: Array<{
  direction: ResizeDirection;
  label: string;
}> = [
  { direction: "n", label: "上辺でサイズ変更" },
  { direction: "e", label: "右辺でサイズ変更" },
  { direction: "s", label: "下辺でサイズ変更" },
  { direction: "w", label: "左辺でサイズ変更" },
  { direction: "ne", label: "右上でサイズ変更" },
  { direction: "se", label: "右下でサイズ変更" },
  { direction: "sw", label: "左下でサイズ変更" },
  { direction: "nw", label: "左上でサイズ変更" }
];
const DEFAULT_CARD_STYLE = {
  backgroundColor: null,
  borderRadius: 14,
  fontSize: 16,
  fontWeight: "bold",
  padding: 12,
  paddingTop: 12,
  paddingRight: 12,
  paddingBottom: 12,
  paddingLeft: 12,
  showLabel: true,
  textAlign: "left",
  textColor: null,
  textDirection: "horizontal"
} as const;

const props = withDefaults(
  defineProps<{
    detail: TableDetail | null;
    editorMode?: "record" | "template";
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
    editorMode: "record",
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
  "clear-template-preview": [];
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
const bindingDraft = ref<Record<number, number | null>>({});
const selectedBindingCardId = ref<number | null>(null);
const templatePreviewMenuOpen = ref(false);
const templatePreviewBindingDraft = ref<Record<number, number | null>>({});
const isTemplatePreviewBindingsOpen = ref(false);
const hasManuallyToggledTemplatePreviewBindings = ref(false);
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
  return (
    item.columnId !== null &&
    displayColumns.value.some((column) => column.id === item.columnId)
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
const selectedLayouts = computed(() =>
  visibleLayouts.value.filter((layout) => isSelected(layout.cardId))
);
const bindableTemplateLayouts = computed(() =>
  isTemplateMode.value ? [] : draftLayouts.value.filter((item) => item.visible)
);
const hasUnboundTemplateForTable = computed(
  () =>
    !isTemplateMode.value &&
    bindableTemplateLayouts.value.length > 0 &&
    displayColumns.value.length > 0 &&
    bindableTemplateLayouts.value.every(
      (layout) =>
        layout.columnId === null ||
        !displayColumns.value.some((column) => column.id === layout.columnId)
    )
);
const isBindingEditorVisible = computed(
  () => hasUnboundTemplateForTable.value || isBindingEditorOpen.value
);
const shouldRenderCardContent = computed(
  () => editMode.value || isTemplatePreviewActive.value
);
const canvasLayouts = computed(() =>
  isBindingEditorVisible.value
    ? bindableTemplateLayouts.value
    : visibleLayouts.value
);
const bindingColumnItems = computed(() =>
  displayColumns.value.map((column) => ({
    title: column.displayName,
    value: column.id
  }))
);
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
const templatePreviewLabel = computed(() => {
  const selected = props.templatePreviewSelectedItem;
  if (selected?.type !== "tableRecord") {
    return "";
  }
  return `${selected.tableDisplayName} / ${selected.recordLabel}`;
});
const bindingDraftValues = computed(() =>
  Object.values(bindingDraft.value).filter(
    (columnId): columnId is number => columnId !== null
  )
);
const templatePreviewBindingValues = computed(() =>
  Object.values(templatePreviewBindingDraft.value).filter(
    (columnId): columnId is number => columnId !== null
  )
);
const templatePreviewBoundCount = computed(
  () => templatePreviewBindingValues.value.length
);
const templatePreviewUnboundCount = computed(
  () => props.templateCards.length - templatePreviewBoundCount.value
);
const hasDuplicateBindingColumns = computed(() => {
  const values = bindingDraftValues.value;
  return new Set(values).size !== values.length;
});
const canSaveBindingDraft = computed(
  () =>
    bindingDraftValues.value.length > 0 &&
    !hasDuplicateBindingColumns.value &&
    !props.saving
);

const selectedCardHasOverride = computed(
  () =>
    selectedLayouts.value.length === 1 && selectedLayouts.value[0].hasOverride
);

const styleInspectorValues = computed(() => ({
  backgroundColor: sharedStyleValue("backgroundColor"),
  borderRadius: sharedStyleValue("borderRadius"),
  fontSize: sharedStyleValue("fontSize"),
  fontWeight: sharedStyleValue("fontWeight"),
  padding: sharedStyleValue("padding"),
  paddingTop: sharedStyleValue("paddingTop"),
  paddingRight: sharedStyleValue("paddingRight"),
  paddingBottom: sharedStyleValue("paddingBottom"),
  paddingLeft: sharedStyleValue("paddingLeft"),
  textAlign: sharedStyleValue("textAlign"),
  textColor: sharedStyleValue("textColor"),
  textDirection: sharedStyleValue("textDirection"),
  showLabel: sharedStyleValue("showLabel")
}));
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
  () =>
    [
      props.templatePreviewBindings,
      props.templateCards,
      props.templatePreviewSelectedItem
    ] as const,
  () => {
    resetTemplatePreviewBindingDraft();
  },
  { immediate: true }
);
watch(
  () => props.templatePreviewSelectedItem,
  (selected, previous) => {
    if (selected === null) {
      isTemplatePreviewBindingsOpen.value = false;
      hasManuallyToggledTemplatePreviewBindings.value = false;
      return;
    }

    if (
      selected !== previous &&
      !hasManuallyToggledTemplatePreviewBindings.value
    ) {
      isTemplatePreviewBindingsOpen.value =
        templatePreviewUnboundCount.value > 0;
    }
  }
);
watch(templatePreviewUnboundCount, (count) => {
  if (
    isTemplatePreviewActive.value &&
    !hasManuallyToggledTemplatePreviewBindings.value
  ) {
    isTemplatePreviewBindingsOpen.value = count > 0;
  }
});
watch(
  () => props.selectedItem,
  () => {
    clearSelection();
    closeBindingEditor();
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
    resetViewport();
  }
});
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
    fitViewportToContent();
    return;
  }

  selectedBindingCardId.value = null;
});
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

function effectiveColumnId(layout: ViewLayoutCardItem) {
  if (isTemplatePreviewActive.value) {
    return templatePreviewBindingDraft.value[layout.cardId] ?? null;
  }

  return layout.columnId;
}

function fieldValue(column: AppColumn) {
  return currentRecord.value?.displayValues[column.columnName] || "未入力";
}

function fieldLabel(columnId: number) {
  return columnById(columnId)?.displayName ?? "";
}

function fieldLabelByLayout(layout: ViewLayoutCardItem) {
  if (isTemplateMode.value) {
    const columnId = effectiveColumnId(layout);
    if (isTemplatePreviewActive.value && columnId !== null) {
      return fieldLabel(columnId);
    }
    return layout.cardId > 0 ? `カード #${layout.cardId}` : "新規カード";
  }
  return layout.columnId === null ? "未紐付け" : fieldLabel(layout.columnId);
}

function removeButtonLabel(layout: ViewLayoutCardItem) {
  if (isTemplateMode.value) {
    return `${fieldLabelByLayout(layout)} を削除`;
  }

  return layout.visible
    ? `${fieldLabelByLayout(layout)}を非表示にする`
    : `${fieldLabelByLayout(layout)}を表示に戻す`;
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

function fieldValueByLayout(layout: ViewLayoutCardItem) {
  if (isTemplateMode.value) {
    if (!isTemplatePreviewActive.value) {
      return "カード枠";
    }
    const columnId = effectiveColumnId(layout);
    return columnId === null
      ? "一時紐付けなし"
      : fieldValueByColumnId(columnId);
  }
  return layout.columnId === null ? "" : fieldValueByColumnId(layout.columnId);
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

function setBindingDraft(cardId: number, columnId: number | null) {
  selectBindingCard(cardId);
  bindingDraft.value = {
    ...bindingDraft.value,
    [cardId]: columnId
  };
}

function resetBindingDraft() {
  bindingDraft.value = Object.fromEntries(
    draftLayouts.value.map((layout) => [layout.cardId, layout.columnId])
  );
}

function resetTemplatePreviewBindingDraft() {
  templatePreviewBindingDraft.value = Object.fromEntries(
    props.templateCards.map((card) => [
      card.cardId,
      props.templatePreviewBindings.find(
        (binding) => binding.cardId === card.cardId
      )?.columnId ?? null
    ])
  );
}

function setTemplatePreviewBinding(cardId: number, columnId: unknown) {
  const nextColumnId = typeof columnId === "number" ? columnId : null;
  templatePreviewBindingDraft.value = {
    ...templatePreviewBindingDraft.value,
    [cardId]: nextColumnId
  };
}

function toggleTemplatePreviewBindings() {
  hasManuallyToggledTemplatePreviewBindings.value = true;
  isTemplatePreviewBindingsOpen.value = !isTemplatePreviewBindingsOpen.value;
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

function openBindingEditor() {
  resetBindingDraft();
  isBindingEditorOpen.value = true;
}

function closeBindingEditor() {
  isBindingEditorOpen.value = false;
  selectedBindingCardId.value = null;
}

function selectBindingCard(cardId: number) {
  selectedBindingCardId.value = cardId;
}

function isBindingCardSelected(cardId: number) {
  return selectedBindingCardId.value === cardId;
}

function saveBindingDraft() {
  if (!canSaveBindingDraft.value) {
    return;
  }

  emit(
    "save-card-column-bindings",
    Object.entries(bindingDraft.value)
      .map(([cardId, columnId]) => ({
        cardId: Number(cardId),
        columnId
      }))
      .filter(
        (binding): binding is ViewLayoutCardColumnBinding =>
          Number.isFinite(binding.cardId) && binding.columnId !== null
      )
  );
  if (!hasUnboundTemplateForTable.value) {
    closeBindingEditor();
  }
}

function layoutStyleValue(
  layout: ViewLayoutCardItem,
  key: LayoutStyleKey
): LayoutStyleValue {
  if (isPaddingSideKey(key)) {
    return layout[key] ?? layout.padding ?? DEFAULT_CARD_STYLE[key];
  }

  return layout[key] ?? DEFAULT_CARD_STYLE[key];
}

function isPaddingSideKey(
  key: LayoutStyleKey
): key is "paddingTop" | "paddingRight" | "paddingBottom" | "paddingLeft" {
  return (
    key === "paddingTop" ||
    key === "paddingRight" ||
    key === "paddingBottom" ||
    key === "paddingLeft"
  );
}

function sharedStyleValue(key: LayoutStyleKey): LayoutStyleValue | "" {
  const [firstLayout, ...restLayouts] = selectedLayouts.value;
  if (!firstLayout) {
    return "";
  }

  const firstValue = layoutStyleValue(firstLayout, key);
  const hasMixedValues = restLayouts.some(
    (layout) => layoutStyleValue(layout, key) !== firstValue
  );
  return hasMixedValues ? "" : firstValue;
}

function cardStyle(layout: ViewLayoutCardItem): CSSProperties {
  const backgroundColor = layoutStyleValue(layout, "backgroundColor");
  const textColor = layoutStyleValue(layout, "textColor");
  return {
    ...(backgroundColor ? { backgroundColor: String(backgroundColor) } : {}),
    borderRadius: `${layoutStyleValue(layout, "borderRadius")}px`,
    ...(textColor ? { color: String(textColor) } : {}),
    fontSize: `${layoutStyleValue(layout, "fontSize")}px`,
    fontWeight: String(layoutStyleValue(layout, "fontWeight")),
    height: `${layout.height}px`,
    paddingBottom: `${layoutStyleValue(layout, "paddingBottom")}px`,
    paddingLeft: `${layoutStyleValue(layout, "paddingLeft")}px`,
    paddingRight: `${layoutStyleValue(layout, "paddingRight")}px`,
    paddingTop: `${layoutStyleValue(layout, "paddingTop")}px`,
    textAlign: layoutStyleValue(
      layout,
      "textAlign"
    ) as CSSProperties["textAlign"],
    transform: `translate(${layout.x}px, ${layout.y}px)`,
    width: `${layout.width}px`
  };
}

function cardContentStyle(layout: ViewLayoutCardItem): CSSProperties {
  return {
    writingMode:
      layoutStyleValue(layout, "textDirection") === "vertical"
        ? "vertical-rl"
        : "horizontal-tb"
  };
}

function styleInputValue(key: LayoutStyleKey) {
  const value = styleInspectorValues.value[key];
  return value == null || value === "" ? "" : String(value);
}

function backgroundColorInputValue() {
  const value = styleInputValue("backgroundColor");
  return value && value !== "transparent"
    ? value
    : themeColorInputValue("--v-theme-surface");
}

function themeColorInputValue(tokenName: string) {
  const documentElement = globalThis.document?.documentElement;
  if (!documentElement) {
    return "";
  }

  const tokenValue = globalThis
    .getComputedStyle(documentElement)
    .getPropertyValue(tokenName)
    .trim();
  const colorChannels = tokenValue
    .split(/[,\s]+/)
    .map((part) => Number(part))
    .filter((part) => Number.isFinite(part))
    .slice(0, 3);

  if (colorChannels.length !== 3) {
    return "";
  }

  return colorChannels
    .map((part) =>
      Math.max(0, Math.min(255, part)).toString(16).padStart(2, "0")
    )
    .join("")
    .replace(/^/, "#");
}

function isTransparentBackgroundSelected() {
  return styleInspectorValues.value.backgroundColor === "transparent";
}

function hasTransparentBackground(layout: ViewLayoutCardItem) {
  return layoutStyleValue(layout, "backgroundColor") === "transparent";
}

function applyBackgroundColorMode(mode: "color" | "transparent") {
  applySelectedStyle(
    "backgroundColor",
    mode === "transparent" ? "transparent" : backgroundColorInputValue() || null
  );
}

function styleNumberInputValue(key: LayoutStyleKey) {
  const value = styleInspectorValues.value[key];
  return typeof value === "number" ? value : "";
}

function styleBooleanInputValue(key: LayoutStyleKey) {
  const value = styleInspectorValues.value[key];
  if (key === "showLabel") {
    return value === "" ? true : value !== false;
  }
  return value === "bold";
}

function applySelectedStyle(key: LayoutStyleKey, value: LayoutStyleValue) {
  if (selectedLayouts.value.length === 0) {
    return;
  }

  updateLayouts(
    selectedLayouts.value.map((layout) => ({
      ...layout,
      [key]: value
    })),
    true
  );
}

function inputTarget(event: unknown) {
  return (event as InputLikeEvent).target;
}

function applyStyleFromInput(key: LayoutStyleKey, event: unknown) {
  const value = inputTarget(event)?.value ?? "";
  if (!value) {
    return;
  }

  applySelectedStyle(key, value);
}

function applyNumberStyleFromInput(key: LayoutStyleKey, event: unknown) {
  const value = Number(inputTarget(event)?.value);
  if (!Number.isFinite(value)) {
    return;
  }

  applySelectedStyle(key, value);
}

function _applyFontWeightFromInput(event: unknown) {
  applySelectedStyle(
    "fontWeight",
    inputTarget(event)?.checked ? "bold" : "normal"
  );
}

function applyFontWeightFromCheckbox(value: boolean | null) {
  applySelectedStyle("fontWeight", value ? "bold" : "normal");
}

function _applyShowLabelFromInput(event: unknown) {
  applySelectedStyle("showLabel", inputTarget(event)?.checked ?? true);
}

function applyShowLabelFromCheckbox(value: boolean | null) {
  applySelectedStyle("showLabel", value ?? true);
}

function resetSelectedStyle() {
  if (selectedLayouts.value.length === 0) {
    return;
  }

  updateLayouts(
    selectedLayouts.value.map((layout) => ({
      ...layout,
      backgroundColor: null,
      borderRadius: null,
      fontSize: null,
      fontWeight: null,
      padding: null,
      paddingTop: null,
      paddingRight: null,
      paddingBottom: null,
      paddingLeft: null,
      textAlign: null,
      textColor: null,
      textDirection: null,
      showLabel: null
    })),
    true
  );
}

function shouldShowLabel(layout: ViewLayoutCardItem) {
  return layoutStyleValue(layout, "showLabel") !== false;
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

function clampViewportScale(scale: number) {
  return Math.max(MIN_VIEWPORT_SCALE, Math.min(MAX_VIEWPORT_SCALE, scale));
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
  const validIds = new Set(visibleLayouts.value.map((layout) => layout.cardId));
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

function boxFromPoints(
  startX: number,
  startY: number,
  endX: number,
  endY: number
) {
  const x = Math.min(startX, endX);
  const y = Math.min(startY, endY);
  return {
    height: Math.abs(endY - startY),
    width: Math.abs(endX - startX),
    x,
    y
  };
}

function intersects(a: SelectionBox, b: SelectionBox) {
  return (
    a.x < b.x + b.width &&
    a.x + a.width > b.x &&
    a.y < b.y + b.height &&
    a.y + a.height > b.y
  );
}

function idsInSelectionBox(box: SelectionBox) {
  return visibleLayouts.value
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
    columnId: null,
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
    columnId: null,
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
    hasOverride: false
  };
}

function layoutToTemplateCard(
  layout: ViewLayoutCardItem
): ViewLayoutTemplateCard {
  return {
    cardId: layout.cardId,
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
    showLabel: layout.showLabel
  };
}
</script>

<template>
  <section ref="panelRef" class="view-free-layout-panel">
    <div class="section-header">
      <div>
        <template v-if="isTemplateMode">
          <h2>テンプレート編集</h2>
          <p class="help-text">
            {{ templateName }} のカード枠と見た目を編集します。
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
                @click="zoomFromCenter(-VIEWPORT_ZOOM_STEP)"
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
                @click="resetViewport"
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
                @click="zoomFromCenter(VIEWPORT_ZOOM_STEP)"
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
                @click="fitViewportToContent"
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
          v-model="templatePreviewMenuOpen"
          :close-on-content-click="false"
          location="bottom end"
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
                @update:model-value="selectTemplatePreviewRecord"
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
          @click:close="clearTemplatePreview"
        >
          プレビュー中: {{ templatePreviewLabel }}
        </v-chip>
        <v-btn
          v-if="
            !isTemplateMode &&
            bindableTemplateLayouts.length > 0 &&
            displayColumns.length > 0
          "
          prepend-icon="mdi-card-bulleted-settings-outline"
          color="primary"
          variant="tonal"
          size="small"
          :disabled="hasUnboundTemplateForTable"
          @click="openBindingEditor"
        >
          表示項目
        </v-btn>
        <v-btn
          v-if="isTemplateMode"
          prepend-icon="mdi-plus"
          color="primary"
          variant="tonal"
          size="small"
          @click="addTemplateCard"
        >
          カード追加
        </v-btn>
        <v-switch
          v-model="editMode"
          :disabled="isBindingEditorVisible"
          hide-details
          density="compact"
          color="primary"
          label="編集"
        />
      </div>
    </div>
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
            :style="cardStyle(layout)"
            @pointerdown.stop="
              isBindingEditorVisible
                ? selectBindingCard(layout.cardId)
                : startCardPointer($event, layout.cardId, layout)
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
                <p class="view-field-card-value">
                  {{ columnDisplayName(bindingDraft[layout.cardId] ?? null) }}
                </p>
              </div>
            </template>
            <template
              v-else-if="
                shouldRenderCardContent ||
                (!isTemplateMode && layout.columnId !== null)
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
                  v-if="shouldShowLabel(layout)"
                  class="view-field-card-header"
                >
                  <span>{{ fieldLabelByLayout(layout) }}</span>
                </div>
                <p class="view-field-card-value">
                  {{ fieldValueByLayout(layout) }}
                </p>
              </div>
            </template>
            <template v-if="editMode && isSelected(layout.cardId)">
              <v-tooltip
                v-for="handle in RESIZE_HANDLES"
                :key="handle.direction"
                :text="`${fieldLabelByLayout(layout)}を${handle.label}`"
                location="bottom"
              >
                <template #activator="{ props: tooltipProps }">
                  <button
                    v-bind="tooltipProps"
                    type="button"
                    class="view-field-resize-handle"
                    :class="`view-field-resize-handle-${handle.direction}`"
                    :aria-label="`${fieldLabelByLayout(layout)}を${handle.label}`"
                    @pointerdown.stop="
                      startResize(
                        $event,
                        layout.cardId,
                        layout,
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
      <aside
        v-if="isBindingEditorVisible"
        class="view-binding-panel"
        @pointerdown.stop
        @click.stop
      >
        <div class="view-binding-setup-copy">
          <strong>
            {{
              hasUnboundTemplateForTable
                ? "このテーブルの表示項目を紐付けてください"
                : "このテーブルの表示項目を編集"
            }}
          </strong>
          <p>
            カードを選ぶと、キャンバス上の対応する位置も強調されます。未使用にしたいカードは未選択のまま保存できます。
          </p>
        </div>
        <div class="view-binding-list">
          <v-card
            v-for="(layout, index) in bindableTemplateLayouts"
            :key="layout.cardId"
            :class="{ selected: isBindingCardSelected(layout.cardId) }"
            variant="outlined"
            rounded="lg"
            class="view-binding-card"
            role="button"
            tabindex="0"
            @click="selectBindingCard(layout.cardId)"
            @focusin="selectBindingCard(layout.cardId)"
          >
            <div>
              <strong>{{ cardBindingLabel(layout, index) }}</strong>
              <span>
                {{ columnDisplayName(bindingDraft[layout.cardId] ?? null) }}
              </span>
            </div>
            <v-select
              :items="bindingColumnItems"
              :model-value="bindingDraft[layout.cardId] ?? null"
              clearable
              density="compact"
              hide-details
              item-title="title"
              item-value="value"
              label="表示カラム"
              variant="outlined"
              @update:model-value="setBindingDraft(layout.cardId, $event)"
            />
          </v-card>
        </div>
        <v-alert
          v-if="hasDuplicateBindingColumns"
          type="warning"
          variant="tonal"
          density="compact"
        >
          同じカラムを複数のカードへ紐付けることはできません。
        </v-alert>
        <div class="view-binding-actions">
          <v-btn
            v-if="!hasUnboundTemplateForTable"
            variant="text"
            :disabled="saving"
            @click="closeBindingEditor"
          >
            閉じる
          </v-btn>
          <v-btn
            color="primary"
            variant="flat"
            :disabled="!canSaveBindingDraft"
            :loading="saving"
            @click="saveBindingDraft"
          >
            紐付けを保存
          </v-btn>
        </div>
      </aside>
      <aside
        v-else-if="editMode"
        class="view-style-inspector"
        @pointerdown.stop
        @click.stop
      >
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
                紐付け済み・保存されません
              </span>
            </div>
            <v-tooltip
              :text="
                isTemplatePreviewBindingsOpen
                  ? '一時紐付けを畳む'
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
                      ? '一時紐付けを畳む'
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
              プレビュー中だけ、カードに表示するカラムを選べます。
            </p>
            <div class="view-template-preview-binding-list">
              <label
                v-for="(layout, index) in draftLayouts"
                :key="layout.cardId"
                class="view-template-preview-binding"
              >
                <span>{{ cardBindingLabel(layout, index) }}</span>
                <v-select
                  :items="bindingColumnItems"
                  :model-value="
                    templatePreviewBindingDraft[layout.cardId] ?? null
                  "
                  item-title="title"
                  item-value="value"
                  label="表示カラム"
                  variant="outlined"
                  density="compact"
                  clearable
                  hide-details
                  @update:model-value="
                    setTemplatePreviewBinding(layout.cardId, $event)
                  "
                />
              </label>
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
          <label class="view-style-control view-background-control">
            <span>背景色</span>
            <div class="view-background-row">
              <input
                type="color"
                :class="{ muted: isTransparentBackgroundSelected() }"
                :value="backgroundColorInputValue()"
                @input="applyStyleFromInput('backgroundColor', $event)"
              />
              <button
                type="button"
                class="view-background-transparent-button"
                :class="{ active: isTransparentBackgroundSelected() }"
                @click="applyBackgroundColorMode('transparent')"
              >
                透明
              </button>
            </div>
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
    </div>
  </section>
</template>
