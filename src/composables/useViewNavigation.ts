import { computed, onMounted, ref } from "vue";

import { api } from "../api";

import type {
  ViewNavNode,
  ViewNavFolderRecord,
  ViewNavTreeNode,
  ViewSelection,
  TableDetail,
  ViewLayoutCardItem,
  ViewLayoutCardColumnBinding,
  ViewLayoutTemplate,
  ViewLayoutTemplateCard,
  ViewTableRecordSummary,
  ViewTableSection,
  TemplatePreviewRecordSelection,
  SaveViewLayoutCardOverridesPayload
} from "../types";

export function useViewNavigation() {
  const tableSections = ref<ViewTableSection[]>([]);

  const customNodes = ref<ViewNavNode[]>([]);

  const folderRecords = ref<ViewNavFolderRecord[]>([]);

  const expandedFolderIds = ref<number[]>([]);

  const selectedItem = ref<ViewSelection | null>(null);

  const selectedTableDetail = ref<TableDetail | null>(null);

  const layoutCardItems = ref<ViewLayoutCardItem[]>([]);
  const activeLayoutTemplateId = ref<number | null>(null);
  const layoutTemplates = ref<ViewLayoutTemplate[]>([]);
  const selectedLayoutTemplate = ref<ViewLayoutTemplate | null>(null);
  const templateLayoutCards = ref<ViewLayoutTemplateCard[]>([]);
  const templatePreviewSelectedItem = ref<ViewSelection | null>(null);
  const templatePreviewTableDetail = ref<TableDetail | null>(null);
  const templatePreviewBindings = ref<ViewLayoutCardColumnBinding[]>([]);
  const templatePreviewLoading = ref(false);
  const selectedFolderLayoutTemplates = ref<ViewLayoutTemplate[]>([]);
  const selectedFolderActiveTemplateId = ref<number | null>(null);

  const loading = ref(false);

  const layoutSaving = ref(false);

  const error = ref("");
  let layoutSaveTimer: ReturnType<typeof globalThis.setTimeout> | null = null;

  const customTree = computed(() =>
    buildViewNavTree(customNodes.value, folderRecords.value, null)
  );

  const folderCount = computed(() => customNodes.value.length);

  const selectedFolderRecords = computed(() => {
    const selected = selectedItem.value;
    if (selected?.type !== "folder") {
      return [];
    }

    return folderRecords.value.filter(
      (record) => record.folderId === selected.folderId
    );
  });

  function clearError() {
    error.value = "";
  }

  async function initialize() {
    loading.value = true;
    error.value = "";

    try {
      const [sections, nodes, records, templates] = await Promise.all([
        api.getViewTableSections(),
        api.listViewNavNodes(),
        api.listViewNavFolderRecords(),
        api.listAllFolderLayoutTemplates()
      ]);

      tableSections.value = sections;
      customNodes.value = nodes;
      folderRecords.value = records;
      layoutTemplates.value = templates;
      expandedFolderIds.value = nodes.map((node) => node.id);
    } catch (loadError) {
      error.value =
        loadError instanceof Error ? loadError.message : String(loadError);
    } finally {
      loading.value = false;
    }
  }

  function toggleFolder(folderId: number) {
    expandedFolderIds.value = toggleExpandedId(
      expandedFolderIds.value,
      folderId
    );
  }

  function selectFolder(node: ViewNavNode) {
    selectedLayoutTemplate.value = null;
    templateLayoutCards.value = [];
    clearTemplatePreview();
    selectedItem.value = {
      type: "folder",
      folderId: node.id,
      folderName: node.name
    };
    selectedTableDetail.value = null;
    layoutCardItems.value = [];
    activeLayoutTemplateId.value = null;
    void loadFolderLayoutTemplates(node.id);
  }

  async function loadFolderLayoutTemplates(folderId: number) {
    try {
      const resolved = await api.listViewLayoutTemplatesForFolder({
        folderId
      });
      selectedFolderLayoutTemplates.value = resolved.templates;
      selectedFolderActiveTemplateId.value = resolved.activeTemplateId;
      mergeLayoutTemplates(resolved.templates);
    } catch (loadError) {
      error.value =
        loadError instanceof Error ? loadError.message : String(loadError);
    }
  }

  async function loadSelectedTableRecord(
    tableId: number,
    recordId: number,
    folderId: number | null
  ) {
    error.value = "";
    loading.value = true;

    try {
      const [detail, resolvedLayout] = await Promise.all([
        api.getTableDetail(tableId),
        api.getResolvedViewFieldLayout({
          tableId,
          recordId,
          folderId
        })
      ]);
      selectedTableDetail.value = detail;
      layoutCardItems.value = resolvedLayout.items;
      activeLayoutTemplateId.value = resolvedLayout.activeTemplateId;
    } catch (loadError) {
      error.value =
        loadError instanceof Error ? loadError.message : String(loadError);
    } finally {
      loading.value = false;
    }
  }

  function saveRecordLayoutOverrides(items: ViewLayoutCardItem[]) {
    const selected = selectedItem.value;
    const templateId = activeLayoutTemplateId.value;
    if (selected?.type !== "tableRecord" || !templateId) {
      return;
    }

    layoutCardItems.value = items;
    layoutSaving.value = true;

    if (layoutSaveTimer) {
      globalThis.clearTimeout(layoutSaveTimer);
    }

    layoutSaveTimer = globalThis.setTimeout(() => {
      void persistRecordLayoutOverrides(
        templateId,
        selected.tableId,
        selected.recordId,
        items
      );
    }, 240);
  }

  async function persistRecordLayoutOverrides(
    templateId: number,
    tableId: number,
    recordId: number,
    items: ViewLayoutCardItem[]
  ) {
    try {
      await api.saveViewLayoutCardOverrides({
        templateId,
        tableId,
        recordId,
        items: toLayoutPayloadItems(items)
      });
      await reloadSelectedLayout();
    } catch (saveError) {
      error.value =
        saveError instanceof Error ? saveError.message : String(saveError);
    } finally {
      layoutSaving.value = false;
    }
  }

  async function createFolderLayoutTemplate(
    folderId: number | null,
    name: string
  ) {
    error.value = "";
    const template = await api.createViewLayoutTemplate({
      name,
      scopeType: "folder",
      folderId
    });
    mergeLayoutTemplates([template]);
    await selectLayoutTemplate(template);
    if (folderId !== null) {
      await assignFolderLayoutTemplate(folderId, template.id);
    }
  }

  async function assignFolderLayoutTemplate(
    folderId: number,
    templateId: number
  ) {
    error.value = "";
    await api.assignViewLayoutFolderTemplate({
      folderId,
      templateId
    });
    await loadFolderLayoutTemplates(folderId);
  }

  async function selectLayoutTemplate(template: ViewLayoutTemplate) {
    error.value = "";
    loading.value = true;
    clearTemplatePreview();
    selectedItem.value = null;
    selectedTableDetail.value = null;
    layoutCardItems.value = [];
    activeLayoutTemplateId.value = null;
    selectedFolderLayoutTemplates.value = [];
    selectedFolderActiveTemplateId.value = null;
    selectedLayoutTemplate.value = template;

    try {
      templateLayoutCards.value = await api.getViewLayoutTemplateCards({
        templateId: template.id
      });
    } catch (loadError) {
      error.value =
        loadError instanceof Error ? loadError.message : String(loadError);
    } finally {
      loading.value = false;
    }
  }

  async function createLayoutTemplate(name: string) {
    error.value = "";
    const template = await api.createViewLayoutTemplate({
      name,
      scopeType: "folder",
      folderId: null
    });
    mergeLayoutTemplates([template]);
    await selectLayoutTemplate(template);
  }

  async function renameLayoutTemplate(templateId: number, name: string) {
    error.value = "";
    const template = await api.renameViewLayoutTemplate({ templateId, name });
    mergeLayoutTemplates([template]);
    if (selectedLayoutTemplate.value?.id === templateId) {
      selectedLayoutTemplate.value = template;
    }
  }

  async function duplicateLayoutTemplate(templateId: number, name: string) {
    error.value = "";
    const template = await api.duplicateViewLayoutTemplate({
      templateId,
      name
    });
    mergeLayoutTemplates([template]);
    await selectLayoutTemplate(template);
  }

  async function deleteLayoutTemplate(templateId: number) {
    error.value = "";
    await api.deleteViewLayoutTemplate({ templateId });
    layoutTemplates.value = layoutTemplates.value.filter(
      (template) => template.id !== templateId
    );
    selectedFolderLayoutTemplates.value =
      selectedFolderLayoutTemplates.value.filter(
        (template) => template.id !== templateId
      );
    if (selectedFolderActiveTemplateId.value === templateId) {
      selectedFolderActiveTemplateId.value = null;
    }
    if (selectedLayoutTemplate.value?.id === templateId) {
      selectedLayoutTemplate.value = null;
      templateLayoutCards.value = [];
      clearTemplatePreview();
    }
  }

  async function selectTemplatePreviewRecord(
    record: TemplatePreviewRecordSelection
  ) {
    const template = selectedLayoutTemplate.value;
    if (!template) {
      return;
    }

    error.value = "";
    templatePreviewLoading.value = true;
    try {
      const [detail, bindings] = await Promise.all([
        api.getTableDetail(record.tableId),
        api.listViewLayoutCardColumnBindings({
          templateId: template.id,
          tableId: record.tableId
        })
      ]);
      templatePreviewSelectedItem.value = {
        type: "tableRecord",
        tableId: record.tableId,
        tableName: record.tableName,
        tableDisplayName: record.tableDisplayName,
        recordId: record.recordId,
        recordLabel: record.recordLabel,
        folderId: null
      };
      templatePreviewTableDetail.value = detail;
      templatePreviewBindings.value = bindings;
    } catch (loadError) {
      error.value =
        loadError instanceof Error ? loadError.message : String(loadError);
    } finally {
      templatePreviewLoading.value = false;
    }
  }

  function clearTemplatePreview() {
    templatePreviewSelectedItem.value = null;
    templatePreviewTableDetail.value = null;
    templatePreviewBindings.value = [];
    templatePreviewLoading.value = false;
  }

  function saveLayoutTemplateCards(cards: ViewLayoutTemplateCard[]) {
    const template = selectedLayoutTemplate.value;
    if (!template) {
      return;
    }

    templateLayoutCards.value = cards;
    layoutSaving.value = true;

    if (layoutSaveTimer) {
      globalThis.clearTimeout(layoutSaveTimer);
    }

    layoutSaveTimer = globalThis.setTimeout(() => {
      void persistLayoutTemplateCards(template.id, cards);
    }, 240);
  }

  async function persistLayoutTemplateCards(
    templateId: number,
    cards: ViewLayoutTemplateCard[]
  ) {
    try {
      await api.saveViewLayoutTemplateCards({
        templateId,
        cards
      });
      templateLayoutCards.value = await api.getViewLayoutTemplateCards({
        templateId
      });
    } catch (saveError) {
      error.value =
        saveError instanceof Error ? saveError.message : String(saveError);
    } finally {
      layoutSaving.value = false;
    }
  }

  async function resetCardLayoutOverride(cardId: number) {
    const selected = selectedItem.value;
    const templateId = activeLayoutTemplateId.value;
    if (selected?.type !== "tableRecord" || !templateId) {
      return;
    }

    error.value = "";
    await api.resetViewLayoutCardOverride({
      templateId,
      tableId: selected.tableId,
      recordId: selected.recordId,
      cardId
    });
    await reloadSelectedLayout();
  }

  async function resetRecordLayoutOverrides() {
    const selected = selectedItem.value;
    const templateId = activeLayoutTemplateId.value;
    if (selected?.type !== "tableRecord" || !templateId) {
      return;
    }

    error.value = "";
    await api.resetViewLayoutCardOverrides({
      templateId,
      tableId: selected.tableId,
      recordId: selected.recordId
    });
    await reloadSelectedLayout();
  }

  async function saveLayoutCardColumnBindings(
    bindings: ViewLayoutCardColumnBinding[]
  ) {
    const selected = selectedItem.value;
    const templateId = activeLayoutTemplateId.value;
    if (selected?.type !== "tableRecord" || !templateId) {
      return;
    }

    error.value = "";
    layoutSaving.value = true;
    try {
      await api.saveViewLayoutCardColumnBindings({
        templateId,
        tableId: selected.tableId,
        bindings
      });
      await reloadSelectedLayout();
    } catch (saveError) {
      error.value =
        saveError instanceof Error ? saveError.message : String(saveError);
    } finally {
      layoutSaving.value = false;
    }
  }

  async function reloadSelectedLayout() {
    const selected = selectedItem.value;
    if (selected?.type !== "tableRecord") {
      return;
    }

    const resolvedLayout = await api.getResolvedViewFieldLayout({
      tableId: selected.tableId,
      recordId: selected.recordId,
      folderId: selected.folderId ?? null
    });
    layoutCardItems.value = resolvedLayout.items;
    activeLayoutTemplateId.value = resolvedLayout.activeTemplateId;
  }

  async function createFolder(parentId: number | null, name: string) {
    error.value = "";

    try {
      const createdNode = await api.createViewNavFolder({
        parentId,
        name
      });

      customNodes.value = [...customNodes.value, createdNode].sort(
        compareNodes
      );
      expandedFolderIds.value = Array.from(
        new Set(
          parentId === null
            ? [...expandedFolderIds.value, createdNode.id]
            : [...expandedFolderIds.value, parentId, createdNode.id]
        )
      );
      selectFolder(createdNode);
    } catch (createError) {
      error.value =
        createError instanceof Error
          ? createError.message
          : String(createError);
      throw createError;
    }
  }

  function selectFolderRecord(record: ViewNavFolderRecord) {
    selectedLayoutTemplate.value = null;
    templateLayoutCards.value = [];
    clearTemplatePreview();
    selectedItem.value = {
      type: "tableRecord",
      tableId: record.tableId,
      tableName: record.tableName,
      tableDisplayName: record.tableDisplayName,
      recordId: record.recordId,
      recordLabel: record.recordLabel,
      folderId: record.folderId
    };
    void loadSelectedTableRecord(
      record.tableId,
      record.recordId,
      record.folderId
    );
  }

  async function deleteFolder(node: ViewNavNode) {
    error.value = "";
    const deletingIds = collectFolderSubtreeIds(customNodes.value, node.id);
    const deletingRecordKeys = folderRecords.value
      .filter((record) => deletingIds.includes(record.folderId))
      .map((record) => `${record.tableId}:${record.recordId}`);

    try {
      await api.deleteViewNavFolder({
        folderId: node.id
      });

      customNodes.value = customNodes.value.filter(
        (customNode) => !deletingIds.includes(customNode.id)
      );
      folderRecords.value = folderRecords.value.filter(
        (record) => !deletingIds.includes(record.folderId)
      );
      expandedFolderIds.value = expandedFolderIds.value.filter(
        (folderId) => !deletingIds.includes(folderId)
      );

      if (
        (selectedItem.value?.type === "folder" &&
          deletingIds.includes(selectedItem.value.folderId)) ||
        (selectedItem.value?.type === "tableRecord" &&
          deletingRecordKeys.includes(
            `${selectedItem.value.tableId}:${selectedItem.value.recordId}`
          ))
      ) {
        selectedItem.value = null;
        selectedTableDetail.value = null;
        layoutCardItems.value = [];
        activeLayoutTemplateId.value = null;
      }
      layoutTemplates.value = layoutTemplates.value.filter(
        (template) =>
          template.folderId === null || !deletingIds.includes(template.folderId)
      );
      if (
        selectedLayoutTemplate.value?.folderId !== null &&
        selectedLayoutTemplate.value?.folderId !== undefined &&
        deletingIds.includes(selectedLayoutTemplate.value.folderId)
      ) {
        selectedLayoutTemplate.value = null;
        templateLayoutCards.value = [];
        clearTemplatePreview();
      }
    } catch (deleteError) {
      error.value =
        deleteError instanceof Error
          ? deleteError.message
          : String(deleteError);
      throw deleteError;
    }
  }

  async function addFolderRecords(
    folderId: number,
    section: ViewTableSection,
    records: ViewTableRecordSummary[]
  ) {
    error.value = "";

    try {
      const createdRecords = await api.addViewNavFolderRecords({
        folderId,
        tableId: section.tableId,
        records: records.map((record) => ({
          recordId: record.id,
          recordLabel: record.label
        }))
      });
      mergeFolderRecords(createdRecords);
      expandedFolderIds.value = Array.from(
        new Set([...expandedFolderIds.value, folderId])
      );
      if (
        selectedItem.value?.type === "folder" &&
        selectedItem.value.folderId === folderId
      ) {
        await loadFolderLayoutTemplates(folderId);
      }
    } catch (addError) {
      error.value =
        addError instanceof Error ? addError.message : String(addError);
      throw addError;
    }
  }

  function mergeFolderRecords(createdRecords: ViewNavFolderRecord[]) {
    if (createdRecords.length === 0) {
      return;
    }

    folderRecords.value = [
      ...folderRecords.value.filter(
        (item) =>
          !createdRecords.some((createdRecord) =>
            isSameFolderRecord(item, createdRecord)
          )
      ),
      ...createdRecords
    ].sort(compareFolderRecords);
  }

  function mergeLayoutTemplates(templates: ViewLayoutTemplate[]) {
    if (templates.length === 0) {
      return;
    }

    layoutTemplates.value = [
      ...layoutTemplates.value.filter(
        (template) => !templates.some((incoming) => incoming.id === template.id)
      ),
      ...templates
    ].sort(compareLayoutTemplates);
  }

  async function removeFolderRecord(record: ViewNavFolderRecord) {
    error.value = "";

    try {
      await api.removeViewNavFolderRecord({
        folderRecordId: record.id
      });

      folderRecords.value = folderRecords.value.filter(
        (item) => item.id !== record.id
      );

      if (
        selectedItem.value?.type === "tableRecord" &&
        selectedItem.value.tableId === record.tableId &&
        selectedItem.value.recordId === record.recordId
      ) {
        selectedItem.value = null;
        selectedTableDetail.value = null;
        layoutCardItems.value = [];
        activeLayoutTemplateId.value = null;
      }
      if (
        selectedItem.value?.type === "folder" &&
        selectedItem.value.folderId === record.folderId
      ) {
        await loadFolderLayoutTemplates(record.folderId);
      }
    } catch (removeError) {
      error.value =
        removeError instanceof Error
          ? removeError.message
          : String(removeError);
      throw removeError;
    }
  }

  async function reorderFolderRecords(
    folderId: number,
    records: ViewNavFolderRecord[]
  ) {
    // 左の閲覧目次で確定した順序を保存し、返却された sortOrder をローカル状態へ反映します。
    error.value = "";

    try {
      const updatedRecords = await api.reorderViewNavFolderRecords({
        folderId,
        orderedFolderRecordIds: records.map((record) => record.id)
      });
      mergeFolderRecords(updatedRecords);
    } catch (reorderError) {
      error.value =
        reorderError instanceof Error
          ? reorderError.message
          : String(reorderError);
      throw reorderError;
    }
  }

  function isFolderExpanded(folderId: number) {
    return expandedFolderIds.value.includes(folderId);
  }

  onMounted(() => {
    void initialize();
  });

  return {
    createFolder,
    customTree,
    deleteFolder,
    error,
    clearError,
    layoutCardItems,
    folderCount,
    isFolderExpanded,
    layoutSaving,
    layoutTemplates,
    selectedLayoutTemplate,
    templateLayoutCards,
    templatePreviewBindings,
    templatePreviewLoading,
    templatePreviewSelectedItem,
    templatePreviewTableDetail,
    selectedFolderLayoutTemplates,
    selectedFolderActiveTemplateId,
    loading,
    assignFolderLayoutTemplate,
    createFolderLayoutTemplate,
    createLayoutTemplate,
    deleteLayoutTemplate,
    duplicateLayoutTemplate,
    renameLayoutTemplate,
    saveLayoutTemplateCards,
    selectLayoutTemplate,
    selectTemplatePreviewRecord,
    clearTemplatePreview,
    resetCardLayoutOverride,
    resetRecordLayoutOverrides,
    saveLayoutCardColumnBindings,
    saveRecordLayoutOverrides,
    selectedFolderRecords,
    selectedItem,
    selectedTableDetail,
    selectFolder,
    selectFolderRecord,
    tableSections,
    addFolderRecords,
    removeFolderRecord,
    reorderFolderRecords,
    toggleFolder,
    refresh: initialize
  };
}

