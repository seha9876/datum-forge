//! CSV/Excel共通のインポートプレビュー生成を担当します。
//!
//! DB更新は行わず、取り込み前にエラー・警告・変換結果を確認するための情報だけを作ります。

use super::mapping::*;
use super::records::*;
use super::*;
use rusqlite::Connection;
use serde_json::Value;
use std::collections::HashMap;

const PREVIEW_ROW_LIMIT: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportPreviewEffect {
    Insert,
    Update,
    Unchanged,
    Skip,
}

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
                    match import_preview_effect(
                        conn,
                        table_name,
                        columns,
                        &values,
                        row.row_number,
                        mode,
                    )? {
                        ImportPreviewEffect::Insert => {
                            inserted_count += 1;
                            push_preview_row(&mut preview_rows, || csv_preview_row(row, mapping));
                        }
                        ImportPreviewEffect::Update => {
                            updated_count += 1;
                            push_preview_row(&mut preview_rows, || csv_preview_row(row, mapping));
                        }
                        ImportPreviewEffect::Unchanged => unchanged_count += 1,
                        ImportPreviewEffect::Skip => skipped_count += 1,
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
                    match import_preview_effect(
                        conn,
                        table_name,
                        columns,
                        &values,
                        row.row_number,
                        mode,
                    )? {
                        ImportPreviewEffect::Insert => {
                            inserted_count += 1;
                            push_preview_row(&mut preview_rows, || excel_preview_row(row, mapping));
                        }
                        ImportPreviewEffect::Update => {
                            updated_count += 1;
                            push_preview_row(&mut preview_rows, || excel_preview_row(row, mapping));
                        }
                        ImportPreviewEffect::Unchanged => unchanged_count += 1,
                        ImportPreviewEffect::Skip => skipped_count += 1,
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

fn import_preview_effect(
    conn: &Connection,
    table_name: &str,
    columns: &[AppColumn],
    values: &HashMap<String, Value>,
    row_number: usize,
    mode: ImportTableCsvMode,
) -> Result<ImportPreviewEffect, DbError> {
    match mode {
        ImportTableCsvMode::SkipExistingPrimaryKeys => {
            let id = csv_row_id(values, row_number)?;
            if record_exists(conn, table_name, id)? {
                Ok(ImportPreviewEffect::Skip)
            } else {
                Ok(ImportPreviewEffect::Insert)
            }
        }
        ImportTableCsvMode::AppendIgnoringPrimaryKeys => Ok(ImportPreviewEffect::Insert),
        ImportTableCsvMode::UpsertByPrimaryKey => {
            let id = csv_row_id(values, row_number)?;
            if record_exists(conn, table_name, id)? {
                if record_values_match(conn, table_name, columns, id, values)? {
                    Ok(ImportPreviewEffect::Unchanged)
                } else {
                    Ok(ImportPreviewEffect::Update)
                }
            } else {
                Ok(ImportPreviewEffect::Insert)
            }
        }
    }
}

fn push_preview_row(
    preview_rows: &mut Vec<HashMap<String, String>>,
    build_row: impl FnOnce() -> HashMap<String, String>,
) {
    if preview_rows.len() < PREVIEW_ROW_LIMIT {
        preview_rows.push(build_row());
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_preview_db() -> (Connection, Vec<AppColumn>) {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE people (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO people (id, name) VALUES (1, 'Alice')", [])
            .unwrap();
        conn.execute("INSERT INTO people (id, name) VALUES (2, 'Bob')", [])
            .unwrap();

        let columns = vec![
            AppColumn {
                id: 1,
                table_id: 1,
                column_name: "id".into(),
                display_name: "ID".into(),
                field_type: "integer".into(),
                sort_order: 1,
                select_option_group_id: None,
                ref_table_id: None,
                is_required: true,
            },
            AppColumn {
                id: 2,
                table_id: 1,
                column_name: "name".into(),
                display_name: "Name".into(),
                field_type: "text".into(),
                sort_order: 2,
                select_option_group_id: None,
                ref_table_id: None,
                is_required: true,
            },
        ];

        (conn, columns)
    }

    fn mapping(columns: &[AppColumn]) -> Vec<ResolvedImportColumnMapping> {
        columns
            .iter()
            .map(|column| ResolvedImportColumnMapping {
                column: column.clone(),
                source_column_name: Some(column.column_name.clone()),
                matched_by: Some("exact".into()),
            })
            .collect()
    }

    fn row(row_number: usize, id: i64, name: &str) -> CsvSourceRow {
        CsvSourceRow {
            row_number,
            values: HashMap::from([
                ("id".into(), id.to_string()),
                ("name".into(), name.to_string()),
            ]),
        }
    }

    #[test]
    fn csv_preview_skip_existing_shows_only_rows_to_insert() {
        let (conn, columns) = setup_preview_db();
        let mapping = mapping(&columns);
        let rows = vec![row(2, 1, "Alice"), row(3, 3, "Carol")];

        let preview = build_csv_import_preview(
            &conn,
            "people",
            &columns,
            &["id".into(), "name".into()],
            &rows,
            &mapping,
            &HashMap::new(),
            ImportTableCsvMode::SkipExistingPrimaryKeys,
        )
        .unwrap();

        assert_eq!(preview.inserted_count, 1);
        assert_eq!(preview.skipped_count, 1);
        assert_eq!(preview.preview_rows.len(), 1);
        assert_eq!(preview.preview_rows[0].get("id").unwrap(), "3");
    }

    #[test]
    fn csv_preview_upsert_shows_only_rows_to_insert_or_update() {
        let (conn, columns) = setup_preview_db();
        let mapping = mapping(&columns);
        let rows = vec![row(2, 1, "Alice"), row(3, 2, "Bobby"), row(4, 3, "Carol")];

        let preview = build_csv_import_preview(
            &conn,
            "people",
            &columns,
            &["id".into(), "name".into()],
            &rows,
            &mapping,
            &HashMap::new(),
            ImportTableCsvMode::UpsertByPrimaryKey,
        )
        .unwrap();

        assert_eq!(preview.inserted_count, 1);
        assert_eq!(preview.updated_count, 1);
        assert_eq!(preview.unchanged_count, 1);
        assert_eq!(preview.preview_rows.len(), 2);
        assert_eq!(preview.preview_rows[0].get("id").unwrap(), "2");
        assert_eq!(preview.preview_rows[1].get("id").unwrap(), "3");
    }
}
