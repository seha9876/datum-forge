export type ViewNavNodeType = "folder";

export interface ViewNavFolderRecord {
  id: number;
  folderId: number;
  tableId: number;
  tableName: string;
  tableDisplayName: string;
  recordId: number;
  recordLabel: string;
  recordTemplateId: number | null;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
}

export interface ViewNavNode {
  id: number;
  nodeType: ViewNavNodeType;
  parentId: number | null;
  name: string;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
}

export interface ViewNavTreeNode extends ViewNavNode {
  children: ViewNavTreeNode[];
  records: ViewNavFolderRecord[];
}

export interface ViewTableRecordSummary {
  id: number;
  label: string;
}

export interface ViewTableSection {
  tableId: number;
  tableName: string;
  displayName: string;
  records: ViewTableRecordSummary[];
}

export interface TemplatePreviewRecordSelection {
  tableId: number;
  tableName: string;
  tableDisplayName: string;
  recordId: number;
  recordLabel: string;
}

export interface CreateViewNavFolderPayload {
  parentId: number | null;
  name: string;
}

export interface DeleteViewNavFolderPayload {
  folderId: number;
}

export interface AddViewNavFolderRecordsPayload {
  folderId: number;
  tableId: number;
  records: Array<{
    recordId: number;
    recordLabel: string;
  }>;
}

export interface RemoveViewNavFolderRecordPayload {
  folderRecordId: number;
}

/** 閲覧目次で、指定フォルダー内レコードを保存したい順番に並べたpayloadです。 */
export interface ReorderViewNavFolderRecordsPayload {
  folderId: number;
  orderedFolderRecordIds: number[];
}

export type ViewSelection =
  | {
      type: "tableRecord";
      tableId: number;
      tableName: string;
      tableDisplayName: string;
      recordId: number;
      recordLabel: string;
      folderId?: number | null;
      folderRecordId?: number | null;
      recordTemplateId?: number | null;
    }
  | {
      type: "folder";
      folderId: number;
      folderName: string;
    };
