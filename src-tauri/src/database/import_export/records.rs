//! Import row conversion and record insert/update helpers.

use super::*;
use crate::database::validation::*;
use rusqlite::{params_from_iter, Connection, OptionalExtension};
use serde_json::Value;
use std::collections::HashMap;

pub(super) fn csv_row_to_values(
    row: &CsvSourceRow,
    columns: &[AppColumn],
    mapping: &[ResolvedImportColumnMapping],
    select_option_maps: &HashMap<String, HashMap<String, i64>>,
) -> Result<HashMap<String, Value>, DbError> {
    let mut values = HashMap::new();
    for column in columns {
        let source = mapping
            .iter()
            .find(|item| item.column.column_name == column.column_name)
            .and_then(|item| item.source_column_name.as_ref())
            .ok_or_else(|| {
                DbError::InvalidInput(format!(
                    "row {}: {} has no CSV column mapping",
                    row.row_number, column.display_name
                ))
            })?;
        let raw = row
            .values
            .get(source)
            .map(String::as_str)
            .unwrap_or_default();
        let value = csv_cell_to_value(column, raw, select_option_maps, row.row_number)?;
        if column.column_name != "id" && column.is_required && is_required_value_empty(Some(&value))
        {
            return Err(DbError::InvalidInput(format!(
                "row {}: {} is required",
                row.row_number, column.display_name
            )));
        }
        values.insert(column.column_name.clone(), value);
    }
    Ok(values)
}

pub(super) fn excel_row_to_values(
    row: &ExcelSourceRow,
    columns: &[AppColumn],
    mapping: &[ResolvedImportColumnMapping],
    select_option_maps: &HashMap<String, HashMap<String, i64>>,
) -> Result<HashMap<String, Value>, DbError> {
    let mut values = HashMap::new();
    for column in columns {
        let source = mapping
            .iter()
            .find(|item| item.column.column_name == column.column_name)
            .and_then(|item| item.source_column_name.as_ref())
            .ok_or_else(|| {
                DbError::InvalidInput(format!(
                    "row {}: {} has no Excel column mapping",
                    row.row_number, column.display_name
                ))
            })?;
        let raw = row
            .values
            .get(source)
            .map(String::as_str)
            .unwrap_or_default();
        let value = csv_cell_to_value(column, raw, select_option_maps, row.row_number)?;
        if column.column_name != "id" && column.is_required && is_required_value_empty(Some(&value))
        {
            return Err(DbError::InvalidInput(format!(
                "row {}: {} is required",
                row.row_number, column.display_name
            )));
        }
        values.insert(column.column_name.clone(), value);
    }
    Ok(values)
}

pub(super) fn record_exists(conn: &Connection, table_name: &str, id: i64) -> Result<bool, DbError> {
    conn.query_row(
        &format!("SELECT 1 FROM \"{}\" WHERE id = ?", table_name),
        [id],
        |_| Ok(()),
    )
    .optional()
    .map(|value| value.is_some())
    .map_err(DbError::from)
}

pub(super) fn record_values_match(
    conn: &Connection,
    table_name: &str,
    columns: &[AppColumn],
    id: i64,
    values: &HashMap<String, Value>,
) -> Result<bool, DbError> {
    let target_columns = columns
        .iter()
        .filter(|column| column.column_name != "id")
        .collect::<Vec<_>>();
    let select_columns = target_columns
        .iter()
        .map(|column| format!("\"{}\"", column.column_name))
        .collect::<Vec<_>>();
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM \"{}\" WHERE id = ?",
        select_columns.join(", "),
        table_name
    ))?;
    let current = stmt
        .query_row([id], |row| {
            let mut current = HashMap::new();
            for (index, column) in target_columns.iter().enumerate() {
                current.insert(
                    column.column_name.clone(),
                    sqlite_value_to_json(row.get_ref(index)?)?,
                );
            }
            Ok(current)
        })
        .optional()?;

    let Some(current) = current else {
        return Ok(false);
    };
    Ok(target_columns
        .iter()
        .all(|column| current.get(&column.column_name) == values.get(&column.column_name)))
}

