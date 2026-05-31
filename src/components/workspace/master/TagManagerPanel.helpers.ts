import type { RecordTag } from "../../../types";

export type GroupFilter = "all" | "unclassified" | "popular" | number;

export interface TagSection {
  key: string;
  title: string;
  count: number;
  tags: RecordTag[];
  groupId?: number;
}

export function tagGroupIds(tag: RecordTag) {
  return tag.groupIds?.length
    ? tag.groupIds
    : tag.groupId === null
      ? []
      : [tag.groupId];
}

export function isTagInGroup(tag: RecordTag, groupId: number) {
  return tagGroupIds(tag).includes(groupId);
}

export function unclassifiedTagsFor(tags: RecordTag[]) {
  return tags.filter((tag) => tagGroupIds(tag).length === 0);
}

export function popularTagsFor(tags: RecordTag[]) {
  return [...tags]
    .filter((tag) => tag.usageCount > 0)
    .sort((a, b) => b.usageCount - a.usageCount || a.name.localeCompare(b.name))
    .slice(0, 20);
}

export function tagsForGroup(tags: RecordTag[], groupId: number) {
  return tags.filter((tag) => isTagInGroup(tag, groupId));
}

export function uniqueTagIdsInSections(sections: TagSection[]) {
  const seen = new Set<number>();
  const ids: number[] = [];

  for (const section of sections) {
    for (const tag of section.tags) {
      if (!seen.has(tag.id)) {
        seen.add(tag.id);
        ids.push(tag.id);
      }
    }
  }

  return ids;
}
