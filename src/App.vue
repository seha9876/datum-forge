<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";

import AppNotificationSnackbar from "./components/AppNotificationSnackbar.vue";
import ConfirmDialog from "./components/ConfirmDialog.vue";
import DatabaseSetupPage from "./components/DatabaseSetupPage.vue";
import ModeWorkspaceShell from "./components/ModeWorkspaceShell.vue";
import SettingsPage from "./components/settings/SettingsPage.vue";
import TableCreateDialog from "./components/TableCreateDialog.vue";
import TableSidebar from "./components/TableSidebar.vue";
import MasterManagementSidebar from "./components/workspace/master/MasterManagementSidebar.vue";
import ViewNavigationSidebar from "./components/workspace/view/ViewNavigationSidebar.vue";
import WorkspaceModeTabs from "./components/workspace/WorkspaceModeTabs.vue";
import WorkspaceModeHelpDialog from "./components/WorkspaceModeHelpDialog.vue";
import {
  createAppNotifications,
  provideAppNotifications
} from "./composables/useAppNotifications";
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
  exportTableCsv,
  fieldTypes,
  fieldTypeLabel,
  fieldTypeMeta,
  importExcelTable,
  importTableCsv,
  inspectExcelTables,
  inputType,
  optionGroupForm,
  recordValues,
  referenceChoices,
  reorderColumns,
  previewExcelTableImport,
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
  activeLayoutTemplateId,
  activeLayoutTemplateName,
  folderCount,
  isFolderExpanded,
  layoutSaving: viewLayoutSaving,
  layoutTemplates,
  loading: viewLoading,
  assignFolderLayoutTemplate,
  assignRecordLayoutTemplate,
  clearRecordLayoutTemplate,
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
const appNotifications = createAppNotifications();
provideAppNotifications(appNotifications);

/** サイドバーを省スペース表示するかどうかを表します。 */
const isSidebarRail = ref(false);
/** カスタムタイトルバーの最大化/復元アイコンを切り替えるための状態です。 */
const isWindowMaximized = ref(false);
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
/** 閲覧モード本文で表示するテーブル総数です。 */
const viewTableCount = computed(() => store.bootstrap?.tables.length ?? 0);
/** ヘルプ表示対象の通常モードです。 */
const modeHelpTarget = computed<Exclude<WorkspaceMode, "settings">>(() =>
  currentMode.value === "settings" ? previousNormalMode.value : currentMode.value
);
/** タイトルバーのヘルプメニューで、モードヘルプを選べる状態かを表します。 */
const canOpenModeHelp = computed(
  () =>
    currentMode.value !== "settings" &&
    !isDatabaseSetupRequired.value &&
    !isCheckingDatabase.value
);
/** タイトルバーのヘルプメニューで、設定画面へ移動できる状態かを表します。 */
const canToggleSettings = computed(
  () => !isDatabaseSetupRequired.value && !isCheckingDatabase.value
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
 * タイトルバーのボタンからサイドバーの通常幅/レール表示を切り替えます。
 * レール表示へ閉じる前に現在幅を覚えておき、再度開いたときに同じ幅へ戻します。
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
 * ウィンドウ移動を始めてはいけない操作部品かどうかを判定します。
 * タイトルバー全体をドラッグ領域にするとボタン操作も奪ってしまうため、
 * ボタンや入力欄などは明示的にドラッグ対象から外します。
 */
function shouldIgnoreWindowDrag(target: EventTarget | null) {
  if (!(target instanceof Element)) {
    return false;
  }

  return !!target.closest(
    "button, a, input, textarea, select, [role='button'], .app-window-no-drag"
  );
}

/**
 * カスタムタイトルバーの空白をドラッグしたときだけウィンドウ移動を開始します。
 */
async function startWindowDrag(event: PointerEvent) {
  if (
    event.button !== 0 ||
    event.detail > 1 ||
    shouldIgnoreWindowDrag(event.target)
  ) {
    return;
  }

  await getCurrentWindow().startDragging();
}

/**
 * 通常のWindowsタイトルバーに近づけるため、空白のダブルクリックで最大化/復元します。
 */
async function toggleMaximizeFromTitlebar(event: MouseEvent) {
  if (shouldIgnoreWindowDrag(event.target)) {
    return;
  }

  await toggleMaximizeWindow();
}

/**
 * 最大化状態を読み直し、最大化/復元ボタンのアイコンを現在状態に合わせます。
 */
async function refreshWindowMaximizedState() {
  isWindowMaximized.value = await getCurrentWindow().isMaximized();
}

/** ウィンドウ右端の最小化ボタンからTauriの最小化APIを呼びます。 */
async function minimizeWindow() {
  await getCurrentWindow().minimize();
}

/** 最大化/復元を切り替え、切り替え後の状態をアイコンへ反映します。 */
async function toggleMaximizeWindow() {
  await getCurrentWindow().toggleMaximize();
  await refreshWindowMaximizedState();
}

/** 標準タイトルバーを消しているため、閉じる操作もVue側からTauri APIへ渡します。 */
async function closeWindow() {
  await getCurrentWindow().close();
}

/**
 * サイドバー右端の細いバーを掴んだ時点の幅とポインター位置を記録します。
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
 * 一定幅より狭くなった場合は、通常幅ではなくレール表示へ切り替えます。
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
 * 幅調整中だけ登録したグローバルイベントを外し、通常カーソルに戻します。
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

/** 設定画面を閉じ、設定を開く前に表示していた通常モードへ戻ります。 */
function closeSettingsPage() {
  currentMode.value = previousNormalMode.value;
}

/** 右上の歯車から設定画面を開閉し、戻り先の通常モードを保持します。 */
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
  void refreshWindowMaximizedState();
});
</script>