function buildViewNavTree(
  nodes: ViewNavNode[],
  records: ViewNavFolderRecord[],
  parentId: number | null
): ViewNavTreeNode[] {
  return nodes
    .filter((node) => node.parentId === parentId)
    .sort(compareNodes)
    .map((node) => ({
      ...node,
      children: buildViewNavTree(nodes, records, node.id),
      records: records
        .filter((record) => record.folderId === node.id)
        .sort(compareFolderRecords)
    }));
}

function compareNodes(left: ViewNavNode, right: ViewNavNode) {
  if (left.sortOrder !== right.sortOrder) {
    return left.sortOrder - right.sortOrder;
  }

  return left.id - right.id;
}

function toggleExpandedId(expandedIds: number[], targetId: number) {
  return expandedIds.includes(targetId)
    ? expandedIds.filter((id) => id !== targetId)
    : [...expandedIds, targetId];
}

function compareFolderRecords(
  left: ViewNavFolderRecord,
  right: ViewNavFolderRecord
) {
  if (left.folderId !== right.folderId) {
    return left.folderId - right.folderId;
  }

  if (left.sortOrder !== right.sortOrder) {
    return left.sortOrder - right.sortOrder;
  }

  return left.id - right.id;
}

function compareLayoutTemplates(
  left: ViewLayoutTemplate,
  right: ViewLayoutTemplate
) {
  if (left.folderId === null && right.folderId !== null) {
    return -1;
  }
  if (left.folderId !== null && right.folderId === null) {
    return 1;
  }
  return left.name.localeCompare(right.name) || left.id - right.id;
}

