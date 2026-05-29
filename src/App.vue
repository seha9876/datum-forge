<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";

import ConfirmDialog from "./components/ConfirmDialog.vue";
import DatabaseSetupPage from "./components/DatabaseSetupPage.vue";
import ModeWorkspaceShell from "./components/ModeWorkspaceShell.vue";
import SettingsPage from "./components/settings/SettingsPage.vue";
import TableCreateDialog from "./components/TableCreateDialog.vue";
import TableSidebar from "./components/TableSidebar.vue";
import MasterManagementSidebar from "./components/workspace/master/MasterManagementSidebar.vue";
import ViewNavigationSidebar from "./components/workspace/view/ViewNavigationSidebar.vue";
import WorkspaceHeader from "./components/WorkspaceHeader.vue";
import WorkspaceModeHelpDialog from "./components/WorkspaceModeHelpDialog.vue";
import {
  createConfirmDialog,
  provideConfirmDialog
} from "./composables/useConfirmDialog";
import { useDatumForge } from "./composables/useDatumForge";
import { useRecordTags } from "./composables/useRecordTags";
import { useViewNavigation } from "./composables/useViewNavigation";
import { useWorkspaceMode } from "./composables/useWorkspaceMode";

import type { WorkspaceMode } from "./composables/useWorkspaceMode";

type MasterSection = "options" | "tags";

/** テーブル未選択時にトップバーへ表示する補助文言です。 */
const EMPTY_TABLE_SUBTITLE = "テーブルを選択してください";
/** テーブルが1件もないときにトップバーへ表示する補助文言です。 */
const NO_TABLE_SUBTITLE = "最初のテーブルを作成してください";
/** マスタ管理時にトップバーへ表示する見出しです。 */
const MASTER_WORKSPACE_TITLE = "マスタ管理";
/** マスタ管理時にトップバーへ表示する補助文言です。 */
const MASTER_WORKSPACE_SUBTITLE = "共通マスタを管理";
/** サイドバーを表示したまま保てる最小幅です。 */
const SIDEBAR_MIN_WIDTH = 220;
/** 最小幅でさらに押し込んだときに閉じる判定幅です。 */
const SIDEBAR_COLLAPSE_WIDTH = 160;
/** サイドバーを広げられる最大幅です。 */
const SIDEBAR_MAX_WIDTH = 520;
/** サイドバーを閉じたときのレール幅です。 */
const SIDEBAR_RAIL_WIDTH = 72;
/** サイドバーの初期表示幅です。 */
const SIDEBAR_DEFAULT_WIDTH = 344;

const {
  addOptionRow,
  columnForm,
  deleteColumn,
  deleteRecord,
  deleteTable,
  editingRecordId,
  fieldTypes,
  fieldTypeLabel,
  fieldTypeMeta,
  inputType,
  optionGroupForm,
  recordValues,
  referenceChoices,
  reorderColumns,
  removeOptionRow,
  resetOptionGroupForm,
  resetRecordForm,
  selectedOptionGroupId,
  selectedTable,
  startEditOptionGroup,
  startEditRecord,
  store,
  submitColumn,
  submitOptionGroup,
  submitRecord,
  submitTable,
  syncOptionOrdering,
  tableForm,
  updateColumn,
  updateLabelColumn
} = useDatumForge();
const { currentMode } = useWorkspaceMode();
const {
  createFolder,
  customTree,
  deleteFolder,
  error: viewError,
  clearError: clearViewError,
  layoutCardItems,
  folderCount,
  isFolderExpanded,
  layoutSaving: viewLayoutSaving,
  layoutTemplates,
  loading: viewLoading,
  assignFolderLayoutTemplate,
  createFolderLayoutTemplate,
  createLayoutTemplate,
  deleteLayoutTemplate,
  duplicateLayoutTemplate,
  renameLayoutTemplate,
  resetCardLayoutOverride,
  resetRecordLayoutOverrides,
  saveLayoutCardColumnBindings,
  saveLayoutTemplateCards,
  saveRecordLayoutOverrides,
  selectedLayoutTemplate,
  templateLayoutCards,
  templatePreviewBindings,
  templatePreviewLoading,
  templatePreviewSelectedItem,
  templatePreviewTableDetail,
  selectedFolderActiveTemplateId,
  selectedFolderLayoutTemplates,
  selectedFolderRecords,
  selectedItem,
  selectedTableDetail: selectedViewTableDetail,
  selectFolder,
  selectFolderRecord,
  selectLayoutTemplate,
  selectTemplatePreviewRecord,
  clearTemplatePreview,
  tableSections,
  toggleFolder,
  addFolderRecords,
  removeFolderRecord,
  reorderFolderRecords,
  refresh: refreshViewNavigation
} = useViewNavigation();
const {
  attachExistingTag,
  attachTagGroup,
  clearRecordTags,
  createAndAttachTag,
  deleteTag,
  deleteTagGroup,
  detachTag,
  detachTagGroup,
  groups: recordTagGroups,
  loadRecordTags,
  refreshTags,
  saveTag,
  saveTagGroup,
  selectedRecordTags,
  tags: recordTags
} = useRecordTags();

