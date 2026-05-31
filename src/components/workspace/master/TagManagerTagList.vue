<script setup lang="ts">
import type { GroupFilter, TagSection } from "./TagManagerPanel.helpers";
import type {
  DragGhostState,
  PointerDragStartEvent
} from "./useTagDragManagement";
import type { RecordTag, RecordTagGroup } from "../../../types";

/**
 * タグ管理の表示専用コンポーネントです。
 * 保存や選択の判断は親へemitし、ここでは一覧UIとユーザー操作の入口だけを持ちます。
 */
const props = defineProps<{
  allTagGroupCount: number;
  canAddToCurrentGroup: boolean;
  dragGhost: DragGhostState | null;
  dragOverGroupId: number | null;
  draggingTag: RecordTag | null;
  groups: RecordTagGroup[];
  groupTagCount: (groupId: number) => number;
  isTagDragging: (tagId: number) => boolean;
  isTagSelected: (tagId: number) => boolean;
  mainTitle: string;
  popularTagCount: number;
  selectedGroup: GroupFilter;
  shouldShowGroupMembershipCheck: (groupId: number) => boolean;
  tagSections: TagSection[];
  totalTagCount: number;
  unclassifiedTagCount: number;
}>();

const emit = defineEmits<{
  openAddTagDialog: [groupId: number];
  openContextMenu: [event: MouseEvent, tag: RecordTag, groupId: number | null];
  openGroupDialog: [];
  preparePointerDrag: [event: PointerDragStartEvent, tag: RecordTag];
  toggleSelectedTagsForGroup: [groupId: number];
  "update:dragGhostElement": [
    element: { style: { left: string; top: string } } | null
  ];
  "update:selectedGroup": [value: GroupFilter];
}>();

function selectGroup(value: GroupFilter) {
  emit("update:selectedGroup", value);
}

function openAddToCurrentGroup() {
  if (typeof props.selectedGroup === "number") {
    emit("openAddTagDialog", props.selectedGroup);
  }
}

function setDragGhostElement(element: unknown) {
  // ドラッグゴーストの座標更新だけは高頻度なので、親のcomposableへDOM参照を渡します。
  emit(
    "update:dragGhostElement",
    element instanceof HTMLElement ? element : null
  );
}
</script>

<template>
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
        @click="selectGroup('all')"
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
        @click="selectGroup('unclassified')"
      >
        <template #append>
          <span class="tag-manager-count">{{ unclassifiedTagCount }}</span>
        </template>
      </v-list-item>
      <v-list-item
        :active="selectedGroup === 'popular'"
        color="primary"
        prepend-icon="mdi-star-outline"
        title="よく使うタグ"
        @click="selectGroup('popular')"
      >
        <template #append>
          <span class="tag-manager-count">{{ popularTagCount }}</span>
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
            @click="emit('openGroupDialog')"
          />
        </template>
      </v-tooltip>
    </div>

    <v-list class="tag-manager-nav tag-manager-group-nav" density="compact" nav>
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
          @click="emit('toggleSelectedTagsForGroup', group.id)"
        >
          <template #append>
            <v-icon
              v-if="shouldShowGroupMembershipCheck(group.id)"
              color="primary"
              icon="mdi-check"
              size="16"
            />
            <span class="tag-manager-count">{{ groupTagCount(group.id) }}</span>
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
            @click="openAddToCurrentGroup"
          />
        </template>
      </v-tooltip>
    </div>

    <div class="tag-section-list">
      <section
        v-for="section in tagSections"
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
                emit('openContextMenu', $event, tag, section.groupId ?? null)
              "
              @pointerdown="emit('preparePointerDrag', $event, tag)"
            >
              {{ tag.name }}（{{ tag.usageCount }}）
            </v-chip>
          </v-hover>
        </div>
        <p v-else class="help-text">タグがありません。</p>
      </section>
    </div>
  </v-card>

  <div
    v-if="dragGhost"
    :ref="setDragGhostElement"
    class="tag-manager-drag-ghost"
  >
    {{ dragGhost.label }}
  </div>
</template>
