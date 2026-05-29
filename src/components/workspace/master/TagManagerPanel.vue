<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";

import { useConfirmDialog } from "../../../composables/useConfirmDialog";

import type {
  RecordTag,
  RecordTagGroup,
  SaveRecordTagGroupPayload,
  SaveRecordTagPayload
} from "../../../types";

type GroupFilter = "all" | "unclassified" | "popular" | number;
type TagDialogMode = "tagPicker" | "groupPicker";
type ContextMenuEvent = {
  preventDefault: () => void;
  clientX: number;
  clientY: number;
};

/** タグチップ上でドラッグ開始候補を作るために使う pointerdown の最小イベント情報です。 */
type PointerDragStartEvent = {
  button: number;
  clientX: number;
  clientY: number;
  ctrlKey?: boolean;
  metaKey?: boolean;
  shiftKey?: boolean;
  preventDefault: () => void;
};

/** ドラッグ中のゴースト移動とドロップ判定に使う pointer イベント情報です。 */
type PointerDragMoveEvent = {
  clientX: number;
  clientY: number;
  preventDefault?: () => void;
};

/** まだドラッグ確定前のタグと押下開始位置を保持します。 */
type PointerDragCandidate = {
  tag: RecordTag;
  startX: number;
  startY: number;
  ctrlKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
};

/** マウスに追従して表示するタグの分身の表示状態です。 */
type DragGhostState = {
  label: string;
};

interface TagSection {
  key: string;
  title: string;
  count: number;
  tags: RecordTag[];
  groupId?: number;
}

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
const selectedTagIds = ref<number[]>([]);
const lastSelectedTagId = ref<number | null>(null);

// Pointer Drag の状態です。HTML5 Drag and Drop は使わず、止まれマークを出さないために自前で管理します。
const draggingTag = ref<RecordTag | null>(null);
const draggingTags = ref<RecordTag[]>([]);
const dragGhost = ref<DragGhostState | null>(null);
const dragGhostElement = ref<{ style: { left: string; top: string } } | null>(
  null
);
const dragOverGroupId = ref<number | null>(null);
const pointerDragStart = ref<PointerDragCandidate | null>(null);

const totalTagCount = computed(() => props.tags.length);
const allTagGroupCount = computed(() => props.groups.length);

function tagGroupIds(tag: RecordTag) {
  return tag.groupIds?.length
    ? tag.groupIds
    : tag.groupId === null
      ? []
      : [tag.groupId];
}

function isTagInGroup(tag: RecordTag, groupId: number) {
  return tagGroupIds(tag).includes(groupId);
}

const unclassifiedTags = computed(() =>
  props.tags.filter((tag) => tagGroupIds(tag).length === 0)
);

const popularTags = computed(() =>
  [...props.tags]
    .filter((tag) => tag.usageCount > 0)
    .sort((a, b) => b.usageCount - a.usageCount || a.name.localeCompare(b.name))
    .slice(0, 20)
);

function tagsForGroup(groupId: number) {
  return props.tags.filter((tag) => isTagInGroup(tag, groupId));
}

function countForGroup(groupId: number) {
  return tagsForGroup(groupId).length;
}

const selectedGroupObject = computed(() =>
  typeof selectedGroup.value === "number"
    ? props.groups.find((group) => group.id === selectedGroup.value)
    : null
);