<template>
  <v-app class="app-root">
    <div
      class="app-window-titlebar"
      @pointerdown="startWindowDrag"
      @dblclick="toggleMaximizeFromTitlebar"
    >
      <img
        class="app-window-icon"
        src="/app-icon.png"
        alt=""
        aria-hidden="true"
        draggable="false"
      />

      <v-tooltip
        :text="
          isSidebarRail ? 'サイドメニューを開く' : 'サイドメニューを閉じる'
        "
        location="bottom"
      >
        <template #activator="{ props: tooltipProps }">
          <v-btn
            v-bind="tooltipProps"
            class="app-window-no-drag"
            :icon="isSidebarRail ? 'mdi-menu-open' : 'mdi-menu'"
            variant="text"
            size="small"
            :disabled="
              currentMode === 'settings' ||
              isDatabaseSetupRequired ||
              isCheckingDatabase
            "
            :aria-label="
              isSidebarRail ? 'サイドメニューを開く' : 'サイドメニューを閉じる'
            "
            @click="toggleSidebar"
          />
        </template>
      </v-tooltip>

      <nav class="app-window-menu-items" aria-label="アプリケーションメニュー">
        <v-btn class="app-window-no-drag" variant="text" size="x-small">
          ファイル
        </v-btn>
        <v-btn class="app-window-no-drag" variant="text" size="x-small">
          編集
        </v-btn>
        <v-btn class="app-window-no-drag" variant="text" size="x-small">
          表示
        </v-btn>
        <v-btn class="app-window-no-drag" variant="text" size="x-small">
          ウィンドウ
        </v-btn>
        <v-menu location="bottom start" transition="fade-transition">
          <template #activator="{ props: menuProps }">
            <v-btn
              v-bind="menuProps"
              class="app-window-no-drag"
              variant="text"
              size="x-small"
            >
              ヘルプ
            </v-btn>
          </template>
          <v-list density="compact" min-width="180">
            <v-list-item
              prepend-icon="mdi-help-circle-outline"
              title="このモードのヘルプ"
              :disabled="!canOpenModeHelp"
              @click="openModeHelp"
            />
            <v-list-item
              prepend-icon="mdi-cog"
              title="設定"
              :active="currentMode === 'settings'"
              :disabled="!canToggleSettings"
              @click="toggleSettingsPage"
            />
          </v-list>
        </v-menu>
      </nav>

      <div class="app-window-mode-group" aria-label="ワークスペースモード">
        <WorkspaceModeTabs v-model="currentMode" class="app-window-mode-tabs" />
      </div>

      <div class="app-window-drag-spacer" aria-hidden="true" />

      <div class="app-window-controls" aria-label="ウィンドウ操作">
        <v-btn
          class="app-window-control app-window-no-drag"
          icon="mdi-window-minimize"
          variant="text"
          size="small"
          aria-label="最小化"
          @click="minimizeWindow"
        />
        <v-btn
          class="app-window-control app-window-no-drag"
          :icon="isWindowMaximized ? 'mdi-window-restore' : 'mdi-window-maximize'"
          variant="text"
          size="small"
          :aria-label="isWindowMaximized ? '元のサイズに戻す' : '最大化'"
          @click="toggleMaximizeWindow"
        />
        <v-btn
          class="app-window-control app-window-close app-window-no-drag"
          icon="mdi-window-close"
          variant="text"
          size="small"
          aria-label="閉じる"
          @click="closeWindow"
        />
      </div>
    </div>

    <DatabaseSetupPage v-if="isDatabaseSetupRequired" />

    <main v-else-if="isCheckingDatabase" class="database-setup-page">
      <section class="database-setup-shell database-setup-loading">
        <v-progress-circular indeterminate color="primary" />
        <p>DB設定を確認しています...</p>
      </section>
    </main>

    <template v-else>
      <v-layout class="app-layout">
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
          />
          <TableSidebar
            v-else
            :bootstrap="store.bootstrap"
            :rail="isSidebarRail"
            :selected-table-id="store.selectedTableId"
            :on-delete-table="deleteTable"
            :on-export-table-csv="exportTableCsv"
            :on-import-excel-table="importExcelTable"
            :on-import-table-csv="importTableCsv"
            :on-inspect-excel-tables="inspectExcelTables"
            :on-load-table="store.loadTable"
            :on-open-create-dialog="openTableDialog"
            :on-preview-excel-table-import="previewExcelTableImport"
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
          <div class="app-main-body-scroll">
            <SettingsPage
              v-if="currentMode === 'settings'"
              :on-back="closeSettingsPage"
            />
            <div v-else class="workspace-shell">
              <!-- タイトルバーで選んだモードに合わせて各ワークスペースを表示します。 -->
              <ModeWorkspaceShell
                :add-option-row="addOptionRow"
                :bootstrap="store.bootstrap"
                :column-form="columnForm"
                :current-mode="currentMode"
                :editing-record-id="editingRecordId"
                :error="store.error"
                :layout-card-items="layoutCardItems"
                :active-layout-template-id="activeLayoutTemplateId"
                :active-layout-template-name="activeLayoutTemplateName"
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
                :on-assign-record-layout-template="assignRecordLayoutTemplate"
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
                :on-save-record-tag="saveTag"
                :on-delete-record-tag="deleteTag"
                :on-attach-record-tag-group="attachTagGroup"
                :on-detach-record-tag-group="detachTagGroup"
                :on-create-folder="createFolder"
                :on-create-folder-layout-template="createFolderLayoutTemplate"
                :on-clear-template-preview="clearTemplatePreview"
                :on-clear-record-layout-template="clearRecordLayoutTemplate"
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
    </template>

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
    <AppNotificationSnackbar
      :controller="appNotifications"
      :notification-settings="store.settings?.notificationSettings"
    />
  </v-app>
</template>
