export type FieldType =
  | "text"
  | "integer"
  | "real"
  | "boolean"
  | "date"
  | "image"
  | "single_select"
  | "reference";

export interface AppTableSummary {
  id: number;
  tableName: string;
  displayName: string;
  labelColumnId: number | null;
  sortOrder: number;
}

export interface AppColumn {
  id: number;
  tableId: number;
  columnName: string;
  displayName: string;
  fieldType: FieldType;
  sortOrder: number;
  selectOptionGroupId: number | null;
  refTableId: number | null;
  isRequired: boolean;
}

export interface SelectOption {
  id: number;
  groupId: number;
  optionNo: number;
  sortOrder: number;
  label: string;
}

export interface SelectOptionGroup {
  id: number;
  name: string;
  description: string | null;
  options: SelectOption[];
}

export interface TableRecord {
  id: number;
  values: Record<string, unknown>;
  displayValues: Record<string, string>;
}

export interface TableDetail {
  table: AppTableSummary;
  columns: AppColumn[];
  records: TableRecord[];
}

export interface ReferenceChoice {
  id: number;
  label: string;
}

export interface AppBootstrap {
  tables: AppTableSummary[];
  optionGroups: SelectOptionGroup[];
}

export interface CreateTablePayload {
  tableName: string;
  displayName: string;
}

export interface DeleteTablePayload {
  tableId: number;
}

export interface AddColumnPayload {
  tableId: number;
  columnName: string;
  displayName: string;
  fieldType: FieldType;
  isRequired: boolean;
  selectOptionGroupId?: number | null;
  refTableId?: number | null;
}

export interface DeleteColumnPayload {
  tableId: number;
  columnId: number;
}

export interface DeleteRecordPayload {
  tableId: number;
  recordId: number;
}

export interface UpdateColumnPayload {
  tableId: number;
  columnId: number;
  columnName: string;
  displayName: string;
  isRequired: boolean;
}

export interface UpdateLabelColumnPayload {
  tableId: number;
  labelColumnId: number | null;
}

export interface ReorderColumnsPayload {
  tableId: number;
  orderedColumnIds: number[];
}

export interface SaveOptionGroupPayload {
  id?: number;
  name: string;
  description?: string;
  options: Array<{
    clientKey?: string;
    id?: number;
    optionNo: number;
    sortOrder: number;
    label: string;
  }>;
}

export interface SaveRecordPayload {
  tableId: number;
  recordId?: number | null;
  values: Record<string, unknown>;
}
