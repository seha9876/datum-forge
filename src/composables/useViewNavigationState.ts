import { computed, ref } from "vue";

import { api } from "../api";

import { buildViewNavTree } from "./useViewNavigation.helpers";

import type {
  TableDetail,
  TemplatePreviewRecordSelection,
  ViewLayoutCardColumnBinding,
  ViewLayoutCardItem,
  ViewLayoutTemplate,
  ViewLayoutTemplateCard,
  ViewNavFolderRecord,
  ViewNavNode,
  ViewSelection,
  ViewTableSection
} from "../types";

export function useViewNavigationState() {
  const tableSections = ref<ViewTableSection[]>([]);
  const customNodes = ref<ViewNavNode[]>([]);
  const folderRecords = ref<ViewNavFolderRecord[]>([]);
  const expandedFolderIds = ref<number[]>([]);
  const selectedItem = ref<ViewSelection | null>(null);
  const selectedTableDetail = ref<TableDetail | null>(null);

  const layoutCardItems = ref<ViewLayoutCardItem[]>([]);
  const activeLayoutTemplateId = ref<number | null>(null);
  const activeLayoutTemplateName = ref<string | null>(null);
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

  return {
    activeLayoutTemplateId,
    activeLayoutTemplateName,
    clearError,
    customNodes,
    customTree,
    error,
    expandedFolderIds,
    folderCount,
    folderRecords,
    initialize,
    layoutCardItems,
    layoutSaving,
    layoutTemplates,
    loading,
    selectedFolderActiveTemplateId,
    selectedFolderLayoutTemplates,
    selectedFolderRecords,
    selectedItem,
    selectedLayoutTemplate,
    selectedTableDetail,
    tableSections,
    templateLayoutCards,
    templatePreviewBindings,
    templatePreviewLoading,
    templatePreviewSelectedItem,
    templatePreviewTableDetail
  };
}

export type ViewNavigationState = ReturnType<typeof useViewNavigationState>;
export type TemplatePreviewRecord = TemplatePreviewRecordSelection;
