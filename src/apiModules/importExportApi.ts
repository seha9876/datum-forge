import { invoke } from "@tauri-apps/api/core";

import type {
  ExportTableCsvPayload,
  ImportExcelTablePayload,
  ImportTableCsvPayload,
  ImportTableCsvResult,
  InspectCsvImportPayload,
  InspectCsvImportResult,
  InspectExcelTablesPayload,
  InspectExcelTablesResult,
  PreviewCsvImportPayload,
  PreviewCsvImportResult,
  PreviewExcelTableImportPayload,
  PreviewExcelTableImportResult
} from "../types";

export const importExportApi = {
  exportTableCsv: (payload: ExportTableCsvPayload) =>
    invoke<void>("export_table_csv", { payload }),
  importTableCsv: (payload: ImportTableCsvPayload) =>
    invoke<ImportTableCsvResult>("import_table_csv", { payload }),
  inspectCsvImport: (payload: InspectCsvImportPayload) =>
    invoke<InspectCsvImportResult>("inspect_csv_import", { payload }),
  previewCsvImport: (payload: PreviewCsvImportPayload) =>
    invoke<PreviewCsvImportResult>("preview_csv_import", { payload }),
  inspectExcelTables: (payload: InspectExcelTablesPayload) =>
    invoke<InspectExcelTablesResult>("inspect_excel_tables", { payload }),
  previewExcelTableImport: (payload: PreviewExcelTableImportPayload) =>
    invoke<PreviewExcelTableImportResult>("preview_excel_table_import", {
      payload
    }),
  importExcelTable: (payload: ImportExcelTablePayload) =>
    invoke<ImportTableCsvResult>("import_excel_table", { payload })
};
