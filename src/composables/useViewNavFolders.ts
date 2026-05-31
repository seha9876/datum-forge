import { api } from "../api";

import {
  collectFolderSubtreeIds,
  compareFolderRecords,
  compareNodes,
  isSameFolderRecord,
  toggleExpandedId
} from "./useViewNavigation.helpers";

import type {
  ViewNavFolderRecord,
  ViewNavNode,
  ViewTableRecordSummary,
  ViewTableSection
} from "../types";
import type { ViewLayoutPersistenceActions } from "./useViewLayoutPersistence";
import type { ViewLayoutTemplateActions } from "./useViewLayoutTemplates";
import type { ViewNavigationState } from "./useViewNavigationState";

/**
 * 閲覧ナビのフォルダ/登録済みレコード操作だけを扱います。
 * レイアウトテンプレートの実処理は依存関数として受け取り、責務をまたぐ保存仕様をここに閉じ込めません。
 */
export function useViewNavFolders(
  state: ViewNavigationState,
  dependencies: {
    clearTemplatePreview: ViewLayoutTemplateActions["clearTemplatePreview"];
    loadFolderLayoutTemplates: ViewLayoutTemplateActions["loadFolderLayoutTemplates"];
    mergeLayoutTemplates: ViewLayoutTemplateActions["mergeLayoutTemplates"];
    reloadSelectedLayout: ViewLayoutPersistenceActions["reloadSelectedLayout"];
  }
) {
  function toggleFolder(folderId: number) {
    state.expandedFolderIds.value = toggleExpandedId(
      state.expandedFolderIds.value,
      folderId
    );
  }

  function selectFolder(node: ViewNavNode) {
    state.selectedLayoutTemplate.value = null;
    state.templateLayoutCards.value = [];
    dependencies.clearTemplatePreview();
    state.selectedItem.value = {
      type: "folder",
      folderId: node.id,
      folderName: node.name
    };
    state.selectedTableDetail.value = null;
    state.layoutCardItems.value = [];
    state.activeLayoutTemplateId.value = null;
    state.activeLayoutTemplateName.value = null;
    // フォルダ選択時は、そのフォルダに割り当て可能なテンプレートだけを再同期します。
    void dependencies.loadFolderLayoutTemplates(node.id);
  }

  async function loadSelectedTableRecord(
    tableId: number,
    recordId: number,
    folderId: number | null,
    folderRecordId: number | null
  ) {
    state.error.value = "";
    state.loading.value = true;

    try {
      const [detail, resolvedLayout] = await Promise.all([
        api.getTableDetail(tableId),
        api.getResolvedViewFieldLayout({
          tableId,
          recordId,
          folderId,
          folderRecordId
        })
      ]);
      state.selectedTableDetail.value = detail;
      state.layoutCardItems.value = resolvedLayout.items;
      state.activeLayoutTemplateId.value = resolvedLayout.activeTemplateId;
      state.activeLayoutTemplateName.value = resolvedLayout.activeTemplateName;
    } catch (loadError) {
      state.error.value =
        loadError instanceof Error ? loadError.message : String(loadError);
    } finally {
      state.loading.value = false;
    }
  }

  async function createFolder(parentId: number | null, name: string) {
    state.error.value = "";

    try {
      const createdNode = await api.createViewNavFolder({
        parentId,
        name
      });

      state.customNodes.value = [...state.customNodes.value, createdNode].sort(
        compareNodes
      );
      state.expandedFolderIds.value = Array.from(
        new Set(
          parentId === null
            ? [...state.expandedFolderIds.value, createdNode.id]
            : [...state.expandedFolderIds.value, parentId, createdNode.id]
        )
      );
      selectFolder(createdNode);
    } catch (createError) {
      state.error.value =
        createError instanceof Error
          ? createError.message
          : String(createError);
      throw createError;
    }
  }

  function selectFolderRecord(record: ViewNavFolderRecord) {
    state.selectedLayoutTemplate.value = null;
    state.templateLayoutCards.value = [];
    dependencies.clearTemplatePreview();
    state.selectedItem.value = {
      type: "tableRecord",
      tableId: record.tableId,
      tableName: record.tableName,
      tableDisplayName: record.tableDisplayName,
      recordId: record.recordId,
      recordLabel: record.recordLabel,
      folderId: record.folderId,
      folderRecordId: record.id,
      recordTemplateId: record.recordTemplateId
    };
    // レコード表示は「フォルダ所属」と「解決済みレイアウト」を両方必要とするため並行して更新します。
    void dependencies.loadFolderLayoutTemplates(record.folderId);
    void loadSelectedTableRecord(
      record.tableId,
      record.recordId,
      record.folderId,
      record.id
    );
  }

  async function deleteFolder(node: ViewNavNode) {
    state.error.value = "";
    const deletingIds = collectFolderSubtreeIds(
      state.customNodes.value,
      node.id
    );
    // 子孫フォルダに含まれるレコード選択も無効になるため、record keyで選択解除対象を判定します。
    const deletingRecordKeys = state.folderRecords.value
      .filter((record) => deletingIds.includes(record.folderId))
      .map((record) => `${record.tableId}:${record.recordId}`);

    try {
      await api.deleteViewNavFolder({
        folderId: node.id
      });

      state.customNodes.value = state.customNodes.value.filter(
        (customNode) => !deletingIds.includes(customNode.id)
      );
      state.folderRecords.value = state.folderRecords.value.filter(
        (record) => !deletingIds.includes(record.folderId)
      );
      state.expandedFolderIds.value = state.expandedFolderIds.value.filter(
        (folderId) => !deletingIds.includes(folderId)
      );

      if (
        (state.selectedItem.value?.type === "folder" &&
          deletingIds.includes(state.selectedItem.value.folderId)) ||
        (state.selectedItem.value?.type === "tableRecord" &&
          deletingRecordKeys.includes(
            `${state.selectedItem.value.tableId}:${state.selectedItem.value.recordId}`
          ))
      ) {
        state.selectedItem.value = null;
        state.selectedTableDetail.value = null;
        state.layoutCardItems.value = [];
        state.activeLayoutTemplateId.value = null;
        state.activeLayoutTemplateName.value = null;
      }
      state.layoutTemplates.value = state.layoutTemplates.value.filter(
        (template) =>
          template.folderId === null || !deletingIds.includes(template.folderId)
      );
      if (
        state.selectedLayoutTemplate.value?.folderId !== null &&
        state.selectedLayoutTemplate.value?.folderId !== undefined &&
        deletingIds.includes(state.selectedLayoutTemplate.value.folderId)
      ) {
        state.selectedLayoutTemplate.value = null;
        state.templateLayoutCards.value = [];
        dependencies.clearTemplatePreview();
      }
    } catch (deleteError) {
      state.error.value =
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
    state.error.value = "";

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
      state.expandedFolderIds.value = Array.from(
        new Set([...state.expandedFolderIds.value, folderId])
      );
      if (
        state.selectedItem.value?.type === "folder" &&
        state.selectedItem.value.folderId === folderId
      ) {
        await dependencies.loadFolderLayoutTemplates(folderId);
      }
    } catch (addError) {
      state.error.value =
        addError instanceof Error ? addError.message : String(addError);
      throw addError;
    }
  }

  function mergeFolderRecords(createdRecords: ViewNavFolderRecord[]) {
    if (createdRecords.length === 0) {
      return;
    }

    state.folderRecords.value = [
      ...state.folderRecords.value.filter(
        (item) =>
          !createdRecords.some((createdRecord) =>
            isSameFolderRecord(item, createdRecord)
          )
      ),
      ...createdRecords
    ].sort(compareFolderRecords);
  }

  function syncSelectedFolderRecord(record: ViewNavFolderRecord) {
    const selected = state.selectedItem.value;
    if (
      selected?.type !== "tableRecord" ||
      selected.folderRecordId !== record.id
    ) {
      return;
    }

    state.selectedItem.value = {
      ...selected,
      folderId: record.folderId,
      folderRecordId: record.id,
      recordTemplateId: record.recordTemplateId
    };
  }

  async function removeFolderRecord(record: ViewNavFolderRecord) {
    state.error.value = "";

    try {
      await api.removeViewNavFolderRecord({
        folderRecordId: record.id
      });

      state.folderRecords.value = state.folderRecords.value.filter(
        (item) => item.id !== record.id
      );

      if (
        state.selectedItem.value?.type === "tableRecord" &&
        state.selectedItem.value.tableId === record.tableId &&
        state.selectedItem.value.recordId === record.recordId
      ) {
        state.selectedItem.value = null;
        state.selectedTableDetail.value = null;
        state.layoutCardItems.value = [];
        state.activeLayoutTemplateId.value = null;
        state.activeLayoutTemplateName.value = null;
      }
      if (
        state.selectedItem.value?.type === "folder" &&
        state.selectedItem.value.folderId === record.folderId
      ) {
        await dependencies.loadFolderLayoutTemplates(record.folderId);
      }
    } catch (removeError) {
      state.error.value =
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
    state.error.value = "";

    try {
      const updatedRecords = await api.reorderViewNavFolderRecords({
        folderId,
        orderedFolderRecordIds: records.map((record) => record.id)
      });
      mergeFolderRecords(updatedRecords);
    } catch (reorderError) {
      state.error.value =
        reorderError instanceof Error
          ? reorderError.message
          : String(reorderError);
      throw reorderError;
    }
  }

  function isFolderExpanded(folderId: number) {
    return state.expandedFolderIds.value.includes(folderId);
  }

  return {
    addFolderRecords,
    createFolder,
    deleteFolder,
    isFolderExpanded,
    mergeFolderRecords,
    removeFolderRecord,
    reorderFolderRecords,
    selectFolder,
    selectFolderRecord,
    syncSelectedFolderRecord,
    toggleFolder
  };
}

export type ViewNavFolderActions = ReturnType<typeof useViewNavFolders>;
