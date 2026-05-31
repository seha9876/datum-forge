//! CSV/Excel共通のインポートプレビュー生成を担当します。
//!
//! DB更新は行わず、取り込み前にエラー・警告・変換結果を確認するための情報だけを作ります。

use super::mapping::*;
use super::records::*;
use super::*;
use rusqlite::Connection;
use std::collections::HashMap;

pub(super) fn build_csv_import_preview(
    conn: &Connection,
    table_name: &str,
    columns: &[AppColumn],
    headers: &[String],
    rows: &[CsvSourceRow],
    mapping: &[ResolvedImportColumnMapping],
    select_option_maps: &HashMap<String, HashMap<String, i64>>,
    mode: ImportTableCsvMode,
) -> Result<PreviewCsvImportResult, DbError> {
    let mut errors = validate_import_mapping("CSV", headers, mapping);
    let mut warnings = import_mapping_warnings("CSV", headers, mapping);
    let mut preview_rows = Vec::new();
    let mut inserted_count = 0;
    let mut updated_count = 0;
    let mut unchanged_count = 0;
    let mut skipped_count = 0;

    if errors.is_empty() {
        // プレビューはDBを書き換えず、実取り込みと同じ判定だけを先に走らせます。
        for row in rows {
            match csv_row_to_values(row, columns, mapping, select_option_maps) {
                Ok(values) => {
                    if preview_rows.len() < 10 {
                        preview_rows.push(csv_preview_row(row, mapping));
                    }
                    match mode {
                        ImportTableCsvMode::SkipExistingPrimaryKeys => {
                            let id = csv_row_id(&values, row.row_number)?;
                            if record_exists(conn, table_name, id)? {
                                skipped_count += 1;
                            } else {
                                inserted_count += 1;
                            }
                        }
                        ImportTableCsvMode::AppendIgnoringPrimaryKeys => {
                            inserted_count += 1;
                        }
                        ImportTableCsvMode::UpsertByPrimaryKey => {
                            let id = csv_row_id(&values, row.row_number)?;
                            if record_exists(conn, table_name, id)? {
                                if record_values_match(conn, table_name, columns, id, &values)? {
                                    unchanged_count += 1;
                                } else {
                                    updated_count += 1;
                                }
                            } else {
                                inserted_count += 1;
                            }
                        }
                    }
                }
                Err(error) => errors.push(error.to_string()),
            }
        }
    }

    if skipped_count > 0 {
        warnings.push(format!(
            "{skipped_count}件は既存IDと重複するため、選択中の方式ではスキップされます。"
        ));
    }
    if unchanged_count > 0 {
        warnings.push(format!(
            "{unchanged_count}件は既存データと同じ内容のため、更新されません。"
        ));
    }

    Ok(PreviewCsvImportResult {
        column_mappings: import_column_mapping_suggestions(mapping),
        preview_rows,
        total_rows: rows.len(),
        inserted_count,
        updated_count,
        unchanged_count,
        skipped_count,
        error_count: errors.len(),
        warnings,
        errors,
    })
}

pub(super) fn build_excel_import_preview(
    conn: &Connection,
    table_name: &str,
    columns: &[AppColumn],
    excel_table: &ExcelTable,
    rows: &[ExcelSourceRow],
    mapping: &[ResolvedImportColumnMapping],
    select_option_maps: &HashMap<String, HashMap<String, i64>>,
    mode: ImportTableCsvMode,
) -> Result<PreviewExcelTableImportResult, DbError> {
    let mut errors = validate_import_mapping("Excel", &excel_table.info.column_names, mapping);
    let mut warnings = import_mapping_warnings("Excel", &excel_table.info.column_names, mapping);
    let mut preview_rows = Vec::new();
    let mut inserted_count = 0;
    let mut updated_count = 0;
    let mut unchanged_count = 0;
    let mut skipped_count = 0;

    if errors.is_empty() {
        for row in rows {
            match excel_row_to_values(row, columns, mapping, select_option_maps) {
                Ok(values) => {
                    if preview_rows.len() < 10 {
                        preview_rows.push(excel_preview_row(row, mapping));
                    }
                    match mode {
                        ImportTableCsvMode::SkipExistingPrimaryKeys => {
                            let id = csv_row_id(&values, row.row_number)?;
                            if record_exists(conn, table_name, id)? {
                                skipped_count += 1;
                            } else {
                                inserted_count += 1;
                            }
                        }
                        ImportTableCsvMode::AppendIgnoringPrimaryKeys => {
                            inserted_count += 1;
                        }
                        ImportTableCsvMode::UpsertByPrimaryKey => {
                            let id = csv_row_id(&values, row.row_number)?;
                            if record_exists(conn, table_name, id)? {
                                if record_values_match(conn, table_name, columns, id, &values)? {
                                    unchanged_count += 1;
                                } else {
                                    updated_count += 1;
                                }
                            } else {
                                inserted_count += 1;
                            }
                        }
                    }
                }
                Err(error) => errors.push(error.to_string()),
            }
        }
    }

    if skipped_count > 0 {
        warnings.push(format!(
            "{skipped_count}件は既存IDと重複するため、選択中の方式ではスキップされます。"
        ));
    }
    if unchanged_count > 0 {
        warnings.push(format!(
            "{unchanged_count}件は既存データと同じ内容のため、更新されません。"
        ));
    }

    Ok(PreviewExcelTableImportResult {
        excel_table: excel_table.info.clone(),
        column_mappings: import_column_mapping_suggestions(mapping),
        preview_rows,
        total_rows: rows.len(),
        inserted_count,
        updated_count,
        unchanged_count,
        skipped_count,
        error_count: errors.len(),
        warnings,
        errors,
    })
}

fn csv_preview_row(
    row: &CsvSourceRow,
    mapping: &[ResolvedImportColumnMapping],
) -> HashMap<String, String> {
    mapping
        .iter()
        .filter_map(|item| {
            let source = item.source_column_name.as_ref()?;
            Some((
                item.column.column_name.clone(),
                row.values.get(source).cloned().unwrap_or_default(),
            ))
        })
        .collect()
}

fn excel_preview_row(
    row: &ExcelSourceRow,
    mapping: &[ResolvedImportColumnMapping],
) -> HashMap<String, String> {
    mapping
        .iter()
        .filter_map(|item| {
            let source = item.source_column_name.as_ref()?;
            Some((
                item.column.column_name.clone(),
                row.values.get(source).cloned().unwrap_or_default(),
            ))
        })
        .collect()
}
