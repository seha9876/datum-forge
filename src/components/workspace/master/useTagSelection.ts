import { computed, ref, watch } from "vue";

import type { GroupFilter } from "./TagManagerPanel.helpers";
import type { RecordTag } from "../../../types";

export type TagPointerSelectionCandidate = {
  tag: RecordTag;
  ctrlKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
};

/**
 * タグの複数選択だけを管理します。
 * 表示中タグが変わったときに選択を剪定し、存在しないタグIDを後続操作へ渡さないようにします。
 */
export function useTagSelection(params: {
  selectedGroup: { value: GroupFilter };
  tags: () => RecordTag[];
  visibleTagIds: () => number[];
}) {
  const selectedTagIds = ref<number[]>([]);
  const lastSelectedTagId = ref<number | null>(null);

  const selectedTags = computed(() =>
    selectedTagIds.value
      .map((tagId) => params.tags().find((tag) => tag.id === tagId))
      .filter((tag): tag is RecordTag => tag !== undefined)
  );

  const singleSelectedTag = computed(() =>
    selectedTags.value.length === 1 ? selectedTags.value[0] : null
  );

  watch(
    () => params.tags().map((tag) => tag.id),
    (tagIds) => {
      pruneSelection(tagIds);
    }
  );

  watch(
    () => params.visibleTagIds(),
    (tagIds) => {
      // グループ切り替えや検索で見えなくなったタグは、一括操作の対象から外します。
      pruneSelection(tagIds);
    }
  );

  function isTagSelected(tagId: number) {
    return selectedTagIds.value.includes(tagId);
  }

  /** タグクリック時の単一選択、Ctrl/Meta 複数選択、Shift 範囲選択を処理します。 */
  function selectTagFromPointerCandidate(
    candidate: TagPointerSelectionCandidate
  ) {
    const tagId = candidate.tag.id;
    const visibleIds = params.visibleTagIds();

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

  function pruneSelection(allowedTagIds: number[]) {
    const allowedIds = new Set(allowedTagIds);
    selectedTagIds.value = selectedTagIds.value.filter((tagId) =>
      allowedIds.has(tagId)
    );
    if (
      lastSelectedTagId.value !== null &&
      !allowedIds.has(lastSelectedTagId.value)
    ) {
      lastSelectedTagId.value =
        selectedTagIds.value[selectedTagIds.value.length - 1] ?? null;
    }
  }

  return {
    isTagSelected,
    selectTagFromPointerCandidate,
    selectedTagIds,
    selectedTags,
    singleSelectedTag
  };
}
