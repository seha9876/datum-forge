<script setup lang="ts">
import { computed, ref } from "vue";

import { useConfirmDialog } from "../../../composables/useConfirmDialog";

import TagManagerDialogs from "./TagManagerDialogs.vue";
import {
  isTagInGroup,
  popularTagsFor,
  tagsForGroup,
  unclassifiedTagsFor,
  uniqueTagIdsInSections,
  type GroupFilter,
  type TagSection
} from "./TagManagerPanel.helpers";
import TagManagerTagList from "./TagManagerTagList.vue";
import { useTagDragManagement } from "./useTagDragManagement";
import { useTagSelection } from "./useTagSelection";

import type {
  RecordTag,
  RecordTagGroup,
  SaveRecordTagGroupPayload,
  SaveRecordTagPayload
} from "../../../types";

type TagDialogMode = "tagPicker" | "groupPicker";

/**
 * タグ管理パネルの親は、保存APIと状態の結線だけを担当します。
 * 一覧表示、ダイアログ、複数選択、ドラッグ操作は子コンポーネント/composableに逃がします。
 */
const props = defineProps<{
  groups: RecordTagGroup[];
  tags: RecordTag[];
  onSaveTagGroup: (payload: SaveRecordTagGroupPayload) => Promise<void>;
  onSaveTag: (payload: SaveRecordTagPayload) => Promise<void>;
  onDeleteTag: (tagId: number) => Promise<void>;
  onAttachTagGroup: (tagId: number, groupId: number) => Promise<void>;
  onDetachTagGroup: (tagId: number, groupId: number) => Promise<void>;
}>();

const confirmDialog = useConfirmDialog();

const selectedGroup = ref<GroupFilter>("all");
const isGroupDialogOpen = ref(false);
const groupNameInput = ref("");
const isTagDialogOpen = ref(false);
const tagDialogMode = ref<TagDialogMode>("tagPicker");
const tagDialogGroupFilter = ref<GroupFilter>("all");
const addTargetGroupId = ref<number | null>(null);
const contextTag = ref<RecordTag | null>(null);
const contextTagGroupId = ref<number | null>(null);
const searchKeyword = ref("");
const contextMenuOpen = ref(false);
const contextMenuTarget = ref<[number, number]>([0, 0]);
const isRenameDialogOpen = ref(false);
const renameInput = ref("");

const totalTagCount = computed(() => props.tags.length);
const allTagGroupCount = computed(() => props.groups.length);
const unclassifiedTags = computed(() => unclassifiedTagsFor(props.tags));
const popularTags = computed(() => popularTagsFor(props.tags));

function tagsInGroup(groupId: number) {
  return tagsForGroup(props.tags, groupId);
}

function countForGroup(groupId: number) {
  return tagsInGroup(groupId).length;
}

const selectedGroupObject = computed(() =>
  typeof selectedGroup.value === "number"
    ? props.groups.find((group) => group.id === selectedGroup.value)
    : null
);

const visibleSections = computed<TagSection[]>(() => {
  // 「すべて」ではタグが複数グループに出るため、選択状態は visibleTagIds 側で重複排除します。
  if (selectedGroup.value === "all") {
    const groupSections = props.groups.map((group) => {
      const tags = tagsInGroup(group.id);
      return {
        key: `group-${group.id}`,
        title: group.name,
        count: tags.length,
        tags,
        groupId: group.id
      };
    });
    return [
      ...groupSections,
      {
        key: "unclassified",
        title: "未分類",
        count: unclassifiedTags.value.length,
        tags: unclassifiedTags.value
      }
    ];
  }

  if (selectedGroup.value === "unclassified") {
    return [
      {
        key: "unclassified",
        title: "未分類",
        count: unclassifiedTags.value.length,
        tags: unclassifiedTags.value
      }
    ];
  }

  if (selectedGroup.value === "popular") {
    return [
      {
        key: "popular",
        title: "よく使うタグ",
        count: popularTags.value.length,
        tags: popularTags.value
      }
    ];
  }

  const group = selectedGroupObject.value;
  if (!group) {
    return [];
  }
  const tags = tagsInGroup(group.id);
  return [
    {
      key: `group-${group.id}`,
      title: group.name,
      count: tags.length,
      tags,
      groupId: group.id
    }
  ];
});

const visibleTagIds = computed(() =>
  uniqueTagIdsInSections(visibleSections.value)
);

const {
  isTagSelected,
  selectTagFromPointerCandidate,
  selectedTags,
  singleSelectedTag
} = useTagSelection({
  selectedGroup,
  tags: () => props.tags,
  visibleTagIds: () => visibleTagIds.value
});

const {
  dragGhost,
  dragGhostElement,
  dragOverGroupId,
  draggingTag,
  isTagDragging,
  preparePointerDrag
} = useTagDragManagement({
  attachTagsToGroup,
  isTagSelected,
  selectedTags: () => selectedTags.value,
  selectTagFromPointerCandidate
});

