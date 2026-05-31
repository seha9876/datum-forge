import { onBeforeUnmount, ref } from "vue";

import type { TagPointerSelectionCandidate } from "./useTagSelection";
import type { RecordTag } from "../../../types";

export type PointerDragStartEvent = {
  button: number;
  clientX: number;
  clientY: number;
  ctrlKey?: boolean;
  metaKey?: boolean;
  shiftKey?: boolean;
  preventDefault: () => void;
};

export type PointerDragMoveEvent = {
  clientX: number;
  clientY: number;
  preventDefault?: () => void;
};

type PointerDragCandidate = TagPointerSelectionCandidate & {
  startX: number;
  startY: number;
};

export type DragGhostState = {
  label: string;
};

/**
 * タグのドラッグ操作だけを管理します。
 * HTML5 Drag and Dropはブラウザ標準ゴーストや禁止カーソルが出やすいため、Pointer Eventsで制御します。
 */
export function useTagDragManagement(params: {
  attachTagsToGroup: (tags: RecordTag[], groupId: number) => Promise<void>;
  isTagSelected: (tagId: number) => boolean;
  selectedTags: () => RecordTag[];
  selectTagFromPointerCandidate: (
    candidate: TagPointerSelectionCandidate
  ) => void;
}) {
  const draggingTag = ref<RecordTag | null>(null);
  const draggingTags = ref<RecordTag[]>([]);
  const dragGhost = ref<DragGhostState | null>(null);
  const dragGhostElement = ref<{ style: { left: string; top: string } } | null>(
    null
  );
  const dragOverGroupId = ref<number | null>(null);
  const pointerDragStart = ref<PointerDragCandidate | null>(null);

  function isTagDragging(tagId: number) {
    return draggingTags.value.some((tag) => tag.id === tagId);
  }

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

  function findDropGroupId(clientX: number, clientY: number) {
    // 子要素上にポインターがあっても、data属性を持つグループ行まで遡れるようelementsFromPointを使います。
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

  function updateDragGhostPosition(clientX: number, clientY: number) {
    if (!dragGhostElement.value) {
      return;
    }
    dragGhostElement.value.style.left = `${clientX}px`;
    dragGhostElement.value.style.top = `${clientY}px`;
  }

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

  async function finishPointerDrag(event: PointerDragMoveEvent) {
    const start = pointerDragStart.value;
    const tags = draggingTags.value;
    const targetGroupId = findDropGroupId(event.clientX, event.clientY);

    if (tags.length > 0 && targetGroupId !== null) {
      await params.attachTagsToGroup(tags, targetGroupId);
    } else if (start && !draggingTag.value) {
      params.selectTagFromPointerCandidate(start);
    }

    clearPointerDragState();
  }

  function getDragTargetTags(tag: RecordTag) {
    const selectedTags = params.selectedTags();
    return params.isTagSelected(tag.id) && selectedTags.length > 0
      ? selectedTags
      : [tag];
  }

  const handlePointerMove = (event: unknown) =>
    movePointerDrag(event as PointerDragMoveEvent);

  const handlePointerUp = (event: unknown) =>
    void finishPointerDrag(event as PointerDragMoveEvent);

  const handlePointerCancel = () => clearPointerDragState();

  onBeforeUnmount(() => clearPointerDragState());

  return {
    dragGhost,
    dragGhostElement,
    dragOverGroupId,
    draggingTag,
    draggingTags,
    isTagDragging,
    preparePointerDrag
  };
}
