import type { ViewNavFolderRecord, ViewNavTreeNode } from "../../../types";

export type ManualTreeItem =
  | {
      id: string;
      kind: "folder";
      title: string;
      searchText: string;
      node: ViewNavTreeNode;
      children: ManualTreeItem[];
    }
  | {
      id: string;
      kind: "record";
      title: string;
      searchText: string;
      record: ViewNavFolderRecord;
    };

export function folderItemId(folderId: number) {
  return `folder:${folderId}`;
}

export function folderRecordItemId(folderRecordId: number) {
  return `folder-record:${folderRecordId}`;
}

export function countDescendants(node: ViewNavTreeNode): number {
  return node.children.reduce(
    (total, child) => total + 1 + countDescendants(child),
    0
  );
}

export function searchManualTreeItem(value: string, query: string) {
  const keyword = query.trim().toLocaleLowerCase();
  if (!keyword) {
    return true;
  }

  return value.toLocaleLowerCase().includes(keyword);
}

export function buildFolderItem(
  node: ViewNavTreeNode,
  ancestors: string[] = []
): ManualTreeItem {
  const currentPath = [...ancestors, node.name];

  return {
    id: folderItemId(node.id),
    kind: "folder",
    title: node.name,
    searchText: folderSearchText(node, ancestors),
    node,
    children: [
      ...node.children.map((child) => buildFolderItem(child, currentPath)),
      ...node.records.map((record) => {
        const label = folderRecordLabel(record);

        return {
          id: folderRecordItemId(record.id),
          kind: "record" as const,
          title: label,
          searchText: folderRecordSearchText(record, label, currentPath),
          record
        };
      })
    ]
  };
}

export function findFolderItem(
  items: ManualTreeItem[],
  itemId: string
): Extract<ManualTreeItem, { kind: "folder" }> | null {
  for (const item of items) {
    if (item.kind === "folder" && item.id === itemId) {
      return item;
    }

    if (item.kind === "folder") {
      const childMatch = findFolderItem(item.children, itemId);
      if (childMatch) {
        return childMatch;
      }
    }
  }

  return null;
}

export function findRecordItem(
  items: ManualTreeItem[],
  itemId: string
): Extract<ManualTreeItem, { kind: "record" }> | null {
  for (const item of items) {
    if (item.kind === "record" && item.id === itemId) {
      return item;
    }

    if (item.kind === "folder") {
      const childMatch = findRecordItem(item.children, itemId);
      if (childMatch) {
        return childMatch;
      }
    }
  }

  return null;
}

export function findFolderNodeById(
  nodes: ViewNavTreeNode[],
  folderId: number
): ViewNavTreeNode | null {
  for (const node of nodes) {
    if (node.id === folderId) {
      return node;
    }

    const childMatch = findFolderNodeById(node.children, folderId);
    if (childMatch) {
      return childMatch;
    }
  }

  return null;
}

function folderRecordLabel(record: ViewNavFolderRecord) {
  return record.recordLabel.replace(/^\d+:/, "") || record.recordLabel;
}

function folderSearchText(node: ViewNavTreeNode, ancestors: string[]) {
  return [...ancestors, node.name].join(" ");
}

function folderRecordSearchText(
  record: ViewNavFolderRecord,
  label: string,
  ancestors: string[]
) {
  return [
    ...ancestors,
    record.recordId,
    record.recordLabel,
    label,
    record.tableName,
    record.tableDisplayName
  ].join(" ");
}
