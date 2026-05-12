<script setup lang="ts">
import { ref } from "vue";

import ManualNavTree from "./ManualNavTree.vue";

import type {
  ViewNavFolderRecord,
  ViewNavTreeNode,
  ViewSelection
} from "../../../types";

defineProps<{
  customTree: ViewNavTreeNode[];
  isFolderExpanded: (folderId: number) => boolean;
  onCreateFolder: (parentId: number | null, name: string) => Promise<void>;
  onDeleteFolder: (node: ViewNavTreeNode) => Promise<void>;
  onSelectFolder: (node: ViewNavTreeNode) => void;
  onSelectFolderRecord: (record: ViewNavFolderRecord) => void;
  onToggleFolder: (folderId: number) => void;
  selectedItem: ViewSelection | null;
  showRecordIds: boolean;
}>();

const navigationSearchQuery = ref("");
</script>

<template>
  <v-card
    tag="section"
    color="surface"
    variant="elevated"
    rounded="xl"
    elevation="2"
    border
    class="view-sidebar-panel"
  >
    <div>
      <v-text-field
        v-model="navigationSearchQuery"
        type="search"
        prepend-inner-icon="mdi-magnify"
        placeholder="フォルダ・レコードを検索"
        aria-label="閲覧目次を検索"
        density="compact"
        variant="outlined"
        clearable
        hide-details
      />
    </div>

    <div>
      <ManualNavTree
        :is-expanded="isFolderExpanded"
        :nodes="customTree"
        :on-create-folder="onCreateFolder"
        :on-delete-folder="onDeleteFolder"
        :on-select-folder-record="onSelectFolderRecord"
        :on-select-folder="onSelectFolder"
        :on-toggle-folder="onToggleFolder"
        :search-query="navigationSearchQuery"
        :selected-item="selectedItem"
        :show-record-ids="showRecordIds"
      />
    </div>
  </v-card>
</template>
