<script setup lang="ts">
import RecordTagEditor from "./RecordTagEditor.vue";
import ViewContentPlaceholder from "./ViewContentPlaceholder.vue";
import ViewFreeLayoutCanvas from "./ViewFreeLayoutCanvas.vue";

import type {
  RecordTag,
  TableDetail,
  TemplatePreviewRecordSelection,
  ViewLayoutCardItem,
  ViewLayoutCardColumnBinding,
  ViewLayoutTemplate,
  ViewLayoutTemplateCard,
  ViewNavFolderRecord,
  ViewSelection,
  ViewTableRecordSummary,
  ViewTableSection
} from "../../../types";

defineProps<{
  error: string;
  onClearError: () => void;
  layoutCardItems: ViewLayoutCardItem[];
  folderCount: number;
  layoutSaving: boolean;
  loading: boolean;
  recordTags: RecordTag[];
  selectedFolderActiveTemplateId: number | null;
  selectedFolderLayoutTemplates: ViewLayoutTemplate[];
  selectedFolderRecords: ViewNavFolderRecord[];
  selectedRecordTags: RecordTag[];
  selectedItem: ViewSelection | null;
  selectedLayoutTemplate: ViewLayoutTemplate | null;
  selectedTableDetail: TableDetail | null;
  templateLayoutCards: ViewLayoutTemplateCard[];
  templatePreviewBindings: ViewLayoutCardColumnBinding[];
  templatePreviewLoading: boolean;
  templatePreviewSelectedItem: ViewSelection | null;
  templatePreviewTableDetail: TableDetail | null;
  tableCount: number;
  tableSections: ViewTableSection[];
  onAddFolderRecords: (
    folderId: number,
    section: ViewTableSection,
    records: ViewTableRecordSummary[]
  ) => Promise<void>;
  onAttachExistingTag: (
    tableId: number,
    recordId: number,
    tagId: number
  ) => Promise<void>;
  onCreateAndAttachTag: (
    tableId: number,
    recordId: number,
    name: string
  ) => Promise<void>;
  onDetachTag: (
    tableId: number,
    recordId: number,
    tagId: number
  ) => Promise<void>;
  onRemoveFolderRecord: (record: ViewNavFolderRecord) => Promise<void>;
  onAssignFolderLayoutTemplate: (
    folderId: number,
    templateId: number
  ) => Promise<void>;
  onCreateFolderLayoutTemplate: (
    folderId: number | null,
    name: string
  ) => Promise<void>;
  onCreateFolder: (parentId: number | null, name: string) => Promise<void>;
  onClearTemplatePreview: () => void;
  onOpenModeHelp: () => void;
  onSelectTemplatePreviewRecord: (
    record: TemplatePreviewRecordSelection
  ) => Promise<void>;
  onResetCardLayoutOverride: (columnId: number) => Promise<void>;
  onResetRecordLayoutOverrides: () => Promise<void>;
  onSaveLayoutCardColumnBindings: (
    bindings: ViewLayoutCardColumnBinding[]
  ) => Promise<void>;
  onSaveRecordLayoutOverrides: (items: ViewLayoutCardItem[]) => void;
  onSaveLayoutTemplateCards: (cards: ViewLayoutTemplateCard[]) => void;
}>();
</script>

<template>
  <!-- 閲覧モード本体。選択内容に応じて編集ビューや案内表示を出し分けます。 -->
  <div class="view-mode-layout">
    <!-- 閲覧ナビやレイアウトの読み込みで失敗したときのエラー表示です。 -->
    <v-alert
      v-if="error"
      type="error"
      variant="tonal"
      rounded="xl"
      border="start"
      closable
      class="error-alert"
      @click:close="onClearError"
    >
      {{ error }}
    </v-alert>

    <!-- テンプレートを選んだときは、カード枠編集用のキャンバスを表示します。 -->
    <template v-if="selectedLayoutTemplate">
      <ViewFreeLayoutCanvas
        editor-mode="template"
        :detail="null"
        :layout-items="[]"
        :saving="layoutSaving"
        :selected-item="null"
        :table-sections="tableSections"
        :template-cards="templateLayoutCards"
        :template-name="selectedLayoutTemplate.name"
        :template-preview-bindings="templatePreviewBindings"
        :template-preview-detail="templatePreviewTableDetail"
        :template-preview-loading="templatePreviewLoading"
        :template-preview-selected-item="templatePreviewSelectedItem"
        @clear-template-preview="onClearTemplatePreview"
        @save-template-cards="onSaveLayoutTemplateCards"
        @select-template-preview-record="onSelectTemplatePreviewRecord"
      />
    </template>

    <template v-else-if="selectedItem?.type === 'tableRecord'">
      <RecordTagEditor
        :all-tags="recordTags"
        :selected-item="selectedItem"
        :selected-tags="selectedRecordTags"
        :on-attach-existing-tag="onAttachExistingTag"
        :on-create-and-attach-tag="onCreateAndAttachTag"
        :on-detach-tag="onDetachTag"
      />

      <ViewFreeLayoutCanvas
        :detail="selectedTableDetail"
        :layout-items="layoutCardItems"
        :saving="layoutSaving"
        :selected-item="selectedItem"
        @reset-card-override="onResetCardLayoutOverride"
        @reset-record-overrides="onResetRecordLayoutOverrides"
        @save-card-column-bindings="onSaveLayoutCardColumnBindings"
        @save-record-overrides="onSaveRecordLayoutOverrides"
      />
    </template>

    <!-- フォルダー選択中または未選択時は、次の操作を案内するプレースホルダーを表示します。 -->
    <ViewContentPlaceholder
      v-else
      :folder-count="folderCount"
      :on-add-folder-records="onAddFolderRecords"
      :on-assign-folder-layout-template="onAssignFolderLayoutTemplate"
      :on-create-folder="onCreateFolder"
      :on-create-folder-layout-template="onCreateFolderLayoutTemplate"
      :on-open-mode-help="onOpenModeHelp"
      :on-remove-folder-record="onRemoveFolderRecord"
      :selected-folder-active-template-id="selectedFolderActiveTemplateId"
      :selected-folder-layout-templates="selectedFolderLayoutTemplates"
      :selected-folder-records="selectedFolderRecords"
      :selected-item="selectedItem"
      :table-count="tableCount"
      :table-sections="tableSections"
    />

    <!-- バックエンドから閲覧データを読み込んでいる間の状態表示です。 -->
    <div v-if="loading" class="view-loading-state">
      <v-progress-circular indeterminate color="primary" size="22" />
      <span>閲覧ナビゲーションを読み込み中です。</span>
    </div>
  </div>
</template>
