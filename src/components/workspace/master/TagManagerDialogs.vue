<script setup lang="ts">
import { computed } from "vue";

import type { GroupFilter, TagSection } from "./TagManagerPanel.helpers";
import type { RecordTag, RecordTagGroup } from "../../../types";

/**
 * タグ管理で使うダイアログ群をまとめた表示コンポーネントです。
 * v-modelの状態名は親に残し、保存・追加の判断も親へemitして副作用を分離します。
 */
const props = defineProps<{
  contextTag: RecordTag | null;
  filteredDialogGroups: RecordTagGroup[];
  filteredDialogSections: TagSection[];
  groupNameInput: string;
  isGroupDialogOpen: boolean;
  isRenameDialogOpen: boolean;
  isTagDialogOpen: boolean;
  isTagInGroup: (tag: RecordTag, groupId: number) => boolean;
  renameInput: string;
  searchKeyword: string;
  tagDialogGroupFilter: GroupFilter;
  tagDialogMode: "tagPicker" | "groupPicker";
}>();

const emit = defineEmits<{
  addContextTagToGroup: [groupId: number];
  addTagToGroup: [tag: RecordTag];
  createGroup: [];
  renameTag: [];
  "update:groupNameInput": [value: string];
  "update:isGroupDialogOpen": [value: boolean];
  "update:isRenameDialogOpen": [value: boolean];
  "update:isTagDialogOpen": [value: boolean];
  "update:renameInput": [value: string];
  "update:searchKeyword": [value: string];
  "update:tagDialogGroupFilter": [value: GroupFilter];
}>();

const groupDialogModel = computed({
  get: () => props.isGroupDialogOpen,
  set: (value) => emit("update:isGroupDialogOpen", value)
});

const renameDialogModel = computed({
  get: () => props.isRenameDialogOpen,
  set: (value) => emit("update:isRenameDialogOpen", value)
});

const tagDialogModel = computed({
  get: () => props.isTagDialogOpen,
  set: (value) => emit("update:isTagDialogOpen", value)
});

const groupNameModel = computed({
  get: () => props.groupNameInput,
  set: (value) => emit("update:groupNameInput", value)
});

const renameInputModel = computed({
  get: () => props.renameInput,
  set: (value) => emit("update:renameInput", value)
});

const searchKeywordModel = computed({
  get: () => props.searchKeyword,
  set: (value) => emit("update:searchKeyword", value)
});

const tagDialogGroupFilterModel = computed({
  get: () => props.tagDialogGroupFilter,
  set: (value) => emit("update:tagDialogGroupFilter", value)
});
</script>

<template>
  <v-dialog v-model="groupDialogModel" max-width="420">
    <v-card color="surface" rounded="lg">
      <v-card-title>タググループを追加</v-card-title>
      <v-card-text>
        <v-text-field
          v-model="groupNameModel"
          autofocus
          density="comfortable"
          hide-details
          label="グループ名"
          variant="outlined"
          @keydown.enter.prevent="emit('createGroup')"
        />
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn variant="text" @click="groupDialogModel = false"
          >キャンセル</v-btn
        >
        <v-btn color="primary" variant="flat" @click="emit('createGroup')"
          >追加</v-btn
        >
      </v-card-actions>
    </v-card>
  </v-dialog>

  <v-dialog v-model="renameDialogModel" max-width="420">
    <v-card color="surface" rounded="lg">
      <v-card-title>名前変更</v-card-title>
      <v-card-text>
        <v-text-field
          v-model="renameInputModel"
          autofocus
          density="comfortable"
          hide-details
          label="タグ名"
          variant="outlined"
          @keydown.enter.prevent="emit('renameTag')"
        />
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn variant="text" @click="renameDialogModel = false"
          >キャンセル</v-btn
        >
        <v-btn color="primary" variant="flat" @click="emit('renameTag')"
          >保存</v-btn
        >
      </v-card-actions>
    </v-card>
  </v-dialog>

  <v-dialog v-model="tagDialogModel" max-width="900">
    <v-card color="surface" rounded="lg" class="tag-search-dialog">
      <v-card-title>
        {{
          tagDialogMode === "tagPicker"
            ? "既存タグを追加"
            : `「${contextTag?.name ?? ""}」をグループに追加`
        }}
      </v-card-title>
      <v-card-text>
        <v-text-field
          v-model="searchKeywordModel"
          density="compact"
          hide-details
          :placeholder="
            tagDialogMode === 'groupPicker' ? 'グループを検索' : 'タグを検索'
          "
          prepend-inner-icon="mdi-magnify"
          variant="outlined"
        />

        <div class="tag-search-layout">
          <v-list class="tag-search-groups" density="compact" nav>
            <v-list-item
              v-if="tagDialogMode === 'tagPicker'"
              :active="tagDialogGroupFilter === 'all'"
              color="primary"
              prepend-icon="mdi-bookmark-outline"
              title="すべて"
              @click="tagDialogGroupFilterModel = 'all'"
            />
            <v-list-item
              v-if="tagDialogMode === 'tagPicker'"
              :active="tagDialogGroupFilter === 'unclassified'"
              color="primary"
              prepend-icon="mdi-folder-outline"
              title="未分類"
              @click="tagDialogGroupFilterModel = 'unclassified'"
            />
            <v-list-item
              v-for="group in filteredDialogGroups"
              :key="group.id"
              :active="tagDialogGroupFilter === group.id"
              color="primary"
              prepend-icon="mdi-bookmark-outline"
              :title="group.name"
              @click="
                tagDialogMode === 'groupPicker'
                  ? emit('addContextTagToGroup', group.id)
                  : (tagDialogGroupFilterModel = group.id)
              "
            >
              <template #append>
                <v-icon
                  v-if="
                    tagDialogMode === 'groupPicker' &&
                    contextTag &&
                    isTagInGroup(contextTag, group.id)
                  "
                  icon="mdi-check"
                  size="16"
                />
              </template>
            </v-list-item>
          </v-list>

          <div class="tag-search-results">
            <section
              v-for="section in filteredDialogSections"
              :key="section.key"
              class="tag-section"
            >
              <div class="tag-section-heading">
                <h3>{{ section.title }}（{{ section.count }}）</h3>
              </div>
              <div class="tag-chip-wrap">
                <v-chip
                  v-for="tag in section.tags"
                  :key="`dialog-${section.key}-${tag.id}`"
                  color="primary"
                  density="comfortable"
                  size="small"
                  variant="tonal"
                  @click="emit('addTagToGroup', tag)"
                >
                  {{ tag.name }}（{{ tag.usageCount }}）
                </v-chip>
              </div>
            </section>
            <p v-if="filteredDialogSections.length === 0" class="help-text">
              該当するタグがありません。
            </p>
          </div>
        </div>
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn variant="text" @click="tagDialogModel = false">閉じる</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>