const confirmDialog = createConfirmDialog();
provideConfirmDialog(confirmDialog);

/** サイドバーを省スペース表示するかどうかを表します。 */
const isSidebarRail = ref(false);
/** ドラッグで調整するサイドバー幅です。 */
const sidebarWidth = ref(SIDEBAR_DEFAULT_WIDTH);
/** レール解除時に復元する通常幅です。 */
const lastExpandedSidebarWidth = ref(SIDEBAR_DEFAULT_WIDTH);
/** テーブル作成ダイアログの開閉状態です。 */
const isTableDialogOpen = ref(false);
/** 現在モードのヘルプダイアログの開閉状態です。 */
const isModeHelpOpen = ref(false);
/** マスタ管理で選択中の管理カテゴリです。 */
const selectedMasterSection = ref<MasterSection>("options");
/** 設定画面を閉じたときに戻る通常画面です。 */
const previousNormalMode = ref<Exclude<WorkspaceMode, "settings">>("design");
/** ドラッグ開始時のサイドバー幅です。 */
const sidebarResizeStartWidth = ref(SIDEBAR_DEFAULT_WIDTH);
/** ドラッグ開始時のポインタ位置です。 */
const sidebarResizeStartX = ref(0);
/** 現在表示中テーブルの表示名をトップバー用に整形します。 */
const currentTableTitle = computed(
  () => selectedTable.value?.table.displayName ?? "Datum Forge"
);
/** 現在表示中テーブルの物理名、未選択時は案内文を返します。 */
const currentTableSubtitle = computed(() => {
  if (selectedTable.value) {
    return selectedTable.value.table.tableName;
  }
  return store.bootstrap?.tables.length === 0
    ? NO_TABLE_SUBTITLE
    : EMPTY_TABLE_SUBTITLE;
});
/** 閲覧モード本文で表示するテーブル総数です。 */
const viewTableCount = computed(() => store.bootstrap?.tables.length ?? 0);
/** 設定画面では上部バーの見出しを固定します。 */
const workspaceTitle = computed(() =>
  currentMode.value === "settings"
    ? "設定"
    : currentMode.value === "master"
      ? MASTER_WORKSPACE_TITLE
      : currentTableTitle.value
);
/** 設定画面では上部バーの補助文言を固定します。 */
const workspaceSubtitle = computed(() =>
  currentMode.value === "settings"
    ? "アプリケーション設定"
    : currentMode.value === "master"
      ? MASTER_WORKSPACE_SUBTITLE
      : currentTableSubtitle.value
);
/** ヘルプ表示対象の通常モードです。 */
const modeHelpTarget = computed<Exclude<WorkspaceMode, "settings">>(() =>
  currentMode.value === "settings" ? previousNormalMode.value : currentMode.value
);
/** DBセットアップが終わるまでは通常ワークスペースを表示しません。 */
const isDatabaseSetupRequired = computed(
  () =>
    !!store.startupDbStatus && store.startupDbStatus.state !== "ready"
);
/** 起動判定中に表示する待機状態です。 */
const isCheckingDatabase = computed(
  () => !store.startupDbStatus && store.loading
);