const visibleSections = computed<TagSection[]>(() => {
  if (selectedGroup.value === "all") {
    const groupSections = props.groups.map((group) => {
      const tags = tagsForGroup(group.id);
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
  const tags = tagsForGroup(group.id);
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

const visibleTagIds = computed(() => {
  const seen = new Set<number>();
  const ids: number[] = [];
  for (const section of visibleSections.value) {
    for (const tag of section.tags) {
      if (!seen.has(tag.id)) {
        seen.add(tag.id);
        ids.push(tag.id);
      }
    }
  }
  return ids;
});

const selectedTags = computed(() =>
  selectedTagIds.value
    .map((tagId) => props.tags.find((tag) => tag.id === tagId))
    .filter((tag): tag is RecordTag => tag !== undefined)
);

const singleSelectedTag = computed(() =>
  selectedTags.value.length === 1 ? selectedTags.value[0] : null
);

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

watch(
  () => props.tags.map((tag) => tag.id),
  (tagIds) => {
    const existingIds = new Set(tagIds);
    selectedTagIds.value = selectedTagIds.value.filter((tagId) =>
      existingIds.has(tagId)
    );
    if (
      lastSelectedTagId.value !== null &&
      !existingIds.has(lastSelectedTagId.value)
    ) {
      lastSelectedTagId.value =
        selectedTagIds.value[selectedTagIds.value.length - 1] ?? null;
    }
  }
);

watch(visibleTagIds, (tagIds) => {
  const visibleIds = new Set(tagIds);
  selectedTagIds.value = selectedTagIds.value.filter((tagId) =>
    visibleIds.has(tagId)
  );
  if (
    lastSelectedTagId.value !== null &&
    !visibleIds.has(lastSelectedTagId.value)
  ) {
    lastSelectedTagId.value =
      selectedTagIds.value[selectedTagIds.value.length - 1] ?? null;
  }
});

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
    source = tagsForGroup(tagDialogGroupFilter.value);
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
      const tags = tagsForGroup(group.id).filter(
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

function isTagSelected(tagId: number) {
  return selectedTagIds.value.includes(tagId);
}

function isTagDragging(tagId: number) {
  return draggingTags.value.some((tag) => tag.id === tagId);
}

function shouldShowGroupMembershipCheck(groupId: number) {
  return (
    singleSelectedTag.value !== null &&
    isTagInGroup(singleSelectedTag.value, groupId)
  );
}

/** タグクリック時の単一選択、Ctrl/Meta 複数選択、Shift 範囲選択を処理します。 */
function selectTagFromPointerCandidate(candidate: PointerDragCandidate) {
  const tagId = candidate.tag.id;
  const visibleIds = visibleTagIds.value;

  if (
    candidate.shiftKey &&
    lastSelectedTagId.value !== null &&
    visibleIds.includes(lastSelectedTagId.value) &&
    visibleIds.includes(tagId)
  ) {
    const startIndex = visibleIds.indexOf(lastSelectedTagId.value);
    const endIndex = visibleIds.indexOf(tagId);
    const [from, to] =
      startIndex < endIndex ? [startIndex, endIndex] : [endIndex, startIndex];
    selectedTagIds.value = visibleIds.slice(from, to + 1);
    return;
  }

  if (candidate.ctrlKey || candidate.metaKey) {
    if (isTagSelected(tagId)) {
      selectedTagIds.value = selectedTagIds.value.filter(
        (selectedId) => selectedId !== tagId
      );
      lastSelectedTagId.value =
        selectedTagIds.value[selectedTagIds.value.length - 1] ?? null;
    } else {
      selectedTagIds.value = [...selectedTagIds.value, tagId];
      lastSelectedTagId.value = tagId;
    }
    return;
  }

  if (isTagSelected(tagId)) {
    selectedTagIds.value = selectedTagIds.value.filter(
      (selectedId) => selectedId !== tagId
    );
    lastSelectedTagId.value =
      selectedTagIds.value[selectedTagIds.value.length - 1] ?? null;
    return;
  }

  selectedTagIds.value = [tagId];
  lastSelectedTagId.value = tagId;
}

function getDragTargetTags(tag: RecordTag) {
  return selectedTagIds.value.includes(tag.id) && selectedTags.value.length > 0
    ? selectedTags.value
    : [tag];
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
  event: ContextMenuEvent,
  tag: RecordTag,
  groupId: number | null = null
) {
  event.preventDefault();
  contextTag.value = tag;
  contextTagGroupId.value = groupId;
  contextMenuTarget.value = [event.clientX, event.clientY];
  contextMenuOpen.value = true;
}

/** タグチップ上で左ボタンが押されたら、ドラッグ開始候補として記録します。 */
function preparePointerDrag(event: PointerDragStartEvent, tag: RecordTag) {
  if (event.button !== 0) {
    return;
  }
  // クリック操作との誤判定を避けるため、この時点ではまだドラッグ中にはしません。
  pointerDragStart.value = {
    tag,
    startX: event.clientX,
    startY: event.clientY,
    ctrlKey: event.ctrlKey === true,
    metaKey: event.metaKey === true,
    shiftKey: event.shiftKey === true
  };
  window.addEventListener("pointermove", handlePointerMove);
  window.addEventListener("pointerup", handlePointerUp);
  window.addEventListener("pointercancel", handlePointerCancel);
}

/** pointer イベント監視を解除し、ドラッグ候補と表示状態を初期化します。 */
function clearPointerDragState() {
  window.removeEventListener("pointermove", handlePointerMove);
  window.removeEventListener("pointerup", handlePointerUp);
  window.removeEventListener("pointercancel", handlePointerCancel);
  pointerDragStart.value = null;
  draggingTag.value = null;
  draggingTags.value = [];
  dragGhost.value = null;
  dragGhostElement.value = null;
  dragOverGroupId.value = null;
}

/** マウス座標の下にある通常タググループを、移動先候補として取得します。 */
function findDropGroupId(clientX: number, clientY: number) {
  const dropTarget = document
    .elementsFromPoint(clientX, clientY)
    .find(
      (element) =>
        element instanceof HTMLElement && element.dataset.tagDropGroupId
    );
  if (!(dropTarget instanceof HTMLElement)) {
    return null;
  }
  const groupId = Number(dropTarget.dataset.tagDropGroupId);
  return Number.isFinite(groupId) ? groupId : null;
}

/** ゴーストの座標だけを DOM へ直接反映し、pointermove 中の再描画を避けます。 */
function updateDragGhostPosition(clientX: number, clientY: number) {
  if (!dragGhostElement.value) {
    return;
  }
  dragGhostElement.value.style.left = `${clientX}px`;
  dragGhostElement.value.style.top = `${clientY}px`;
}

/** 一定距離以上動いたらドラッグを開始し、タグの分身をマウスへ追従させます。 */
function movePointerDrag(event: PointerDragMoveEvent) {
  const start = pointerDragStart.value;
  if (!start) {
    return;
  }

  const distance = Math.hypot(
    event.clientX - start.startX,
    event.clientY - start.startY
  );
  if (!draggingTag.value && distance < 4) {
    return;
  }

  // ドラッグが確定した後だけ既定の選択操作を止め、ゴーストを表示します。
  event.preventDefault?.();
  const targetTags = getDragTargetTags(start.tag);
  draggingTag.value = start.tag;
  draggingTags.value = targetTags;
  if (!dragGhost.value) {
    dragGhost.value = {
      label:
        targetTags.length === 1
          ? `${start.tag.name}（${start.tag.usageCount}）`
          : `${targetTags.length}件`
    };
    window.requestAnimationFrame(() =>
      updateDragGhostPosition(event.clientX, event.clientY)
    );
  } else {
    updateDragGhostPosition(event.clientX, event.clientY);
  }

  const nextGroupId = findDropGroupId(event.clientX, event.clientY);
  if (dragOverGroupId.value !== nextGroupId) {
    dragOverGroupId.value = nextGroupId;
  }
}

/** マウスを離した位置が通常タググループ上なら、タグの一括追加を確定します。 */
async function finishPointerDrag(event: PointerDragMoveEvent) {
  const start = pointerDragStart.value;
  const tags = draggingTags.value;
  const targetGroupId = findDropGroupId(event.clientX, event.clientY);

  if (tags.length > 0 && targetGroupId !== null) {
    await attachTagsToGroup(tags, targetGroupId);
  } else if (start && !draggingTag.value) {
    selectTagFromPointerCandidate(start);
  }

  clearPointerDragState();
}

/** window に登録する pointermove ハンドラです。タグの分身と移動先候補を更新します。 */
const handlePointerMove = (event: unknown) =>
  movePointerDrag(event as PointerDragMoveEvent);

/** window に登録する pointerup ハンドラです。非同期の移動確定処理を呼び出します。 */
const handlePointerUp = (event: unknown) =>
  void finishPointerDrag(event as PointerDragMoveEvent);

/** pointercancel 時にドラッグ表示とイベント監視を確実に解除します。 */
const handlePointerCancel = () => clearPointerDragState();

// コンポーネント破棄時に window へ登録した pointer イベントを残さないようにします。
onBeforeUnmount(() => clearPointerDragState());

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
    <v-card
      tag="section"
      color="surface"
      variant="elevated"
      rounded="xl"
      elevation="2"
      border
      class="tag-manager-side"
    >
      <v-list class="tag-manager-nav" density="compact" nav>
        <v-list-item
          :active="selectedGroup === 'all'"
          color="primary"
          prepend-icon="mdi-bookmark-outline"
          title="すべて"
          @click="selectedGroup = 'all'"
        >
          <template #append>
            <span class="tag-manager-count">{{ totalTagCount }}</span>
          </template>
        </v-list-item>
        <v-list-item
          :active="selectedGroup === 'unclassified'"
          color="primary"
          prepend-icon="mdi-folder-outline"
          title="未分類"
          @click="selectedGroup = 'unclassified'"
        >
          <template #append>
            <span class="tag-manager-count">{{ unclassifiedTags.length }}</span>
          </template>
        </v-list-item>
        <v-list-item
          :active="selectedGroup === 'popular'"
          color="primary"
          prepend-icon="mdi-star-outline"
          title="よく使うタグ"
          @click="selectedGroup = 'popular'"
        >
          <template #append>
            <span class="tag-manager-count">{{ popularTags.length }}</span>
          </template>
        </v-list-item>
      </v-list>

      <div class="tag-manager-group-head">
        <span>タググループ（{{ allTagGroupCount }}）</span>
        <v-tooltip text="タググループを追加" location="bottom">
          <template #activator="{ props: tooltipProps }">
            <v-btn
              v-bind="tooltipProps"
              icon="mdi-plus"
              size="x-small"
              variant="text"
              aria-label="タググループを追加"
              @click="openGroupDialog"
            />
          </template>
        </v-tooltip>
      </div>

      <v-list
        class="tag-manager-nav tag-manager-group-nav"
        density="compact"
        nav
      >
        <div
          v-for="group in groups"
          :key="group.id"
          :data-tag-drop-group-id="group.id"
          :class="{
            'tag-group-drop-target': draggingTag,
            'drag-over': dragOverGroupId === group.id
          }"
        >
          <v-list-item
            :active="selectedGroup === group.id"
            color="primary"
            prepend-icon="mdi-bookmark-outline"
            :title="group.name"
            @click="toggleSelectedTagsForGroup(group.id)"
          >
            <template #append>
              <v-icon
                v-if="shouldShowGroupMembershipCheck(group.id)"
                color="primary"
                icon="mdi-check"
                size="16"
              />
              <span class="tag-manager-count">{{
                countForGroup(group.id)
              }}</span>
            </template>
          </v-list-item>
        </div>
      </v-list>
    </v-card>

    <v-card
      tag="section"
      color="surface"
      variant="elevated"
      rounded="xl"
      elevation="2"
      border
      class="tag-manager-main"
    >
      <div class="tag-manager-main-head">
        <h2>{{ mainTitle }}</h2>
        <v-tooltip
          v-if="canAddToCurrentGroup"
          text="既存タグを追加"
          location="bottom"
        >
          <template #activator="{ props: tooltipProps }">
            <v-btn
              v-bind="tooltipProps"
              color="primary"
              icon="mdi-plus"
              size="small"
              variant="tonal"
              aria-label="既存タグを追加"
              @click="openAddTagDialog(selectedGroup as number)"
            />
          </template>
        </v-tooltip>
      </div>

      <div class="tag-section-list">
        <section
          v-for="section in visibleSections"
          :key="section.key"
          class="tag-section"
        >
          <div v-if="selectedGroup === 'all'" class="tag-section-heading">
            <h3>{{ section.title }}（{{ section.count }}）</h3>
          </div>

          <div v-if="section.tags.length > 0" class="tag-chip-wrap">
            <v-hover
              v-for="tag in section.tags"
              :key="`${section.key}-${tag.id}`"
              v-slot="{ isHovering, props: hoverProps }"
            >
              <v-chip
                v-bind="hoverProps"
                :class="[
                  'tag-manager-chip',
                  {
                    'tag-manager-chip-hovered': isHovering,
                    'tag-manager-chip-selected': isTagSelected(tag.id),
                    'tag-manager-chip-dragging': isTagDragging(tag.id)
                  }
                ]"
                color="primary"
                density="comfortable"
                :elevation="isHovering || isTagSelected(tag.id) ? 3 : 0"
                :prepend-icon="isTagSelected(tag.id) ? 'mdi-check' : undefined"
                size="small"
                :variant="
                  isHovering || isTagSelected(tag.id) ? 'elevated' : 'tonal'
                "
                @contextmenu="
                  openContextMenu($event, tag, section.groupId ?? null)
                "
                @pointerdown="preparePointerDrag($event, tag)"
              >
                {{ tag.name }}（{{ tag.usageCount }}）
              </v-chip>
            </v-hover>
          </div>
          <p v-else class="help-text">タグがありません。</p>
        </section>
      </div>
    </v-card>

    <div v-if="dragGhost" ref="dragGhostElement" class="tag-manager-drag-ghost">
      {{ dragGhost.label }}
    </div>

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

    <v-dialog v-model="isGroupDialogOpen" max-width="420">
      <v-card color="surface" rounded="lg">
        <v-card-title>タググループを追加</v-card-title>
        <v-card-text>
          <v-text-field
            v-model="groupNameInput"
            autofocus
            density="comfortable"
            hide-details
            label="グループ名"
            variant="outlined"
            @keydown.enter.prevent="createGroup"
          />
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="isGroupDialogOpen = false"
            >キャンセル</v-btn
          >
          <v-btn color="primary" variant="flat" @click="createGroup"
            >追加</v-btn
          >
        </v-card-actions>
      </v-card>
    </v-dialog>

    <v-dialog v-model="isRenameDialogOpen" max-width="420">
      <v-card color="surface" rounded="lg">
        <v-card-title>名前変更</v-card-title>
        <v-card-text>
          <v-text-field
            v-model="renameInput"
            autofocus
            density="comfortable"
            hide-details
            label="タグ名"
            variant="outlined"
            @keydown.enter.prevent="renameTag"
          />
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="isRenameDialogOpen = false"
            >キャンセル</v-btn
          >
          <v-btn color="primary" variant="flat" @click="renameTag">保存</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <v-dialog v-model="isTagDialogOpen" max-width="900">
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
            v-model="searchKeyword"
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
                @click="tagDialogGroupFilter = 'all'"
              />
              <v-list-item
                v-if="tagDialogMode === 'tagPicker'"
                :active="tagDialogGroupFilter === 'unclassified'"
                color="primary"
                prepend-icon="mdi-folder-outline"
                title="未分類"
                @click="tagDialogGroupFilter = 'unclassified'"
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
                    ? addContextTagToGroup(group.id)
                    : (tagDialogGroupFilter = group.id)
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
                    @click="addTagToGroup(tag)"
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
          <v-btn variant="text" @click="isTagDialogOpen = false">閉じる</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>
