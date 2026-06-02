<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";

import { useConfirmDialog } from "../../../composables/useConfirmDialog";

import { isKnownCardPresetId, listCardPresets } from "./cardPresets/registry";
import { useFreeLayoutStyleEditing } from "./useFreeLayoutStyleEditing";
import ViewFreeLayoutBindingPanel from "./ViewFreeLayoutBindingPanel.vue";
import {
  boxFromPoints,
  clampViewportScale,
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
  ViewLayoutCardItem,
  ViewLayoutCardColumnBinding,
  ViewLayoutTemplate,
  ViewLayoutTemplateCard,
  ViewSelection,
  ViewTableSection
} from "../../../types";
import type { CSSProperties } from "vue";
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
  () => selectedRecordPanelKey.value,
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
    // 注意: 拡大率の自動変更はユーザーの意図しない動作となるため行わない
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
    (layout) => bindingDraft.value[layout.cardId] !== layout.columnId
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

function shouldShowLabel(layout: ViewLayoutCardItem) {
  return layoutStyleValue(layout, "showLabel") !== false;
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
    hasOverride: false
  };
}

function layoutToTemplateCard(
  layout: ViewLayoutCardItem
): ViewLayoutTemplateCard {
  return {
    cardId: layout.cardId,
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
    showLabel: layout.showLabel
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
      <ViewFreeLayoutBindingPanel
        v-if="isBindingEditorVisible"
        :bindable-template-layouts="bindableTemplateLayouts"
        :binding-column-items="bindingColumnItems"
        :binding-draft="bindingDraft"
        :can-save-binding-draft="canSaveBindingDraft"
        :card-binding-label="cardBindingLabel"
        :column-display-name="columnDisplayName"
        :has-duplicate-binding-columns="hasDuplicateBindingColumns"
        :has-unbound-template-for-table="hasUnboundTemplateForTable"
        :is-binding-card-selected="isBindingCardSelected"
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
        @select-binding-card="selectBindingCard"
        @set-binding-draft="setBindingDraft"
      />
      <ViewFreeLayoutStyleInspector
        v-else-if="editMode"
        :apply-background-color-mode="applyBackgroundColorMode"
        :apply-font-weight-from-checkbox="applyFontWeightFromCheckbox"
        :apply-number-style-from-input="applyNumberStyleFromInput"
        :apply-preset-id="applyPresetId"
        :apply-selected-style="applySelectedStyle"
        :apply-show-label-from-checkbox="applyShowLabelFromCheckbox"
        :apply-style-from-input="applyStyleFromInput"
        :background-color-input-value="backgroundColorInputValue"
        :binding-column-items="bindingColumnItems"
        :card-binding-label="cardBindingLabel"
        :card-preset-items="cardPresetItems"
        :draft-layouts="draftLayouts"
        :has-record-overrides="hasRecordOverrides"
        :is-template-mode="isTemplateMode"
        :is-template-preview-active="isTemplatePreviewActive"
        :is-template-preview-bindings-open="isTemplatePreviewBindingsOpen"
        :is-transparent-background-selected="isTransparentBackgroundSelected"
        :reset-record-overrides="resetRecordOverrides"
        :reset-selected-card-override="resetSelectedCardOverride"
        :reset-selected-style="resetSelectedStyle"
        :selected-card-has-override="selectedCardHasOverride"
        :selected-layouts="selectedLayouts"
        :selected-preset-id="selectedPresetId"
        :set-template-preview-binding="setTemplatePreviewBinding"
        :style-boolean-input-value="styleBooleanInputValue"
        :style-input-value="styleInputValue"
        :style-inspector-values="styleInspectorValues"
        :style-number-input-value="styleNumberInputValue"
        :template-cards="templateCards"
        :template-preview-binding-draft="templatePreviewBindingDraft"
        :template-preview-bound-count="templatePreviewBoundCount"
        :theme-color-input-value="themeColorInputValue"
        :toggle-template-preview-bindings="toggleTemplatePreviewBindings"
      />
    </div>
  </section>
</template>
