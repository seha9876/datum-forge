import type {
  SaveViewLayoutCardOverridesPayload,
  ViewLayoutCardItem,
  ViewLayoutTemplate,
  ViewNavFolderRecord,
  ViewNavNode,
  ViewNavTreeNode
} from "../types";

export function buildViewNavTree(
  nodes: ViewNavNode[],
  records: ViewNavFolderRecord[],
  parentId: number | null
): ViewNavTreeNode[] {
  return nodes
    .filter((node) => node.parentId === parentId)
    .sort(compareNodes)
    .map((node) => ({
      ...node,
      children: buildViewNavTree(nodes, records, node.id),
      records: records
        .filter((record) => record.folderId === node.id)
        .sort(compareFolderRecords)
    }));
}

export function toggleExpandedId(expandedIds: number[], targetId: number) {
  return expandedIds.includes(targetId)
    ? expandedIds.filter((id) => id !== targetId)
    : [...expandedIds, targetId];
}

export function compareLayoutTemplates(
  left: ViewLayoutTemplate,
  right: ViewLayoutTemplate
) {
  if (left.folderId === null && right.folderId !== null) {
    return -1;
  }
  if (left.folderId !== null && right.folderId === null) {
    return 1;
  }
  return left.name.localeCompare(right.name) || left.id - right.id;
}

export function isSameFolderRecord(
  left: ViewNavFolderRecord,
  right: ViewNavFolderRecord
) {
  return (
    left.folderId === right.folderId &&
    left.tableId === right.tableId &&
    left.recordId === right.recordId
  );
}

export function toLayoutPayloadItems(
  items: ViewLayoutCardItem[]
): SaveViewLayoutCardOverridesPayload["items"] {
  return items.map((item) => ({
    cardId: item.cardId,
    columnId: item.columnId,
    x: item.x,
    y: item.y,
    width: item.width,
    height: item.height,
    visible: item.visible,
    backgroundColor: item.backgroundColor,
    textColor: item.textColor,
    fontSize: item.fontSize,
    textDirection: item.textDirection,
    fontWeight: item.fontWeight,
    textAlign: item.textAlign,
    padding: item.padding,
    paddingTop: item.paddingTop,
    paddingRight: item.paddingRight,
    paddingBottom: item.paddingBottom,
    paddingLeft: item.paddingLeft,
    borderRadius: item.borderRadius,
    showLabel: item.showLabel
  }));
}

export function collectFolderSubtreeIds(nodes: ViewNavNode[], rootId: number) {
  const result = new Set<number>([rootId]);
  let changed = true;

  while (changed) {
    changed = false;
    for (const node of nodes) {
      if (
        node.parentId !== null &&
        result.has(node.parentId) &&
        !result.has(node.id)
      ) {
        result.add(node.id);
        changed = true;
      }
    }
  }

  return Array.from(result);
}

export function compareNodes(left: ViewNavNode, right: ViewNavNode) {
  if (left.sortOrder !== right.sortOrder) {
    return left.sortOrder - right.sortOrder;
  }

  return left.id - right.id;
}

export function compareFolderRecords(
  left: ViewNavFolderRecord,
  right: ViewNavFolderRecord
) {
  if (left.folderId !== right.folderId) {
    return left.folderId - right.folderId;
  }

  if (left.sortOrder !== right.sortOrder) {
    return left.sortOrder - right.sortOrder;
  }

  return left.id - right.id;
}