/**
 * サイドバーの展開・折りたたみ状態を切り替えます。
 */
function toggleSidebar() {
  if (isSidebarRail.value) {
    isSidebarRail.value = false;
    sidebarWidth.value = lastExpandedSidebarWidth.value;
    return;
  }

  lastExpandedSidebarWidth.value = sidebarWidth.value;
  isSidebarRail.value = true;
}

/**
 * サイドバー幅のドラッグ調整を開始します。
 */
function startSidebarResize(event: globalThis.MouseEvent) {
  sidebarResizeStartWidth.value = isSidebarRail.value
    ? SIDEBAR_RAIL_WIDTH
    : sidebarWidth.value;
  sidebarResizeStartX.value = event.clientX;
  window.addEventListener("mousemove", handleSidebarResize);
  window.addEventListener("mouseup", stopSidebarResize);
  document.body.classList.add("is-resizing-sidebar");
}

/**
 * ドラッグ量に応じてサイドバー幅を更新します。
 */
function handleSidebarResize(event: globalThis.MouseEvent) {
  const nextWidth =
    sidebarResizeStartWidth.value + (event.clientX - sidebarResizeStartX.value);

  if (nextWidth <= SIDEBAR_COLLAPSE_WIDTH) {
    isSidebarRail.value = true;
    return;
  }

  isSidebarRail.value = false;
  sidebarWidth.value = Math.min(
    SIDEBAR_MAX_WIDTH,
    Math.max(SIDEBAR_MIN_WIDTH, nextWidth)
  );
  lastExpandedSidebarWidth.value = sidebarWidth.value;
}

/**
 * サイドバー幅調整を終了します。
 */
function stopSidebarResize() {
  window.removeEventListener("mousemove", handleSidebarResize);
  window.removeEventListener("mouseup", stopSidebarResize);
  document.body.classList.remove("is-resizing-sidebar");
}

/**
 * テーブル作成ダイアログを開きます。
 */
function openTableDialog() {
  isTableDialogOpen.value = true;
}

/**
 * 現在モードのヘルプダイアログを開きます。
 */
function openModeHelp() {
  if (currentMode.value === "settings") {
    return;
  }

  isModeHelpOpen.value = true;
}

function closeSettingsPage() {
  currentMode.value = previousNormalMode.value;
}

function toggleSettingsPage() {
  if (currentMode.value === "settings") {
    closeSettingsPage();
    return;
  }

  previousNormalMode.value = currentMode.value;
  currentMode.value = "settings";
}

watch(currentMode, (mode) => {
  if (mode === "settings") {
    isModeHelpOpen.value = false;
  }
  if (mode === "view") {
    void refreshViewNavigation();
  }
  if (mode === "view" || mode === "master") {
    void refreshTags();
  }
});

watch(
  selectedItem,
  (item) => {
    if (item?.type === "tableRecord") {
      void loadRecordTags(item.tableId, item.recordId);
      return;
    }
    clearRecordTags();
  },
  { immediate: true }
);

onBeforeUnmount(() => {
  stopSidebarResize();
});

onMounted(() => {
  void refreshTags();
});
</script>

