import { onMounted } from "vue";

import { useViewLayoutPersistence } from "./useViewLayoutPersistence";
import { useViewLayoutTemplates } from "./useViewLayoutTemplates";
import { useViewNavFolders } from "./useViewNavFolders";
import { useViewNavigationState } from "./useViewNavigationState";

export function useViewNavigation() {
  const state = useViewNavigationState();
  const persistenceActions = useViewLayoutPersistence(state);

  let templateActions!: ReturnType<typeof useViewLayoutTemplates>;
  const folderActions = useViewNavFolders(state, {
    clearTemplatePreview: () => templateActions.clearTemplatePreview(),
    loadFolderLayoutTemplates: (folderId) =>
      templateActions.loadFolderLayoutTemplates(folderId),
    mergeLayoutTemplates: (templates) =>
      templateActions.mergeLayoutTemplates(templates),
    reloadSelectedLayout: () => persistenceActions.reloadSelectedLayout()
  });

  templateActions = useViewLayoutTemplates(state, {
    mergeFolderRecords: folderActions.mergeFolderRecords,
    reloadSelectedLayout: persistenceActions.reloadSelectedLayout,
    syncSelectedFolderRecord: folderActions.syncSelectedFolderRecord
  });

  onMounted(() => {
    void state.initialize();
  });

  return {
    createFolder: folderActions.createFolder,
    customTree: state.customTree,
    deleteFolder: folderActions.deleteFolder,
    error: state.error,
    clearError: state.clearError,
    layoutCardItems: state.layoutCardItems,
    activeLayoutTemplateId: state.activeLayoutTemplateId,
    activeLayoutTemplateName: state.activeLayoutTemplateName,
    folderCount: state.folderCount,
    isFolderExpanded: folderActions.isFolderExpanded,
    layoutSaving: state.layoutSaving,
    layoutTemplates: state.layoutTemplates,
    selectedLayoutTemplate: state.selectedLayoutTemplate,
    templateLayoutCards: state.templateLayoutCards,
    templatePreviewBindings: state.templatePreviewBindings,
    templatePreviewLoading: state.templatePreviewLoading,
    templatePreviewSelectedItem: state.templatePreviewSelectedItem,
    templatePreviewTableDetail: state.templatePreviewTableDetail,
    selectedFolderLayoutTemplates: state.selectedFolderLayoutTemplates,
    selectedFolderActiveTemplateId: state.selectedFolderActiveTemplateId,
    loading: state.loading,
    assignFolderLayoutTemplate: templateActions.assignFolderLayoutTemplate,
    assignRecordLayoutTemplate: templateActions.assignRecordLayoutTemplate,
    clearRecordLayoutTemplate: templateActions.clearRecordLayoutTemplate,
    createFolderLayoutTemplate: templateActions.createFolderLayoutTemplate,
    createLayoutTemplate: templateActions.createLayoutTemplate,
    deleteLayoutTemplate: templateActions.deleteLayoutTemplate,
    duplicateLayoutTemplate: templateActions.duplicateLayoutTemplate,
    renameLayoutTemplate: templateActions.renameLayoutTemplate,
    saveLayoutTemplateCards: persistenceActions.saveLayoutTemplateCards,
    selectLayoutTemplate: templateActions.selectLayoutTemplate,
    selectTemplatePreviewRecord: templateActions.selectTemplatePreviewRecord,
    clearTemplatePreview: templateActions.clearTemplatePreview,
    resetCardLayoutOverride: persistenceActions.resetCardLayoutOverride,
    resetRecordLayoutOverrides: persistenceActions.resetRecordLayoutOverrides,
    saveLayoutCardColumnBindings:
      persistenceActions.saveLayoutCardColumnBindings,
    saveRecordLayoutOverrides: persistenceActions.saveRecordLayoutOverrides,
    selectedFolderRecords: state.selectedFolderRecords,
    selectedItem: state.selectedItem,
    selectedTableDetail: state.selectedTableDetail,
    selectFolder: folderActions.selectFolder,
    selectFolderRecord: folderActions.selectFolderRecord,
    tableSections: state.tableSections,
    addFolderRecords: folderActions.addFolderRecords,
    removeFolderRecord: folderActions.removeFolderRecord,
    reorderFolderRecords: folderActions.reorderFolderRecords,
    toggleFolder: folderActions.toggleFolder,
    refresh: state.initialize
  };
}