function isSameFolderRecord(
  left: ViewNavFolderRecord,
  right: ViewNavFolderRecord
) {
  return (
    left.folderId === right.folderId &&
    left.tableId === right.tableId &&
    left.recordId === right.recordId
  );
}

function toLayoutPayloadItems(
  items: ViewLayoutCardItem[]
): SaveViewLayoutCardOverridesPayload["items"] {
  return items.map((item) => ({
    cardId: item.cardId,
    columnId: item.columnId,
    x: item.x,
    y: item.y,
    width: item.width,
    height: item.height,
    visible: item.visible,
    backgroundColor: item.backgroundColor,
    textColor: item.textColor,
    fontSize: item.fontSize,
    textDirection: item.textDirection,
    fontWeight: item.fontWeight,
    textAlign: item.textAlign,
    padding: item.padding,
    paddingTop: item.paddingTop,
    paddingRight: item.paddingRight,
    paddingBottom: item.paddingBottom,
    paddingLeft: item.paddingLeft,
    borderRadius: item.borderRadius,
    showLabel: item.showLabel
  }));
}

function collectFolderSubtreeIds(nodes: ViewNavNode[], rootId: number) {
  const result = new Set<number>([rootId]);
  let changed = true;

  while (changed) {
    changed = false;
    for (const node of nodes) {
      if (
        node.parentId !== null &&
        result.has(node.parentId) &&
        !result.has(node.id)
      ) {
        result.add(node.id);
        changed = true;
      }
    }
  }

  return Array.from(result);
}