<template>
  <v-app class="app-root">
    <DatabaseSetupPage v-if="isDatabaseSetupRequired" />

    <main v-else-if="isCheckingDatabase" class="database-setup-page">
      <section class="database-setup-shell database-setup-loading">
        <v-progress-circular indeterminate color="primary" />
        <p>DB設定を確認しています...</p>
      </section>
    </main>

    <v-layout v-else class="app-layout">
      <!-- テーブル一覧と画面遷移の起点になるサイドバー領域です。 -->
      <v-navigation-drawer
        v-if="currentMode !== 'settings'"
        permanent
        location="start"
        :rail="isSidebarRail"
        :rail-width="SIDEBAR_RAIL_WIDTH"
        :width="sidebarWidth"
        class="app-drawer"
      >
        <div class="app-sidebar-inner">
          <ViewNavigationSidebar
            v-if="currentMode === 'view'"
            :custom-tree="customTree"
            :is-folder-expanded="isFolderExpanded"
            :on-create-folder="createFolder"
            :on-create-layout-template="createLayoutTemplate"
            :on-delete-layout-template="deleteLayoutTemplate"
            :on-delete-folder="deleteFolder"
            :on-duplicate-layout-template="duplicateLayoutTemplate"
            :on-rename-layout-template="renameLayoutTemplate"
            :on-reorder-folder-records="reorderFolderRecords"
            :on-select-folder-record="selectFolderRecord"
            :on-select-folder="selectFolder"
            :on-select-layout-template="selectLayoutTemplate"
            :on-toggle-sidebar="toggleSidebar"
            :on-toggle-folder="toggleFolder"
            :layout-templates="layoutTemplates"
            :rail="isSidebarRail"
            :selected-layout-template-id="selectedLayoutTemplate?.id ?? null"
            :selected-item="selectedItem"
            :show-record-ids="
              store.settings?.showRecordIdsInNavigation ?? true
            "
          />
          <MasterManagementSidebar
            v-else-if="currentMode === 'master'"
            v-model:selected-section="selectedMasterSection"
            :rail="isSidebarRail"
            :on-toggle-sidebar="toggleSidebar"
          />
          <TableSidebar
            v-else
            :bootstrap="store.bootstrap"
            :rail="isSidebarRail"
            :selected-table-id="store.selectedTableId"
            :on-delete-table="deleteTable"
            :on-load-table="store.loadTable"
            :on-open-create-dialog="openTableDialog"
            :on-toggle-sidebar="toggleSidebar"
          />
        </div>
        <v-tooltip text="左サイドバーの幅を調整" location="right">
          <template #activator="{ props: tooltipProps }">
            <button
              v-bind="tooltipProps"
              type="button"
              class="app-drawer-resize-handle"
              :class="{ rail: isSidebarRail }"
              aria-label="左サイドバーの幅を調整"
              @mousedown.prevent="startSidebarResize"
            />
          </template>
        </v-tooltip>
      </v-navigation-drawer>

      <v-main class="app-main">
        <div class="app-main-frame">
          <!-- 現在テーブル名とモード切り替えを表示する上部バーです。 -->
          <WorkspaceHeader
            v-model="currentMode"
            :table-title="workspaceTitle"
            :table-subtitle="workspaceSubtitle"
            :on-open-mode-help="openModeHelp"
            :on-open-settings="toggleSettingsPage"
          />

          <div class="app-main-body-scroll">
            <SettingsPage
              v-if="currentMode === 'settings'"
              :on-back="closeSettingsPage"
            />
            <div v-else class="workspace-shell">
              <!-- 設計・データ・マスタ管理の各ワークスペースを切り替えて表示します。 -->
              <ModeWorkspaceShell
                :add-option-row="addOptionRow"
                :bootstrap="store.bootstrap"
                :column-form="columnForm"
                :current-mode="currentMode"
                :editing-record-id="editingRecordId"
                :error="store.error"
                :layout-card-items="layoutCardItems"
                :field-types="fieldTypes"
                :field-type-label="fieldTypeLabel"
                :field-type-meta="fieldTypeMeta"
                :input-type="inputType"
                :option-group-form="optionGroupForm"
                :record-values="recordValues"
                :record-tag-groups="recordTagGroups"
                :record-tags="recordTags"
                :reference-choices="referenceChoices"
                :selected-folder-active-template-id="selectedFolderActiveTemplateId"
                :selected-folder-layout-templates="selectedFolderLayoutTemplates"
                :selected-folder-records="selectedFolderRecords"
                :selected-item="selectedItem"
                :selected-layout-template="selectedLayoutTemplate"
                :selected-option-group-id="selectedOptionGroupId"
                :selected-record-tags="selectedRecordTags"
                :selected-master-section="selectedMasterSection"
                :selected-table="selectedTable"
                :selected-view-table-detail="selectedViewTableDetail"
                :template-layout-cards="templateLayoutCards"
                :template-preview-bindings="templatePreviewBindings"
                :template-preview-loading="templatePreviewLoading"
                :template-preview-selected-item="templatePreviewSelectedItem"
                :template-preview-table-detail="templatePreviewTableDetail"
                :table-sections="tableSections"
                :table-count="viewTableCount"
                :view-error="viewError"
                :on-clear-view-error="clearViewError"
                :on-open-mode-help="openModeHelp"
                :on-open-table-dialog="openTableDialog"
                :view-folder-count="folderCount"
                :view-layout-saving="viewLayoutSaving"
                :view-loading="viewLoading"
                :on-assign-folder-layout-template="assignFolderLayoutTemplate"
                :on-delete-column="deleteColumn"
                :on-delete-record="deleteRecord"
                :on-remove-option-row="removeOptionRow"
                :on-reorder-columns="reorderColumns"
                :on-reset-option-group-form="resetOptionGroupForm"
                :on-reset-record-form="resetRecordForm"
                :on-start-edit-option-group="startEditOptionGroup"
                :on-start-edit-record="startEditRecord"
                :on-submit-column="submitColumn"
                :on-submit-option-group="submitOptionGroup"
                :on-submit-record="submitRecord"
                :on-sync-option-ordering="syncOptionOrdering"
                :on-save-record-tag-group="saveTagGroup"
                :on-delete-record-tag-group="deleteTagGroup"
                :on-save-record-tag="saveTag"
                :on-delete-record-tag="deleteTag"
                :on-attach-record-tag-group="attachTagGroup"
                :on-detach-record-tag-group="detachTagGroup"
                :on-create-folder="createFolder"
                :on-create-folder-layout-template="createFolderLayoutTemplate"
                :on-clear-template-preview="clearTemplatePreview"
                :on-select-template-preview-record="selectTemplatePreviewRecord"
                :on-save-layout-card-column-bindings="saveLayoutCardColumnBindings"
                :on-save-layout-template-cards="saveLayoutTemplateCards"
                :on-save-record-layout-overrides="saveRecordLayoutOverrides"
                :on-add-folder-records="addFolderRecords"
                :on-attach-existing-tag="attachExistingTag"
                :on-create-and-attach-tag="createAndAttachTag"
                :on-detach-tag="detachTag"
                :on-remove-folder-record="removeFolderRecord"
                :on-reset-card-layout-override="resetCardLayoutOverride"
                :on-reset-record-layout-overrides="resetRecordLayoutOverrides"
                :on-update-column="updateColumn"
                :on-update-label-column="updateLabelColumn"
              />
            </div>
          </div>
        </div>
      </v-main>
    </v-layout>

    <!-- 新規テーブル作成用のモーダルダイアログです。 -->
    <TableCreateDialog
      v-model="isTableDialogOpen"
      :table-form="tableForm"
      :on-submit-table="submitTable"
    />
    <WorkspaceModeHelpDialog
      v-model="isModeHelpOpen"
      :folder-count="folderCount"
      :mode="modeHelpTarget"
      :on-create-folder="createFolder"
      :on-create-folder-layout-template="createFolderLayoutTemplate"
      :table-count="viewTableCount"
    />
    <ConfirmDialog :controller="confirmDialog" />
  </v-app>
</template>
