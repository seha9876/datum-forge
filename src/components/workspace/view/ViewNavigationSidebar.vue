<script setup lang="ts">
import { ref } from "vue";

import ViewNavigationPanel from "./ViewNavigationPanel.vue";
import ViewTemplatePanel from "./ViewTemplatePanel.vue";

import type {
  ViewLayoutTemplate,
  ViewNavFolderRecord,
  ViewNavTreeNode,
  ViewSelection
} from "../../../types";

defineProps<{
  customTree: ViewNavTreeNode[];
  isFolderExpanded: (folderId: number) => boolean;
  layoutTemplates: ViewLayoutTemplate[];
  onCreateFolder: (parentId: number | null, name: string) => Promise<void>;
  onCreateLayoutTemplate: (name: string) => Promise<void>;
  onDeleteFolder: (node: ViewNavTreeNode) => Promise<void>;
  onDeleteLayoutTemplate: (templateId: number) => Promise<void>;
  onDuplicateLayoutTemplate: (
    templateId: number,
    name: string
  ) => Promise<void>;
  onRenameLayoutTemplate: (templateId: number, name: string) => Promise<void>;
  onReorderFolderRecords: (
    folderId: number,
    records: ViewNavFolderRecord[]
  ) => Promise<void>;
  onSelectFolder: (node: ViewNavTreeNode) => void;
  onSelectFolderRecord: (record: ViewNavFolderRecord) => void;
  onSelectLayoutTemplate: (template: ViewLayoutTemplate) => void;
  onToggleSidebar: () => void;
  onToggleFolder: (folderId: number) => void;
  rail: boolean;
  selectedLayoutTemplateId: number | null;
  selectedItem: ViewSelection | null;
  showRecordIds: boolean;
}>();

const activeTab = ref<"navigation" | "templates">("navigation");

const RAIL_TOGGLE_GLYPH = "›";
</script>

<template>
  <div
    v-if="rail"
    class="sidebar-content rail"
    aria-label="閲覧モードサイドバー"
  >
    <div class="sidebar-rail-shell">
      <div class="sidebar-rail-actions">
        <v-tooltip text="サイドバーを開く" location="right">
          <template #activator="{ props: tooltipProps }">
            <button
              v-bind="tooltipProps"
              type="button"
              class="sidebar-rail-toggle"
              aria-label="サイドバーを開く"
              @click="onToggleSidebar"
            >
              <span class="sidebar-rail-toggle-glyph" aria-hidden="true">
                {{ RAIL_TOGGLE_GLYPH }}
              </span>
            </button>
          </template>
        </v-tooltip>
      </div>

      <div class="sidebar-scroll-section rail">
        <v-list
          class="sidebar-list sidebar-list-rail"
          nav
          density="comfortable"
        >
          <v-tooltip text="目次" location="right">
            <template #activator="{ props: tooltipProps }">
              <v-list-item
                v-bind="tooltipProps"
                :active="activeTab === 'navigation'"
                prepend-icon="mdi-folder-outline"
                rounded="xl"
                class="sidebar-table-item rail"
                aria-label="目次"
                @click="activeTab = 'navigation'"
              />
            </template>
          </v-tooltip>

          <v-tooltip text="テンプレート" location="right">
            <template #activator="{ props: tooltipProps }">
              <v-list-item
                v-bind="tooltipProps"
                :active="activeTab === 'templates'"
                prepend-icon="mdi-view-dashboard-outline"
                rounded="xl"
                class="sidebar-table-item rail"
                aria-label="テンプレート"
                @click="activeTab = 'templates'"
              />
            </template>
          </v-tooltip>
        </v-list>
      </div>
    </div>
  </div>

  <div v-else class="view-nav-sidebar-shell">
    <div class="view-nav-sidebar-header">
      <v-btn-toggle
        v-model="activeTab"
        class="view-nav-toggle"
        color="primary"
        density="compact"
        mandatory
        variant="tonal"
      >
        <v-tooltip text="目次" location="bottom">
          <template #activator="{ props: tooltipProps }">
            <v-btn
              v-bind="tooltipProps"
              value="navigation"
              aria-label="目次"
              class="view-nav-toggle-btn"
            >
              <v-icon icon="mdi-folder-outline" />
              <span class="view-nav-toggle-label">目次</span>
            </v-btn>
          </template>
        </v-tooltip>

        <v-tooltip text="テンプレート" location="bottom">
          <template #activator="{ props: tooltipProps }">
            <v-btn
              v-bind="tooltipProps"
              value="templates"
              aria-label="テンプレート"
              class="view-nav-toggle-btn"
            >
              <v-icon icon="mdi-view-dashboard-outline" />
              <span class="view-nav-toggle-label">テンプレート</span>
            </v-btn>
          </template>
        </v-tooltip>
      </v-btn-toggle>

      <v-tooltip text="サイドバーを閉じる" location="bottom">
        <template #activator="{ props: tooltipProps }">
          <v-btn
            v-bind="tooltipProps"
            icon="mdi-chevron-left"
            variant="text"
            size="small"
            class="sidebar-collapse-btn view-nav-collapse-btn"
            aria-label="サイドバーを閉じる"
            @click="onToggleSidebar"
          />
        </template>
      </v-tooltip>
    </div>

    <ViewNavigationPanel
      v-if="activeTab === 'navigation'"
      :custom-tree="customTree"
      :is-folder-expanded="isFolderExpanded"
      :on-create-folder="onCreateFolder"
      :on-delete-folder="onDeleteFolder"
      :on-reorder-folder-records="onReorderFolderRecords"
      :on-select-folder-record="onSelectFolderRecord"
      :on-select-folder="onSelectFolder"
      :on-toggle-folder="onToggleFolder"
      :selected-item="selectedItem"
      :show-record-ids="showRecordIds"
    />

    <ViewTemplatePanel
      v-else
      :layout-templates="layoutTemplates"
      :on-create-layout-template="onCreateLayoutTemplate"
      :on-delete-layout-template="onDeleteLayoutTemplate"
      :on-duplicate-layout-template="onDuplicateLayoutTemplate"
      :on-rename-layout-template="onRenameLayoutTemplate"
      :on-select-layout-template="onSelectLayoutTemplate"
      :selected-layout-template-id="selectedLayoutTemplateId"
    />
  </div>
</template>
