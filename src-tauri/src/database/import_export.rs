//! Import/export command entry points.

mod csv;
mod excel;
mod mapping;
mod preview;
mod records;

use self::csv::*;
use self::excel::*;
use self::mapping::*;
use self::preview::*;
use self::records::*;
use super::settings::{
    last_excel_import_tables_setting, notification_settings_setting,
    save_settings_with_app_settings, show_record_ids_in_navigation_setting,
};
use super::*;
use serde_json::Value;
use std::fs;

impl Db {
    pub fn export_table_csv(&self, payload: ExportTableCsvPayload) -> Result<(), DbError> {
        let output_path = payload.output_path.trim();
        if output_path.is_empty() {
            return Err(DbError::InvalidInput("outputPath is required".into()));
        }

        // 画面表示と同じ文字列をCSVに出すため、表示値付きのレコード一覧を使います。
        let table = self.get_table_summary(payload.table_id)?;
        let columns = self.list_columns(payload.table_id)?;
        let records = self.list_records_with_order(&table.table_name, &columns, "ASC")?;
        // Excelで日本語を開きやすいよう、UTF-8 BOMを先頭に付けます。
        let mut csv = String::from("\u{feff}");
        csv.push_str(
            &columns
                .iter()
                .map(|column| csv_escape_field(&column.display_name))
                .collect::<Vec<_>>()
                .join(","),
        );
        csv.push_str("\r\n");

        for record in &records {
            let row = columns
                .iter()
                .map(|column| {
                    record
                        .display_values
                        .as_object()
                        .and_then(|values| values.get(&column.column_name))
                        .and_then(Value::as_str)
                        .map(csv_escape_field)
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
                .join(",");
            csv.push_str(&row);
            csv.push_str("\r\n");
        }

        fs::write(output_path, csv)?;
        Ok(())
    }

    pub fn import_table_csv(
        &mut self,
        payload: ImportTableCsvPayload,
    ) -> Result<ImportTableCsvResult, DbError> {
        let input_path = payload.input_path.trim();
        if input_path.is_empty() {
            return Err(DbError::InvalidInput("inputPath is required".into()));
        }

        let (headers, rows) = read_csv_import_source(input_path)?;
        let table = self.get_table_summary(payload.table_id)?;
        let columns = self.list_columns(payload.table_id)?;
        let select_option_maps = self.csv_select_option_maps(&columns)?;
        let resolved_mapping =
            resolve_import_column_mapping(&columns, &headers, &payload.column_mapping);
        let preview = build_csv_import_preview(
            &self.conn,
            &table.table_name,
            &columns,
            &headers,
            &rows,
            &resolved_mapping,
            &select_option_maps,
            payload.mode,
        )?;

        if !preview.errors.is_empty() {
            return Err(DbError::InvalidInput(preview.errors.join("\n")));
        }

        // インポートは途中失敗時に中途半端な行を残さないよう、全行を1トランザクションで処理します。
        let tx = self.conn.transaction()?;
        let mut inserted_count = 0;
        let mut updated_count = 0;
        let mut skipped_count = 0;

        for source_row in &rows {
            let values =
                csv_row_to_values(source_row, &columns, &resolved_mapping, &select_option_maps)?;
            match payload.mode {
                ImportTableCsvMode::SkipExistingPrimaryKeys => {
                    // 既存IDは触らず、CSVにしかないIDの行だけ追加します。
                    let id = csv_row_id(&values, source_row.row_number)?;
                    if csv_record_exists(&tx, &table.table_name, id)? {
                        skipped_count += 1;
                        continue;
                    }
                    csv_insert_record(&tx, &table.table_name, &columns, &values, true)?;
                    inserted_count += 1;
                }
                ImportTableCsvMode::AppendIgnoringPrimaryKeys => {
                    // CSVのIDを無視し、DB側に新しいIDを採番させて全行を追加します。
                    csv_insert_record(&tx, &table.table_name, &columns, &values, false)?;
                    inserted_count += 1;
                }
                ImportTableCsvMode::UpsertByPrimaryKey => {
                    // 同じIDがあれば更新、なければCSVのIDを維持して追加します。
                    let id = csv_row_id(&values, source_row.row_number)?;
                    if csv_record_exists(&tx, &table.table_name, id)? {
                        csv_update_record(&tx, &table.table_name, &columns, &values, id)?;
                        updated_count += 1;
                    } else {
                        csv_insert_record(&tx, &table.table_name, &columns, &values, true)?;
                        inserted_count += 1;
                    }
                }
            }
        }
        tx.commit()?;
        let mut details = preview.warnings;
        if skipped_count > 0 && details.is_empty() {
            details.push(format!(
                "{skipped_count}件は既存IDと重複したためスキップしました。"
            ));
        }

        Ok(ImportTableCsvResult {
            status: if skipped_count > 0 {
                "warning".into()
            } else {
                "success".into()
            },
            inserted_count,
            updated_count,
            skipped_count,
            error_count: 0,
            details,
        })
    }

    pub fn inspect_csv_import(
        &self,
        payload: InspectCsvImportPayload,
    ) -> Result<InspectCsvImportResult, DbError> {
        let input_path = payload.input_path.trim();
        if input_path.is_empty() {
            return Err(DbError::InvalidInput("inputPath is required".into()));
        }

        let columns = self.list_columns(payload.table_id)?;
        let (headers, rows) = read_csv_import_source(input_path)?;
        let mapping = resolve_import_column_mapping(&columns, &headers, &[]);

        Ok(InspectCsvImportResult {
            headers,
            row_count: rows.len(),
            column_mappings: import_column_mapping_suggestions(&mapping),
        })
    }

    pub fn preview_csv_import(
        &self,
        payload: PreviewCsvImportPayload,
    ) -> Result<PreviewCsvImportResult, DbError> {
        let input_path = payload.input_path.trim();
        if input_path.is_empty() {
            return Err(DbError::InvalidInput("inputPath is required".into()));
        }

        let table = self.get_table_summary(payload.table_id)?;
        let columns = self.list_columns(payload.table_id)?;
        let select_option_maps = self.csv_select_option_maps(&columns)?;
        let (headers, rows) = read_csv_import_source(input_path)?;
        let resolved_mapping =
            resolve_import_column_mapping(&columns, &headers, &payload.column_mapping);

        build_csv_import_preview(
            &self.conn,
            &table.table_name,
            &columns,
            &headers,
            &rows,
            &resolved_mapping,
            &select_option_maps,
            payload.mode,
        )
    }

    pub fn inspect_excel_tables(
        &self,
        payload: InspectExcelTablesPayload,
    ) -> Result<InspectExcelTablesResult, DbError> {
        let input_path = payload.input_path.trim();
        if input_path.is_empty() {
            return Err(DbError::InvalidInput("inputPath is required".into()));
        }

        let target_table = self.get_table_summary(payload.table_id)?;
        let workbook = ExcelWorkbook::open(input_path)?;
        let tables = workbook.table_infos();
        let last_used_table_name = last_excel_import_tables_setting()
            .get(&payload.table_id)
            .cloned();
        let suggested_table_name =
            suggest_excel_table_name(&target_table, &tables, last_used_table_name.as_deref());

        Ok(InspectExcelTablesResult {
            tables,
            suggested_table_name,
            last_used_table_name,
        })
    }

    pub fn preview_excel_table_import(
        &self,
        payload: PreviewExcelTableImportPayload,
    ) -> Result<PreviewExcelTableImportResult, DbError> {
        let table = self.get_table_summary(payload.table_id)?;
        let columns = self.list_columns(payload.table_id)?;
        let workbook = ExcelWorkbook::open(payload.input_path.trim())?;
        let excel_table = workbook.table_by_name(&payload.excel_table_name)?;
        let select_option_maps = self.csv_select_option_maps(&columns)?;
        let resolved_mapping = resolve_import_column_mapping(
            &columns,
            &excel_table.info.column_names,
            &payload.column_mapping,
        );
        let rows = workbook.table_rows(&excel_table);

        build_excel_import_preview(
            &self.conn,
            &table.table_name,
            &columns,
            &excel_table,
            &rows,
            &resolved_mapping,
            &select_option_maps,
            payload.mode,
        )
    }

    pub fn import_excel_table(
        &mut self,
        payload: ImportExcelTablePayload,
    ) -> Result<ImportTableCsvResult, DbError> {
        let table = self.get_table_summary(payload.table_id)?;
        let columns = self.list_columns(payload.table_id)?;
        let workbook = ExcelWorkbook::open(payload.input_path.trim())?;
        let excel_table = workbook.table_by_name(&payload.excel_table_name)?;
        let select_option_maps = self.csv_select_option_maps(&columns)?;
        let resolved_mapping = resolve_import_column_mapping(
            &columns,
            &excel_table.info.column_names,
            &payload.column_mapping,
        );
        let rows = workbook.table_rows(&excel_table);
        let preview = build_excel_import_preview(
            &self.conn,
            &table.table_name,
            &columns,
            &excel_table,
            &rows,
            &resolved_mapping,
            &select_option_maps,
            payload.mode,
        )?;

        if !preview.errors.is_empty() {
            return Err(DbError::InvalidInput(preview.errors.join("\n")));
        }

        let tx = self.conn.transaction()?;
        let mut inserted_count = 0;
        let mut updated_count = 0;
        let mut skipped_count = 0;

        for source_row in &rows {
            let values =
                excel_row_to_values(source_row, &columns, &resolved_mapping, &select_option_maps)?;
            match payload.mode {
                ImportTableCsvMode::SkipExistingPrimaryKeys => {
                    let id = csv_row_id(&values, source_row.row_number)?;
                    if csv_record_exists(&tx, &table.table_name, id)? {
                        skipped_count += 1;
                        continue;
                    }
                    csv_insert_record(&tx, &table.table_name, &columns, &values, true)?;
                    inserted_count += 1;
                }
                ImportTableCsvMode::AppendIgnoringPrimaryKeys => {
                    csv_insert_record(&tx, &table.table_name, &columns, &values, false)?;
                    inserted_count += 1;
                }
                ImportTableCsvMode::UpsertByPrimaryKey => {
                    let id = csv_row_id(&values, source_row.row_number)?;
                    if csv_record_exists(&tx, &table.table_name, id)? {
                        csv_update_record(&tx, &table.table_name, &columns, &values, id)?;
                        updated_count += 1;
                    } else {
                        csv_insert_record(&tx, &table.table_name, &columns, &values, true)?;
                        inserted_count += 1;
                    }
                }
            }
        }
        tx.commit()?;

        let mut settings = last_excel_import_tables_setting();
        settings.insert(payload.table_id, excel_table.name.clone());
        save_settings_with_app_settings(
            &self.db_path,
            show_record_ids_in_navigation_setting(),
            notification_settings_setting(),
            settings,
        )?;

        let mut details = preview.warnings;
        if skipped_count > 0 && details.is_empty() {
            details.push(format!(
                "{skipped_count}件は既存IDと重複したためスキップしました。"
            ));
        }

        Ok(ImportTableCsvResult {
            status: if skipped_count > 0 || !details.is_empty() {
                "warning".into()
            } else {
                "success".into()
            },
            inserted_count,
            updated_count,
            skipped_count,
            error_count: 0,
            details,
        })
    }
}
