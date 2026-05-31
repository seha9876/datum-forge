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

export interface ViewLayoutCardItem {
  tableId: number;
  cardId: number;
  columnId: number | null;
  label?: string | null;
  x: number;
  y: number;
  width: number;
  height: number;
  visible: boolean;
  backgroundColor?: string | null;
  textColor?: string | null;
  fontSize?: number | null;
  textDirection?: "horizontal" | "vertical" | null;
  fontWeight?: "normal" | "bold" | null;
  textAlign?: "left" | "center" | "right" | null;
  padding?: number | null;
  paddingTop?: number | null;
  paddingRight?: number | null;
  paddingBottom?: number | null;
  paddingLeft?: number | null;
  borderRadius?: number | null;
  showLabel?: boolean | null;
  hasOverride: boolean;
}

export interface ViewLayoutCardColumnBinding {
  cardId: number;
  columnId: number;
}

export interface ViewLayoutTemplate {
  id: number;
  name: string;
  scopeType: "folder";
  folderId: number | null;
  createdAt: string;
  updatedAt: string;
}

export interface ViewLayoutTemplateCard {
  cardId: number;
  x: number;
  y: number;
  width: number;
  height: number;
  visible: boolean;
  label: string | null;
  backgroundColor?: string | null;
  textColor?: string | null;
  fontSize?: number | null;
  textDirection?: "horizontal" | "vertical" | null;
  fontWeight?: "normal" | "bold" | null;
  textAlign?: "left" | "center" | "right" | null;
  padding?: number | null;
  paddingTop?: number | null;
  paddingRight?: number | null;
  paddingBottom?: number | null;
  paddingLeft?: number | null;
  borderRadius?: number | null;
  showLabel?: boolean | null;
}

export interface ResolvedViewFieldLayout {
  templates: ViewLayoutTemplate[];
  activeTemplateId: number | null;
  activeTemplateName: string | null;
  items: ViewLayoutCardItem[];
}

export interface FolderViewLayoutTemplates {
  templates: ViewLayoutTemplate[];
  activeTemplateId: number | null;
}

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

export interface SaveViewLayoutCardOverridesPayload {
  templateId: number;
  tableId: number;
  recordId: number;
  items: Array<{
    cardId: number;
    columnId: number | null;
    x: number;
    y: number;
    width: number;
    height: number;
    visible: boolean;
    backgroundColor?: string | null;
    textColor?: string | null;
    fontSize?: number | null;
    textDirection?: "horizontal" | "vertical" | null;
    fontWeight?: "normal" | "bold" | null;
    textAlign?: "left" | "center" | "right" | null;
    padding?: number | null;
    paddingTop?: number | null;
    paddingRight?: number | null;
    paddingBottom?: number | null;
    paddingLeft?: number | null;
    borderRadius?: number | null;
    showLabel?: boolean | null;
  }>;
}

export interface CreateViewLayoutTemplatePayload {
  name: string;
  scopeType?: "folder";
  folderId?: number | null;
}

export interface RenameViewLayoutTemplatePayload {
  templateId: number;
  name: string;
}

export interface DuplicateViewLayoutTemplatePayload {
  templateId: number;
  name: string;
}

export interface DeleteViewLayoutTemplatePayload {
  templateId: number;
}

export interface ListViewLayoutTemplatesForFolderPayload {
  folderId: number;
}

export interface AssignViewLayoutFolderTemplatePayload {
  folderId: number;
  templateId: number;
}

export interface GetResolvedViewFieldLayoutPayload {
  tableId: number;
  recordId: number;
  folderId?: number | null;
  folderRecordId?: number | null;
}

export interface AssignViewLayoutRecordTemplatePayload {
  folderRecordId: number;
  templateId: number;
}

export interface ClearViewLayoutRecordTemplatePayload {
  folderRecordId: number;
}

export interface GetViewLayoutTemplateCardsPayload {
  templateId: number;
}

export interface ListViewLayoutCardColumnBindingsPayload {
  templateId: number;
  tableId: number;
}