fn csv_cell_to_value(
    column: &AppColumn,
    raw: &str,
    select_option_maps: &HashMap<String, HashMap<String, i64>>,
    row_number: usize,
) -> Result<Value, DbError> {
    let text = raw.trim();
    if text.is_empty() {
        // 空セルは未入力として扱い、必須チェックは呼び出し元で行います。
        return Ok(Value::Null);
    }

    match column.field_type.as_str() {
        "integer" | "date" => text.parse::<i64>().map(Value::from).map_err(|_| {
            DbError::InvalidInput(format!(
                "row {}: {} must be an integer",
                row_number, column.display_name
            ))
        }),
        "real" => text.parse::<f64>().map(Value::from).map_err(|_| {
            DbError::InvalidInput(format!(
                "row {}: {} must be a number",
                row_number, column.display_name
            ))
        }),
        "boolean" => match text {
            "true" | "1" => Ok(Value::from(true)),
            "false" | "0" => Ok(Value::from(false)),
            _ => Err(DbError::InvalidInput(format!(
                "row {}: {} must be true, false, 1, or 0",
                row_number, column.display_name
            ))),
        },
        "single_select" => {
            let option_no = select_option_maps
                .get(&column.column_name)
                .and_then(|options| options.get(text))
                .copied()
                .ok_or_else(|| {
                    DbError::InvalidInput(format!(
                        "row {}: {} does not match an option",
                        row_number, column.display_name
                    ))
                })?;
            Ok(Value::from(option_no))
        }
        "reference" => {
            // エクスポート値は「ID:表示名」なので、コロンより前のIDだけ取り出します。
            let id_text = text.split_once(':').map(|(id, _)| id).unwrap_or(text);
            id_text.trim().parse::<i64>().map(Value::from).map_err(|_| {
                DbError::InvalidInput(format!(
                    "row {}: {} must be a reference id",
                    row_number, column.display_name
                ))
            })
        }
        _ => Ok(Value::String(raw.to_string())),
    }
}

pub(super) fn csv_row_id(
    values: &HashMap<String, Value>,
    row_number: usize,
) -> Result<i64, DbError> {
    // IDを使う方式では、CSV行のidが空または数値以外なら処理できません。
    values
        .get("id")
        .and_then(Value::as_i64)
        .ok_or_else(|| DbError::InvalidInput(format!("row {}: id is required", row_number)))
}

pub(super) fn csv_record_exists(
    conn: &rusqlite::Transaction<'_>,
    table_name: &str,
    id: i64,
) -> Result<bool, DbError> {
    // 重複判定だけなので、存在すれば1行返す軽いSELECTにしています。
    conn.query_row(
        &format!("SELECT 1 FROM \"{}\" WHERE id = ?", table_name),
        [id],
        |_| Ok(()),
    )
    .optional()
    .map(|value| value.is_some())
    .map_err(DbError::from)
}

pub(super) fn csv_insert_record(
    conn: &rusqlite::Transaction<'_>,
    table_name: &str,
    columns: &[AppColumn],
    values: &HashMap<String, Value>,
    include_id: bool,
) -> Result<(), DbError> {
    // include_id=false のときはid列をINSERT対象から外し、SQLiteに自動採番させます。
    let target_columns = columns
        .iter()
        .filter(|column| include_id || column.column_name != "id")
        .collect::<Vec<_>>();
    let column_names = target_columns
        .iter()
        .map(|column| format!("\"{}\"", column.column_name))
        .collect::<Vec<_>>();
    let placeholders = std::iter::repeat("?")
        .take(target_columns.len())
        .collect::<Vec<_>>();
    let sql_values = target_columns
        .iter()
        .map(|column| to_sql_value(values.get(&column.column_name), &column.field_type))
        .collect::<Vec<_>>();
    conn.execute(
        &format!(
            "INSERT INTO \"{}\" ({}) VALUES ({})",
            table_name,
            column_names.join(", "),
            placeholders.join(", ")
        ),
        params_from_iter(sql_values.iter().map(|value| value.as_ref())),
    )?;
    Ok(())
}

pub(super) fn csv_update_record(
    conn: &rusqlite::Transaction<'_>,
    table_name: &str,
    columns: &[AppColumn],
    values: &HashMap<String, Value>,
    id: i64,
) -> Result<(), DbError> {
    // 置き換え方式でもidは変更せず、非IDカラムだけをCSV内容で更新します。
    let target_columns = columns
        .iter()
        .filter(|column| column.column_name != "id")
        .collect::<Vec<_>>();
    let assignments = target_columns
        .iter()
        .map(|column| format!("\"{}\" = ?", column.column_name))
        .collect::<Vec<_>>();
    let mut sql_values = target_columns
        .iter()
        .map(|column| to_sql_value(values.get(&column.column_name), &column.field_type))
        .collect::<Vec<_>>();
    sql_values.push(Box::new(id));
    conn.execute(
        &format!(
            "UPDATE \"{}\" SET {} WHERE id = ?",
            table_name,
            assignments.join(", ")
        ),
        params_from_iter(sql_values.iter().map(|value| value.as_ref())),
    )?;
    Ok(())
}
