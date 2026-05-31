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
  columnMapping: ImportColumnMappingPayload[];
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

export interface ImportColumnMappingPayload {
  targetColumnName: string;
  sourceColumnName: string;
}

export interface InspectCsvImportPayload {
  tableId: number;
  inputPath: string;
}

export interface InspectCsvImportResult {
  headers: string[];
  rowCount: number;
  columnMappings: ImportColumnMappingSuggestion[];
}

export interface PreviewCsvImportPayload {
  tableId: number;
  inputPath: string;
  mode: ImportTableCsvMode;
  columnMapping: ImportColumnMappingPayload[];
}

export interface PreviewExcelTableImportPayload {
  tableId: number;
  inputPath: string;
  excelTableName: string;
  mode: ImportTableCsvMode;
  columnMapping: ImportColumnMappingPayload[];
}

export interface ImportExcelTablePayload {
  tableId: number;
  inputPath: string;
  excelTableName: string;
  mode: ImportTableCsvMode;
  columnMapping: ImportColumnMappingPayload[];
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

export interface ImportColumnMappingSuggestion {
  targetColumnName: string;
  targetDisplayName: string;
  sourceColumnName: string | null;
  matchedBy: string | null;
  isRequired: boolean;
}

export interface PreviewExcelTableImportResult {
  excelTable: ExcelTableInfo;
  columnMappings: ImportColumnMappingSuggestion[];
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

export interface PreviewCsvImportResult {
  columnMappings: ImportColumnMappingSuggestion[];
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
