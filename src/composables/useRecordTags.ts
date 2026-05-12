import { ref } from "vue";

import { api } from "../api";

import type {
  RecordTag,
  RecordTagGroup,
  SaveRecordTagGroupPayload,
  SaveRecordTagPayload
} from "../types";

export function useRecordTags() {
  const groups = ref<RecordTagGroup[]>([]);
  const tags = ref<RecordTag[]>([]);
  const selectedRecordTags = ref<RecordTag[]>([]);
  const loading = ref(false);
  const error = ref("");

  function replaceTag(nextTag: RecordTag) {
    const index = tags.value.findIndex((tag) => tag.id === nextTag.id);
    if (index >= 0) {
      tags.value[index] = nextTag;
      return;
    }
    tags.value.push(nextTag);
  }

  async function refreshTags() {
    loading.value = true;
    error.value = "";
    try {
      const bundle = await api.listRecordTags();
      groups.value = bundle.groups;
      tags.value = bundle.tags;
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err);
    } finally {
      loading.value = false;
    }
  }

  async function loadRecordTags(tableId: number, recordId: number) {
    loading.value = true;
    error.value = "";
    try {
      selectedRecordTags.value = await api.listRecordTagsForRecord(
        tableId,
        recordId
      );
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err);
      selectedRecordTags.value = [];
    } finally {
      loading.value = false;
    }
  }

  function clearRecordTags() {
    selectedRecordTags.value = [];
  }

  async function attachExistingTag(
    tableId: number,
    recordId: number,
    tagId: number
  ) {
    const tag = await api.attachRecordTag({ tableId, recordId, tagId });
    if (!selectedRecordTags.value.some((selected) => selected.id === tag.id)) {
      selectedRecordTags.value.push(tag);
    }
    await refreshTags();
    await loadRecordTags(tableId, recordId);
  }

  async function createAndAttachTag(
    tableId: number,
    recordId: number,
    name: string
  ) {
    const tag = await api.createAndAttachRecordTag({ tableId, recordId, name });
    if (!selectedRecordTags.value.some((selected) => selected.id === tag.id)) {
      selectedRecordTags.value.push(tag);
    }
    replaceTag(tag);
    await refreshTags();
    await loadRecordTags(tableId, recordId);
  }

  async function detachTag(tableId: number, recordId: number, tagId: number) {
    await api.detachRecordTag({ tableId, recordId, tagId });
    selectedRecordTags.value = selectedRecordTags.value.filter(
      (tag) => tag.id !== tagId
    );
    await refreshTags();
  }

  async function saveTagGroup(payload: SaveRecordTagGroupPayload) {
    await api.saveRecordTagGroup(payload);
    await refreshTags();
  }

  async function deleteTagGroup(groupId: number) {
    await api.deleteRecordTagGroup({ groupId });
    await refreshTags();
  }

  async function saveTag(payload: SaveRecordTagPayload) {
    await api.saveRecordTag(payload);
    await refreshTags();
  }

  async function deleteTag(tagId: number) {
    await api.deleteRecordTag({ tagId });
    selectedRecordTags.value = selectedRecordTags.value.filter(
      (tag) => tag.id !== tagId
    );
    await refreshTags();
  }

  async function attachTagGroup(tagId: number, groupId: number) {
    await api.attachRecordTagGroup({ tagId, groupId });
    await refreshTags();
  }

  async function detachTagGroup(tagId: number, groupId: number) {
    await api.detachRecordTagGroup({ tagId, groupId });
    await refreshTags();
  }

  return {
    attachTagGroup,
    clearRecordTags,
    createAndAttachTag,
    deleteTag,
    deleteTagGroup,
    detachTag,
    detachTagGroup,
    error,
    groups,
    loading,
    loadRecordTags,
    refreshTags,
    saveTag,
    saveTagGroup,
    selectedRecordTags,
    tags,
    attachExistingTag
  };
}