export interface SaveViewLayoutTemplateCardsPayload {
  templateId: number;
  cards: ViewLayoutTemplateCard[];
}

export interface SaveViewLayoutCardColumnBindingsPayload {
  templateId: number;
  tableId: number;
  bindings: ViewLayoutCardColumnBinding[];
}

export interface ResetViewLayoutCardOverridePayload {
  templateId: number;
  tableId: number;
  recordId: number;
  cardId: number;
}

export interface ResetViewLayoutCardOverridesPayload {
  templateId: number;
  tableId: number;
  recordId: number;
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

/** 閲覧目次で、指定フォルダー内レコードを保存したい順番に並べた payload です。 */
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

export interface AppBootstrap {
  tables: AppTableSummary[];
  optionGroups: SelectOptionGroup[];
}

export interface AppSettings {
  dbPath: string;
  showRecordIdsInNavigation: boolean;
  notificationSettings: NotificationSettings;
  lastExcelImportTables: Record<number, string>;
}

export interface NotificationSettings {
  usePerKindDurations: boolean;
  commonDurationSeconds: number;
  successDurationSeconds: number;
  warningDurationSeconds: number;
  errorDurationSeconds: number;
}

export interface UpdateNotificationSettingsPayload {
  notificationSettings: NotificationSettings;
}

export type StartupDbState = "ready" | "firstLaunch" | "missingDb" | "error";

export interface StartupDbStatus {
  state: StartupDbState;
  dbPath: string | null;
  defaultDbDirectory: string;
  defaultDbFileName: string;
  missingDbPath: string | null;
  message: string | null;
}

export interface CreateDatabasePayload {
  dbDirectory: string;
  dbFileName: string;
}

export interface CreateTablePayload {
  tableName: string;
  displayName: string;
}

export interface DeleteTablePayload {
  tableId: number;
}

export interface ExportTableCsvPayload {
  tableId: number;
  outputPath: string;
}

export type ImportTableCsvMode =
  | "skipExistingPrimaryKeys"
  | "appendIgnoringPrimaryKeys"
  | "upsertByPrimaryKey";

export interface ImportTableCsvPayload {
  tableId: number;
  inputPath: string;
  mode: ImportTableCsvMode;
}

export type ImportTableCsvStatus = "success" | "warning";

export interface ImportTableCsvResult {
  status: ImportTableCsvStatus;
  insertedCount: number;
  updatedCount: number;
  skippedCount: number;
  errorCount: number;
  details: string[];
}

export interface InspectExcelTablesPayload {
  tableId: number;
  inputPath: string;
}

export interface ExcelColumnMappingPayload {
  targetColumnName: string;
  sourceColumnName: string;
}

export interface PreviewExcelTableImportPayload {
  tableId: number;
  inputPath: string;
  excelTableName: string;
  mode: ImportTableCsvMode;
  columnMapping: ExcelColumnMappingPayload[];
}

export interface ImportExcelTablePayload {
  tableId: number;
  inputPath: string;
  excelTableName: string;
  mode: ImportTableCsvMode;
  columnMapping: ExcelColumnMappingPayload[];
}

export interface ExcelTableInfo {
  name: string;
  displayName: string;
  sheetName: string;
  range: string;
  columnNames: string[];
  rowCount: number;
}

export interface InspectExcelTablesResult {
  tables: ExcelTableInfo[];
  suggestedTableName: string | null;
  lastUsedTableName: string | null;
}

export interface ExcelColumnMappingSuggestion {
  targetColumnName: string;
  targetDisplayName: string;
  sourceColumnName: string | null;
  matchedBy: string | null;
  isRequired: boolean;
}

export interface PreviewExcelTableImportResult {
  excelTable: ExcelTableInfo;
  columnMappings: ExcelColumnMappingSuggestion[];
  previewRows: Array<Record<string, string>>;
  totalRows: number;
  insertedCount: number;
  updatedCount: number;
  unchangedCount: number;
  skippedCount: number;
  errorCount: number;
  warnings: string[];
  errors: string[];
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
