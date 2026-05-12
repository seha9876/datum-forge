<script setup lang="ts">
import DataModeView from "./workspace/data/DataModeView.vue";
import DesignModeView from "./workspace/design/DesignModeView.vue";
import OptionGroupManagerPanel from "./workspace/master/OptionGroupManagerPanel.vue";
import TagManagerPanel from "./workspace/master/TagManagerPanel.vue";
import ViewModeContent from "./workspace/view/ViewModeContent.vue";

import type { WorkspaceMode } from "../composables/useWorkspaceMode";
import type {
  AddColumnPayload,
  AppBootstrap,
  AppColumn,
  FieldType,
  ReferenceChoice,
  RecordTag,
  RecordTagGroup,
  SaveOptionGroupPayload,
  SaveRecordTagGroupPayload,
  SaveRecordTagPayload,
  SelectOptionGroup,
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
} from "../types";

type MasterSection = "options" | "tags";

defineProps<{
  addOptionRow: () => void;
  bootstrap: AppBootstrap | null;
  columnForm: AddColumnPayload;
  currentMode: WorkspaceMode;
  editingRecordId: number | null;
  error: string;
  layoutCardItems: ViewLayoutCardItem[];
  fieldTypes: FieldType[];
  fieldTypeLabel: (fieldType: FieldType) => string;
  fieldTypeMeta: (column: AppColumn) => string;
  inputType: (fieldType: FieldType) => string;
  optionGroupForm: SaveOptionGroupPayload;
  recordValues: Record<string, unknown>;
  recordTagGroups: RecordTagGroup[];
  recordTags: RecordTag[];
  referenceChoices: (column: AppColumn) => ReferenceChoice[];
  selectedFolderActiveTemplateId: number | null;
  selectedFolderLayoutTemplates: ViewLayoutTemplate[];
  selectedFolderRecords: ViewNavFolderRecord[];
  selectedLayoutTemplate: ViewLayoutTemplate | null;
  selectedRecordTags: RecordTag[];
  selectedItem: ViewSelection | null;
  selectedMasterSection: MasterSection;
  selectedOptionGroupId: number | null;
  selectedTable: TableDetail | null;
  selectedViewTableDetail: TableDetail | null;
  templateLayoutCards: ViewLayoutTemplateCard[];
  templatePreviewBindings: ViewLayoutCardColumnBinding[];
  templatePreviewLoading: boolean;
  templatePreviewSelectedItem: ViewSelection | null;
  templatePreviewTableDetail: TableDetail | null;
  tableSections: ViewTableSection[];
  tableCount: number;
  viewError: string;
  onClearViewError: () => void;
  onOpenTableDialog: () => void;
  onOpenModeHelp: () => void;
  viewFolderCount: number;
  viewLayoutSaving: boolean;
  viewLoading: boolean;
  onDeleteColumn: (columnId: number) => Promise<void>;
  onDeleteRecord: (recordId: number) => Promise<void>;
  onRemoveOptionRow: (index: number) => void;
  onReorderColumns: (columns: AppColumn[]) => Promise<void>;
  onResetOptionGroupForm: () => void;
  onResetRecordForm: () => void;
  onStartEditOptionGroup: (group: SelectOptionGroup) => void;
  onStartEditRecord: (recordId: number) => void;
  onSubmitColumn: () => Promise<void>;
  onSubmitOptionGroup: () => Promise<void>;
  onSubmitRecord: () => Promise<boolean>;
  onSyncOptionOrdering: () => void;
  onSaveRecordTagGroup: (payload: SaveRecordTagGroupPayload) => Promise<void>;
  onDeleteRecordTagGroup: (groupId: number) => Promise<void>;
  onSaveRecordTag: (payload: SaveRecordTagPayload) => Promise<void>;
  onDeleteRecordTag: (tagId: number) => Promise<void>;
  onAttachRecordTagGroup: (tagId: number, groupId: number) => Promise<void>;
  onDetachRecordTagGroup: (tagId: number, groupId: number) => Promise<void>;
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
  onAddFolderRecords: (
    folderId: number,
    section: ViewTableSection,
    records: ViewTableRecordSummary[]
  ) => Promise<void>;
  onRemoveFolderRecord: (record: ViewNavFolderRecord) => Promise<void>;
  onUpdateLabelColumn: (labelColumnId: number | null) => Promise<void>;
  onUpdateColumn: (
    columnId: number,
    columnName: string,
    displayName: string,
    isRequired: boolean
  ) => Promise<void>;
}>();
</script>