const detachTargetGroupId = computed(() => {
  if (typeof selectedGroup.value === "number") {
    return selectedGroup.value;
  }
  return contextTagGroupId.value;
});

const canDetachContextTagFromGroup = computed(
  () =>
    contextTag.value !== null &&
    detachTargetGroupId.value !== null &&
    isTagInGroup(contextTag.value, detachTargetGroupId.value)
);

const mainTitle = computed(() => {
  if (selectedGroup.value === "all") {
    return `すべて（${totalTagCount.value}）`;
  }
  return visibleSections.value[0]?.title
    ? `${visibleSections.value[0].title}（${visibleSections.value[0].count}）`
    : "タグ";
});

const canAddToCurrentGroup = computed(
  () => typeof selectedGroup.value === "number"
);

const filteredDialogGroups = computed(() => {
  if (tagDialogMode.value !== "groupPicker") {
    return props.groups;
  }
  const keyword = searchKeyword.value.trim().toLocaleLowerCase();
  if (!keyword) {
    return props.groups;
  }
  return props.groups.filter((group) =>
    group.name.toLocaleLowerCase().includes(keyword)
  );
});

const filteredDialogTags = computed(() => {
  const keyword = searchKeyword.value.trim().toLocaleLowerCase();
  let source = props.tags;

  if (tagDialogGroupFilter.value === "unclassified") {
    source = unclassifiedTags.value;
  } else if (tagDialogGroupFilter.value === "popular") {
    source = popularTags.value;
  } else if (typeof tagDialogGroupFilter.value === "number") {
    source = tagsInGroup(tagDialogGroupFilter.value);
  }

  if (!keyword) {
    return source;
  }
  return source.filter((tag) => tag.name.toLocaleLowerCase().includes(keyword));
});

const filteredDialogSections = computed<TagSection[]>(() => {
  if (tagDialogMode.value === "groupPicker" && contextTag.value) {
    return [
      {
        key: "selected-tag",
        title: "追加するタグ",
        count: 1,
        tags: [contextTag.value]
      }
    ];
  }

  if (tagDialogGroupFilter.value !== "all") {
    return [
      {
        key: String(tagDialogGroupFilter.value),
        title:
          tagDialogGroupFilter.value === "unclassified"
            ? "未分類"
            : tagDialogGroupFilter.value === "popular"
              ? "よく使うタグ"
              : (props.groups.find(
                  (group) => group.id === tagDialogGroupFilter.value
                )?.name ?? "タグ"),
        count: filteredDialogTags.value.length,
        tags: filteredDialogTags.value
      }
    ];
  }

  const keyword = searchKeyword.value.trim().toLocaleLowerCase();
  return props.groups
    .map((group) => {
      const tags = tagsInGroup(group.id).filter(
        (tag) => !keyword || tag.name.toLocaleLowerCase().includes(keyword)
      );
      return {
        key: `dialog-group-${group.id}`,
        title: group.name,
        count: tags.length,
        tags
      };
    })
    .filter((section) => section.tags.length > 0);
});

function openGroupDialog() {
  groupNameInput.value = "";
  isGroupDialogOpen.value = true;
}

async function createGroup() {
  const name = groupNameInput.value.trim();
  if (!name) {
    return;
  }
  await props.onSaveTagGroup({ name });
  groupNameInput.value = "";
  isGroupDialogOpen.value = false;
}

function openAddTagDialog(groupId: number) {
  tagDialogMode.value = "tagPicker";
  addTargetGroupId.value = groupId;
  tagDialogGroupFilter.value = "all";
  searchKeyword.value = "";
  isTagDialogOpen.value = true;
}

function openGroupPickerDialog(tag: RecordTag) {
  tagDialogMode.value = "groupPicker";
  contextTag.value = tag;
  addTargetGroupId.value = null;
  tagDialogGroupFilter.value = "all";
  searchKeyword.value = "";
  isTagDialogOpen.value = true;
}

async function addTagToGroup(tag: RecordTag) {
  if (tagDialogMode.value !== "tagPicker" || addTargetGroupId.value === null) {
    return;
  }
  if (!isTagInGroup(tag, addTargetGroupId.value)) {
    await props.onAttachTagGroup(tag.id, addTargetGroupId.value);
  }
}

async function addContextTagToGroup(groupId: number) {
  if (!contextTag.value || isTagInGroup(contextTag.value, groupId)) {
    return;
  }
  await props.onAttachTagGroup(contextTag.value.id, groupId);
  isTagDialogOpen.value = false;
}

function shouldShowGroupMembershipCheck(groupId: number) {
  return (
    singleSelectedTag.value !== null &&
    isTagInGroup(singleSelectedTag.value, groupId)
  );
}

async function attachTagsToGroup(tags: RecordTag[], groupId: number) {
  for (const tag of tags) {
    if (!isTagInGroup(tag, groupId)) {
      await props.onAttachTagGroup(tag.id, groupId);
    }
  }
}

