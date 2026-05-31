export interface RecordTagGroup {
  id: number;
  name: string;
  sortOrder: number;
  usageCount: number;
  tagCount: number;
  createdAt: string;
  updatedAt: string;
}

export interface RecordTag {
  id: number;
  groupId: number | null;
  groupIds: number[];
  name: string;
  sortOrder: number;
  usageCount: number;
  createdAt: string;
  updatedAt: string;
}

export interface RecordTagBundle {
  groups: RecordTagGroup[];
  tags: RecordTag[];
}

export interface SaveRecordTagGroupPayload {
  id?: number | null;
  name: string;
}

export interface SaveRecordTagPayload {
  id?: number | null;
  name: string;
  groupId?: number | null;
}

export interface DeleteRecordTagPayload {
  tagId: number;
}

export interface RecordTagGroupLinkPayload {
  tagId: number;
  groupId: number;
}

export interface AttachRecordTagPayload {
  tableId: number;
  recordId: number;
  tagId: number;
}

export interface CreateAndAttachRecordTagPayload {
  tableId: number;
  recordId: number;
  name: string;
}

export interface DetachRecordTagPayload {
  tableId: number;
  recordId: number;
  tagId: number;
}