<template>
  <main class="content">
    <v-alert
      v-if="error"
      type="error"
      variant="tonal"
      rounded="xl"
      border="start"
      class="error-alert"
    >
      {{ error }}
    </v-alert>

    <template
      v-if="selectedTable || currentMode === 'master' || currentMode === 'view'"
    >
      <DesignModeView
        v-if="selectedTable && currentMode === 'design'"
        :bootstrap="bootstrap"
        :column-form="columnForm"
        :field-type-label="fieldTypeLabel"
        :field-type-meta="fieldTypeMeta"
        :field-types="fieldTypes"
        :selected-table="selectedTable"
        :on-delete-column="onDeleteColumn"
        :on-reorder-columns="onReorderColumns"
        :on-submit-column="onSubmitColumn"
        :on-update-column="onUpdateColumn"
        :on-update-label-column="onUpdateLabelColumn"
      />

      <DataModeView
        v-else-if="selectedTable && currentMode === 'data'"
        :bootstrap="bootstrap"
        :editing-record-id="editingRecordId"
        :input-type="inputType"
        :record-values="recordValues"
        :reference-choices="referenceChoices"
        :selected-table="selectedTable"
        :on-delete-record="onDeleteRecord"
        :on-reset-record-form="onResetRecordForm"
        :on-start-edit-record="onStartEditRecord"
        :on-submit-record="onSubmitRecord"
      />

      <ViewModeContent
        v-else-if="currentMode === 'view'"
        :error="viewError"
        :on-clear-error="onClearViewError"
        :layout-card-items="layoutCardItems"
        :folder-count="viewFolderCount"
        :layout-saving="viewLayoutSaving"
        :loading="viewLoading"
        :record-tags="recordTags"
        :selected-record-tags="selectedRecordTags"
        :on-add-folder-records="onAddFolderRecords"
        :on-assign-folder-layout-template="onAssignFolderLayoutTemplate"
        :on-attach-existing-tag="onAttachExistingTag"
        :on-create-folder="onCreateFolder"
        :on-create-folder-layout-template="onCreateFolderLayoutTemplate"
        :on-clear-template-preview="onClearTemplatePreview"
        :on-open-mode-help="onOpenModeHelp"
        :on-create-and-attach-tag="onCreateAndAttachTag"
        :on-detach-tag="onDetachTag"
        :on-remove-folder-record="onRemoveFolderRecord"
        :selected-item="selectedItem"
        :selected-folder-active-template-id="selectedFolderActiveTemplateId"
        :selected-folder-layout-templates="selectedFolderLayoutTemplates"
        :selected-folder-records="selectedFolderRecords"
        :selected-layout-template="selectedLayoutTemplate"
        :selected-table-detail="selectedViewTableDetail"
        :template-layout-cards="templateLayoutCards"
        :template-preview-bindings="templatePreviewBindings"
        :template-preview-loading="templatePreviewLoading"
        :template-preview-selected-item="templatePreviewSelectedItem"
        :template-preview-table-detail="templatePreviewTableDetail"
        :table-count="tableCount"
        :table-sections="tableSections"
        :on-reset-card-layout-override="onResetCardLayoutOverride"
        :on-reset-record-layout-overrides="onResetRecordLayoutOverrides"
        :on-save-layout-card-column-bindings="onSaveLayoutCardColumnBindings"
        :on-save-layout-template-cards="onSaveLayoutTemplateCards"
        :on-save-record-layout-overrides="onSaveRecordLayoutOverrides"
        :on-select-template-preview-record="onSelectTemplatePreviewRecord"
      />

      <div v-else-if="currentMode === 'master'" class="master-workspace">
        <OptionGroupManagerPanel
          v-if="selectedMasterSection === 'options'"
          :bootstrap="bootstrap"
          :option-group-form="optionGroupForm"
          :selected-option-group-id="selectedOptionGroupId"
          :on-add-option-row="addOptionRow"
          :on-remove-option-row="onRemoveOptionRow"
          :on-reset-option-group-form="onResetOptionGroupForm"
          :on-start-edit-option-group="onStartEditOptionGroup"
          :on-submit-option-group="onSubmitOptionGroup"
          :on-sync-option-ordering="onSyncOptionOrdering"
        />
        <TagManagerPanel
          v-else
          :groups="recordTagGroups"
          :tags="recordTags"
          :on-save-tag-group="onSaveRecordTagGroup"
          :on-delete-tag-group="onDeleteRecordTagGroup"
          :on-save-tag="onSaveRecordTag"
          :on-delete-tag="onDeleteRecordTag"
          :on-attach-tag-group="onAttachRecordTagGroup"
          :on-detach-tag-group="onDetachRecordTagGroup"
        />
      </div>
    </template>

    <v-card
      v-else-if="
        (currentMode === 'design' || currentMode === 'data') && tableCount === 0
      "
      class="empty-card"
      color="surface"
      variant="elevated"
      rounded="xl"
      elevation="2"
      border
    >
      <div class="table-onboarding-empty">
        <v-avatar size="64" color="primary" variant="tonal">
          <v-icon icon="mdi-table-plus" size="34" />
        </v-avatar>
        <div class="table-onboarding-copy">
          <h2>最初のテーブルを作成しましょう</h2>
          <p>
            テーブルは、管理したいデータの入れ物です。まずは「キャラクター」「アイテム」「取引先」など、集めたい情報の名前で作成します。作成後は設計モードでカラムを追加し、データモードでレコードを入力できます。
          </p>
        </div>
        <div class="table-onboarding-steps" aria-label="テーブル作成後の流れ">
          <v-sheet
            class="table-onboarding-step"
            color="surface"
            rounded="lg"
            border
          >
            <strong>1</strong>
            <span>テーブルを作成</span>
          </v-sheet>
          <v-sheet
            class="table-onboarding-step"
            color="surface"
            rounded="lg"
            border
          >
            <strong>2</strong>
            <span>設計モードでカラムを追加</span>
          </v-sheet>
          <v-sheet
            class="table-onboarding-step"
            color="surface"
            rounded="lg"
            border
          >
            <strong>3</strong>
            <span>データモードでレコードを入力</span>
          </v-sheet>
        </div>
        <v-btn
          color="primary"
          size="large"
          prepend-icon="mdi-table-plus"
          @click="onOpenTableDialog"
        >
          最初のテーブルを作成
        </v-btn>
      </div>
    </v-card>

    <v-card
      v-else
      class="empty-card"
      color="surface"
      variant="elevated"
      rounded="xl"
      elevation="2"
      border
    >
      <h2>テーブルを選択してください</h2>
      <p>
        左のテーブル一覧から対象を選ぶと、設計モードとデータモードを切り替えて編集できます。
      </p>
    </v-card>
  </main>
</template>
