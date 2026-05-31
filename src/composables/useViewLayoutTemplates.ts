import { api } from "../api";

import { compareLayoutTemplates } from "./useViewNavigation.helpers";

import type {
  TemplatePreviewRecordSelection,
  ViewLayoutTemplate,
  ViewNavFolderRecord
} from "../types";
import type { ViewLayoutPersistenceActions } from "./useViewLayoutPersistence";
import type { ViewNavigationState } from "./useViewNavigationState";

/**
 * レイアウトテンプレートの作成・選択・割り当てを担当します。
 * 実際のカード保存や解決済みレイアウト再取得は persistence 側へ委譲します。
 */
export function useViewLayoutTemplates(
  state: ViewNavigationState,
  dependencies: {
    mergeFolderRecords: (records: ViewNavFolderRecord[]) => void;
    reloadSelectedLayout: ViewLayoutPersistenceActions["reloadSelectedLayout"];
    syncSelectedFolderRecord: (record: ViewNavFolderRecord) => void;
  }
) {
  async function loadFolderLayoutTemplates(folderId: number) {
    try {
      const resolved = await api.listViewLayoutTemplatesForFolder({
        folderId
      });
      state.selectedFolderLayoutTemplates.value = resolved.templates;
      state.selectedFolderActiveTemplateId.value = resolved.activeTemplateId;
      mergeLayoutTemplates(resolved.templates);
    } catch (loadError) {
      state.error.value =
        loadError instanceof Error ? loadError.message : String(loadError);
    }
  }

  async function createFolderLayoutTemplate(
    folderId: number | null,
    name: string
  ) {
    state.error.value = "";
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
    state.error.value = "";
    await api.assignViewLayoutFolderTemplate({
      folderId,
      templateId
    });
    await loadFolderLayoutTemplates(folderId);
    // レコード個別テンプレートがない場合だけ、フォルダ割当の変更が現在表示へ反映されます。
    if (
      state.selectedItem.value?.type === "tableRecord" &&
      state.selectedItem.value.folderId === folderId &&
      !state.selectedItem.value.recordTemplateId
    ) {
      await dependencies.reloadSelectedLayout();
    }
  }

  async function assignRecordLayoutTemplate(
    folderRecordId: number,
    templateId: number
  ) {
    state.error.value = "";
    try {
      const updatedRecord = await api.assignViewLayoutRecordTemplate({
        folderRecordId,
        templateId
      });
      dependencies.mergeFolderRecords([updatedRecord]);
      dependencies.syncSelectedFolderRecord(updatedRecord);
      await dependencies.reloadSelectedLayout();
    } catch (saveError) {
      state.error.value =
        saveError instanceof Error ? saveError.message : String(saveError);
      throw saveError;
    }
  }

  async function clearRecordLayoutTemplate(folderRecordId: number) {
    state.error.value = "";
    try {
      const updatedRecord = await api.clearViewLayoutRecordTemplate({
        folderRecordId
      });
      dependencies.mergeFolderRecords([updatedRecord]);
      dependencies.syncSelectedFolderRecord(updatedRecord);
      await dependencies.reloadSelectedLayout();
    } catch (saveError) {
      state.error.value =
        saveError instanceof Error ? saveError.message : String(saveError);
      throw saveError;
    }
  }

  async function selectLayoutTemplate(template: ViewLayoutTemplate) {
    state.error.value = "";
    state.loading.value = true;
    clearTemplatePreview();
    state.selectedItem.value = null;
    state.selectedTableDetail.value = null;
    state.layoutCardItems.value = [];
    state.activeLayoutTemplateId.value = null;
    state.activeLayoutTemplateName.value = null;
    state.selectedFolderLayoutTemplates.value = [];
    state.selectedFolderActiveTemplateId.value = null;
    state.selectedLayoutTemplate.value = template;

    try {
      state.templateLayoutCards.value = await api.getViewLayoutTemplateCards({
        templateId: template.id
      });
    } catch (loadError) {
      state.error.value =
        loadError instanceof Error ? loadError.message : String(loadError);
    } finally {
      state.loading.value = false;
    }
  }

  async function createLayoutTemplate(name: string) {
    state.error.value = "";
    const template = await api.createViewLayoutTemplate({
      name,
      scopeType: "folder",
      folderId: null
    });
    mergeLayoutTemplates([template]);
    await selectLayoutTemplate(template);
  }

  async function renameLayoutTemplate(templateId: number, name: string) {
    state.error.value = "";
    const template = await api.renameViewLayoutTemplate({ templateId, name });
    mergeLayoutTemplates([template]);
    if (state.selectedLayoutTemplate.value?.id === templateId) {
      state.selectedLayoutTemplate.value = template;
    }
  }

  async function duplicateLayoutTemplate(templateId: number, name: string) {
    state.error.value = "";
    const template = await api.duplicateViewLayoutTemplate({
      templateId,
      name
    });
    mergeLayoutTemplates([template]);
    await selectLayoutTemplate(template);
  }

  async function deleteLayoutTemplate(templateId: number) {
    state.error.value = "";
    await api.deleteViewLayoutTemplate({ templateId });
    state.layoutTemplates.value = state.layoutTemplates.value.filter(
      (template) => template.id !== templateId
    );
    state.selectedFolderLayoutTemplates.value =
      state.selectedFolderLayoutTemplates.value.filter(
        (template) => template.id !== templateId
      );
    state.folderRecords.value = state.folderRecords.value.map((record) =>
      record.recordTemplateId === templateId
        ? { ...record, recordTemplateId: null }
        : record
    );
    // 削除したテンプレートを参照する表示状態を残すと、次回保存時に存在しないIDを送ってしまいます。
    if (
      state.selectedItem.value?.type === "tableRecord" &&
      state.selectedItem.value.recordTemplateId === templateId
    ) {
      state.selectedItem.value = {
        ...state.selectedItem.value,
        recordTemplateId: null
      };
      await dependencies.reloadSelectedLayout();
    }
    if (state.selectedFolderActiveTemplateId.value === templateId) {
      state.selectedFolderActiveTemplateId.value = null;
    }
    if (
      state.selectedItem.value?.type === "tableRecord" &&
      state.activeLayoutTemplateId.value === templateId
    ) {
      await dependencies.reloadSelectedLayout();
    }
    if (state.selectedLayoutTemplate.value?.id === templateId) {
      state.selectedLayoutTemplate.value = null;
      state.templateLayoutCards.value = [];
      clearTemplatePreview();
    }
  }

  async function selectTemplatePreviewRecord(
    record: TemplatePreviewRecordSelection
  ) {
    const template = state.selectedLayoutTemplate.value;
    if (!template) {
      return;
    }

    state.error.value = "";
    state.templatePreviewLoading.value = true;
    try {
      const [detail, bindings] = await Promise.all([
        api.getTableDetail(record.tableId),
        api.listViewLayoutCardColumnBindings({
          templateId: template.id,
          tableId: record.tableId
        })
      ]);
      state.templatePreviewSelectedItem.value = {
        type: "tableRecord",
        tableId: record.tableId,
        tableName: record.tableName,
        tableDisplayName: record.tableDisplayName,
        recordId: record.recordId,
        recordLabel: record.recordLabel,
        folderId: null
      };
      state.templatePreviewTableDetail.value = detail;
      state.templatePreviewBindings.value = bindings;
    } catch (loadError) {
      state.error.value =
        loadError instanceof Error ? loadError.message : String(loadError);
    } finally {
      state.templatePreviewLoading.value = false;
    }
  }

  function clearTemplatePreview() {
    state.templatePreviewSelectedItem.value = null;
    state.templatePreviewTableDetail.value = null;
    state.templatePreviewBindings.value = [];
    state.templatePreviewLoading.value = false;
  }

  function mergeLayoutTemplates(templates: ViewLayoutTemplate[]) {
    if (templates.length === 0) {
      return;
    }

    state.layoutTemplates.value = [
      ...state.layoutTemplates.value.filter(
        (template) => !templates.some((incoming) => incoming.id === template.id)
      ),
      ...templates
    ].sort(compareLayoutTemplates);
  }

  return {
    assignFolderLayoutTemplate,
    assignRecordLayoutTemplate,
    clearRecordLayoutTemplate,
    clearTemplatePreview,
    createFolderLayoutTemplate,
    createLayoutTemplate,
    deleteLayoutTemplate,
    duplicateLayoutTemplate,
    loadFolderLayoutTemplates,
    mergeLayoutTemplates,
    renameLayoutTemplate,
    selectLayoutTemplate,
    selectTemplatePreviewRecord
  };
}

export type ViewLayoutTemplateActions = ReturnType<
  typeof useViewLayoutTemplates
>;