async function toggleSelectedTagsForGroup(groupId: number) {
  const tags = selectedTags.value;
  if (tags.length === 0) {
    // 選択タグがない場合のグループクリックは、一括操作ではなく表示切り替えとして扱います。
    selectedGroup.value = groupId;
    return;
  }

  const allTagsInGroup = tags.every((tag) => isTagInGroup(tag, groupId));
  if (allTagsInGroup) {
    for (const tag of tags) {
      await props.onDetachTagGroup(tag.id, groupId);
    }
    return;
  }

  await attachTagsToGroup(tags, groupId);
}

function openContextMenu(
  event: MouseEvent,
  tag: RecordTag,
  groupId: number | null = null
) {
  event.preventDefault();
  contextTag.value = tag;
  contextTagGroupId.value = groupId;
  contextMenuTarget.value = [event.clientX, event.clientY];
  contextMenuOpen.value = true;
}

function openRenameDialog() {
  if (!contextTag.value) {
    return;
  }
  renameInput.value = contextTag.value.name;
  contextMenuOpen.value = false;
  isRenameDialogOpen.value = true;
}

async function renameTag() {
  if (!contextTag.value) {
    return;
  }
  const name = renameInput.value.trim();
  if (!name) {
    return;
  }
  await props.onSaveTag({ id: contextTag.value.id, name });
  isRenameDialogOpen.value = false;
}

async function detachFromCurrentGroup() {
  const groupId = detachTargetGroupId.value;
  if (!contextTag.value || groupId === null) {
    return;
  }
  await props.onDetachTagGroup(contextTag.value.id, groupId);
  contextMenuOpen.value = false;
}

async function deleteTag() {
  if (!contextTag.value) {
    return;
  }
  const tag = contextTag.value;
  const confirmMessage =
    tag.usageCount === 0
      ? `「${tag.name}」を削除しますか？`
      : `「${tag.name}」は ${tag.usageCount} 件、使用されています。本当に削除しますか？`;
  const ok = await confirmDialog.open({
    title: "タグの削除",
    message: confirmMessage,
    confirmText: "削除",
    color: "error"
  });
  if (!ok) {
    return;
  }
  await props.onDeleteTag(tag.id);
  contextMenuOpen.value = false;
}
</script>

<template>
  <div class="tag-manager-grid">
    <TagManagerTagList
      v-model:selected-group="selectedGroup"
      :all-tag-group-count="allTagGroupCount"
      :can-add-to-current-group="canAddToCurrentGroup"
      :drag-ghost="dragGhost"
      :drag-over-group-id="dragOverGroupId"
      :dragging-tag="draggingTag"
      :group-tag-count="countForGroup"
      :groups="groups"
      :is-tag-dragging="isTagDragging"
      :is-tag-selected="isTagSelected"
      :main-title="mainTitle"
      :popular-tag-count="popularTags.length"
      :should-show-group-membership-check="shouldShowGroupMembershipCheck"
      :tag-sections="visibleSections"
      :total-tag-count="totalTagCount"
      :unclassified-tag-count="unclassifiedTags.length"
      @open-add-tag-dialog="openAddTagDialog"
      @open-context-menu="openContextMenu"
      @open-group-dialog="openGroupDialog"
      @prepare-pointer-drag="preparePointerDrag"
      @toggle-selected-tags-for-group="toggleSelectedTagsForGroup"
      @update:drag-ghost-element="dragGhostElement = $event"
    />

    <v-menu
      v-model="contextMenuOpen"
      :target="contextMenuTarget"
      location="end"
    >
      <v-list color="surface" density="compact">
        <v-list-item
          prepend-icon="mdi-pencil-outline"
          title="名前変更"
          @click="openRenameDialog"
        />
        <v-list-item
          prepend-icon="mdi-folder-plus-outline"
          title="グループに追加"
          @click="contextTag && openGroupPickerDialog(contextTag)"
        />
        <v-list-item
          :disabled="!canDetachContextTagFromGroup"
          prepend-icon="mdi-folder-remove-outline"
          title="グループから削除"
          @click="detachFromCurrentGroup"
        />
        <v-list-item
          base-color="error"
          prepend-icon="mdi-delete-outline"
          title="削除"
          @click="deleteTag"
        />
      </v-list>
    </v-menu>

    <TagManagerDialogs
      v-model:group-name-input="groupNameInput"
      v-model:is-group-dialog-open="isGroupDialogOpen"
      v-model:is-rename-dialog-open="isRenameDialogOpen"
      v-model:is-tag-dialog-open="isTagDialogOpen"
      v-model:rename-input="renameInput"
      v-model:search-keyword="searchKeyword"
      v-model:tag-dialog-group-filter="tagDialogGroupFilter"
      :context-tag="contextTag"
      :filtered-dialog-groups="filteredDialogGroups"
      :filtered-dialog-sections="filteredDialogSections"
      :is-tag-in-group="isTagInGroup"
      :tag-dialog-mode="tagDialogMode"
      @add-context-tag-to-group="addContextTagToGroup"
      @add-tag-to-group="addTagToGroup"
      @create-group="createGroup"
      @rename-tag="renameTag"
    />
  </div>
</template>
