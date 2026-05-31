//! Table, column, and record persistence operations.

use super::validation::*;
use super::*;
use rusqlite::{params, params_from_iter, OptionalExtension};
use serde_json::{Map, Value};
use std::collections::HashMap;

impl Db {
    pub fn bootstrap(&self) -> Result<AppBootstrap, DbError> {
        Ok(AppBootstrap {
            tables: self.list_tables()?,
            option_groups: self.list_option_groups()?,
        })
    }

    pub fn create_table(&self, payload: CreateTablePayload) -> Result<i64, DbError> {
        validate_identifier(&payload.table_name)?;
        let sort_order = self.next_sort_order("app_tables", None)?;
        self.conn.execute(
            "INSERT INTO app_tables (table_name, display_name, sort_order) VALUES (?, ?, ?)",
            params![payload.table_name, payload.display_name, sort_order],
        )?;
        let table_id = self.conn.last_insert_rowid();
        let table_name = self.table_name_by_id(table_id)?;
        self.conn.execute(
            &format!(
                "CREATE TABLE \"{}\" (id INTEGER PRIMARY KEY AUTOINCREMENT)",
                table_name
            ),
            [],
        )?;

        let sort_order = self.next_sort_order("app_table_columns", Some(table_id))?;
        self.conn.execute(
            "
            INSERT INTO app_table_columns (
              table_id, column_name, display_name, field_type, sort_order, is_required
            ) VALUES (?, 'id', 'ID', 'integer', ?, 1)
            ",
            params![table_id, sort_order],
        )?;
        let id_column_id = self.conn.last_insert_rowid();
        self.conn.execute(
            "UPDATE app_tables SET label_column_id = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            params![id_column_id, table_id],
        )?;
        Ok(table_id)
    }

    pub fn delete_table(&self, payload: DeleteTablePayload) -> Result<(), DbError> {
        let table = self.get_table_summary(payload.table_id)?;
        let referrers = self.table_reference_sources(payload.table_id)?;
        if !referrers.is_empty() {
            return Err(DbError::InvalidInput(format!(
                "他のテーブルから参照されているため削除できません: {}",
                referrers.join(", ")
            )));
        }

        self.conn.execute_batch("BEGIN IMMEDIATE TRANSACTION")?;
        let result = (|| -> Result<(), DbError> {
            self.conn.execute(
                "DELETE FROM record_tag_links WHERE table_id = ?",
                [payload.table_id],
            )?;
            self.conn.execute(
                "
                DELETE FROM view_layout_record_template_assignments
                WHERE folder_record_id IN (
                  SELECT id FROM view_nav_folder_records WHERE table_id = ?
                )
                ",
                [payload.table_id],
            )?;
            self.conn.execute(
                "DELETE FROM view_nav_folder_records WHERE table_id = ?",
                [payload.table_id],
            )?;
            self.conn.execute(
                "DELETE FROM view_layout_card_overrides WHERE table_id = ?",
                [payload.table_id],
            )?;
            self.conn.execute(
                "DELETE FROM view_layout_card_column_bindings WHERE table_id = ?",
                [payload.table_id],
            )?;
            self.conn.execute(
                "DELETE FROM app_table_columns WHERE table_id = ?",
                [payload.table_id],
            )?;
            self.conn
                .execute("DELETE FROM app_tables WHERE id = ?", [payload.table_id])?;
            self.conn
                .execute(&format!("DROP TABLE \"{}\"", table.table_name), [])?;
            Ok(())
        })();

        match result {
            Ok(()) => {
                self.conn.execute_batch("COMMIT")?;
                Ok(())
            }
            Err(error) => {
                let _ = self.conn.execute_batch("ROLLBACK");
                Err(error)
            }
        }
    }

    pub fn add_column(&self, payload: AddColumnPayload) -> Result<(), DbError> {
        validate_identifier(&payload.column_name)?;
        validate_field_type(&payload.field_type)?;
        if payload.field_type == "single_select" && payload.select_option_group_id.is_none() {
            return Err(DbError::InvalidInput(
                "single_select requires selectOptionGroupId".into(),
            ));
        }
        if payload.field_type == "reference" && payload.ref_table_id.is_none() {
            return Err(DbError::InvalidInput(
                "reference requires refTableId".into(),
            ));
        }

        let table_name = self.table_name_by_id(payload.table_id)?;
        let sort_order = self.next_sort_order("app_table_columns", Some(payload.table_id))?;
        let current_label_column_name = self.label_column_name(payload.table_id)?;
        let tx = self.conn.unchecked_transaction()?;
        // 物理列の追加とメタ情報登録をまとめ、途中失敗時に実テーブルだけ変わる状態を避けます。
        tx.execute(
            &format!(
                "ALTER TABLE \"{}\" ADD COLUMN \"{}\" {}",
                table_name,
                payload.column_name,
                sqlite_type_for(&payload.field_type)
            ),
            [],
        )?;

        tx.execute(
            "
            INSERT INTO app_table_columns (
              table_id, column_name, display_name, field_type, sort_order,
              select_option_group_id, ref_table_id, is_required
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ",
            params![
                payload.table_id,
                payload.column_name,
                payload.display_name,
                payload.field_type,
                sort_order,
                payload.select_option_group_id,
                payload.ref_table_id,
                bool_to_i64(payload.is_required)
            ],
        )?;

        if current_label_column_name == "id" && payload.column_name != "id" {
            let new_column_id = tx.last_insert_rowid();
            tx.execute(
                "UPDATE app_tables SET label_column_id = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                params![new_column_id, payload.table_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn delete_column(&self, payload: DeleteColumnPayload) -> Result<(), DbError> {
        let table_name = self.table_name_by_id(payload.table_id)?;
        let (column_name, current_label_column_id): (String, Option<i64>) = self.conn.query_row(
            "
            SELECT c.column_name, t.label_column_id
            FROM app_table_columns c
            JOIN app_tables t ON t.id = c.table_id
            WHERE c.id = ? AND c.table_id = ?
            ",
            params![payload.column_id, payload.table_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        if column_name == "id" {
            return Err(DbError::InvalidInput("id column cannot be deleted".into()));
        }

        self.conn.execute(
            &format!(
                "ALTER TABLE \"{}\" DROP COLUMN \"{}\"",
                table_name, column_name
            ),
            [],
        )?;

        self.conn.execute(
            "DELETE FROM app_table_columns WHERE id = ?",
            [payload.column_id],
        )?;

        if current_label_column_id == Some(payload.column_id) {
            let next_label_column_id: Option<i64> = self
                .conn
                .query_row(
                    "
                    SELECT id
                    FROM app_table_columns
                    WHERE table_id = ? AND column_name != 'id'
                    ORDER BY sort_order, id
                    LIMIT 1
                    ",
                    [payload.table_id],
                    |row| row.get(0),
                )
                .optional()?;

            self.conn.execute(
                "UPDATE app_tables SET label_column_id = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                params![next_label_column_id, payload.table_id],
            )?;
        }

        self.resequence_column_sort_order(payload.table_id)?;

        Ok(())
    }

    pub fn update_column(&self, payload: UpdateColumnPayload) -> Result<(), DbError> {
        if payload.display_name.trim().is_empty() {
            return Err(DbError::InvalidInput("display name is required".into()));
        }

        let table_name = self.table_name_by_id(payload.table_id)?;
        let (current_column_name, field_type): (String, String) = self.conn.query_row(
            "
            SELECT column_name, field_type
            FROM app_table_columns
            WHERE id = ? AND table_id = ?
            ",
            params![payload.column_id, payload.table_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        if current_column_name == "id" && payload.column_name != "id" {
            return Err(DbError::InvalidInput(
                "id column name cannot be changed".into(),
            ));
        }

        if current_column_name != payload.column_name {
            validate_identifier(&payload.column_name)?;
            self.conn.execute(
                &format!(
                    "ALTER TABLE \"{}\" RENAME COLUMN \"{}\" TO \"{}\"",
                    table_name, current_column_name, payload.column_name
                ),
                [],
            )?;
        }

        validate_field_type(&field_type)?;
        self.conn.execute(
            "
            UPDATE app_table_columns
            SET column_name = ?, display_name = ?, is_required = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ? AND table_id = ?
            ",
            params![
                payload.column_name,
                payload.display_name,
                bool_to_i64(payload.is_required),
                payload.column_id,
                payload.table_id
            ],
        )?;

        Ok(())
    }

    pub fn update_label_column(&self, payload: UpdateLabelColumnPayload) -> Result<(), DbError> {
        if let Some(label_column_id) = payload.label_column_id {
            let column_exists: Option<i64> = self
                .conn
                .query_row(
                    "SELECT id FROM app_table_columns WHERE id = ? AND table_id = ?",
                    params![label_column_id, payload.table_id],
                    |row| row.get(0),
                )
                .optional()?;

            if column_exists.is_none() {
                return Err(DbError::InvalidInput(
                    "label column must belong to the selected table".into(),
                ));
            }
        }

        self.conn.execute(
            "UPDATE app_tables SET label_column_id = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            params![payload.label_column_id, payload.table_id],
        )?;

        Ok(())
    }

    pub fn reorder_columns(&self, payload: ReorderColumnsPayload) -> Result<(), DbError> {
        let current_ids = self
            .list_columns(payload.table_id)?
            .into_iter()
            .map(|column| column.id)
            .collect::<Vec<_>>();

        if current_ids.len() != payload.ordered_column_ids.len() {
            return Err(DbError::InvalidInput(
                "orderedColumnIds length mismatch".into(),
            ));
        }

        let mut sorted_current = current_ids.clone();
        let mut sorted_requested = payload.ordered_column_ids.clone();
        sorted_current.sort_unstable();
        sorted_requested.sort_unstable();

        if sorted_current != sorted_requested {
            return Err(DbError::InvalidInput(
                "orderedColumnIds must match current table columns".into(),
            ));
        }

        for (index, column_id) in payload.ordered_column_ids.into_iter().enumerate() {
            self.conn.execute(
                "UPDATE app_table_columns SET sort_order = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND table_id = ?",
                params![index as i64 + 1, column_id, payload.table_id],
            )?;
        }

        Ok(())
    }

    pub fn save_option_group(&self, payload: SaveOptionGroupPayload) -> Result<i64, DbError> {
        if payload.name.trim().is_empty() {
            return Err(DbError::InvalidInput(
                "option group name is required".into(),
            ));
        }
        let group_id = if let Some(id) = payload.id {
            self.conn.execute(
                "UPDATE select_option_groups SET name = ?, description = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                params![payload.name, payload.description, id],
            )?;
            self.conn
                .execute("DELETE FROM select_options WHERE group_id = ?", [id])?;
            id
        } else {
            self.conn.execute(
                "INSERT INTO select_option_groups (name, description) VALUES (?, ?)",
                params![payload.name, payload.description],
            )?;
            self.conn.last_insert_rowid()
        };

        for option in payload
            .options
            .into_iter()
            .filter(|item| !item.label.trim().is_empty())
        {
            self.conn.execute(
                "INSERT INTO select_options (group_id, option_no, sort_order, label) VALUES (?, ?, ?, ?)",
                params![group_id, option.option_no, option.sort_order, option.label],
            )?;
        }
        Ok(group_id)
    }

    pub fn get_table_detail(&self, table_id: i64) -> Result<TableDetail, DbError> {
        let table = self.get_table_summary(table_id)?;
        let columns = self.list_columns(table_id)?;
        let records = self.list_records(&table.table_name, &columns)?;
        Ok(TableDetail {
            table,
            columns,
            records,
        })
    }

    pub fn save_record(&self, payload: SaveRecordPayload) -> Result<(), DbError> {
        let table = self.get_table_summary(payload.table_id)?;
        let columns = self
            .list_columns(payload.table_id)?
            .into_iter()
            .filter(|column| column.column_name != "id")
            .collect::<Vec<_>>();
        if columns.is_empty() {
            return Ok(());
        }

        let object = payload
            .values
            .as_object()
            .ok_or_else(|| DbError::InvalidInput("values must be an object".into()))?;

        if let Some(column) = columns.iter().find(|column| {
            column.is_required && is_required_value_empty(object.get(&column.column_name))
        }) {
            return Err(DbError::InvalidInput(format!(
                "{} is required",
                column.display_name
            )));
        }

        if let Some(record_id) = payload.record_id {
            let assignments = columns
                .iter()
                .map(|column| format!("\"{}\" = ?", column.column_name))
                .collect::<Vec<_>>();
            let mut values = columns
                .iter()
                .map(|column| to_sql_value(object.get(&column.column_name), &column.field_type))
                .collect::<Vec<_>>();
            values.push(Box::new(record_id));
            self.conn.execute(
                &format!(
                    "UPDATE \"{}\" SET {} WHERE id = ?",
                    table.table_name,
                    assignments.join(", ")
                ),
                params_from_iter(values.iter().map(|value| value.as_ref())),
            )?;
        } else {
            let column_names = columns
                .iter()
                .map(|column| format!("\"{}\"", column.column_name))
                .collect::<Vec<_>>();
            let placeholders = std::iter::repeat("?")
                .take(columns.len())
                .collect::<Vec<_>>();
            let values = columns
                .iter()
                .map(|column| to_sql_value(object.get(&column.column_name), &column.field_type))
                .collect::<Vec<_>>();
            self.conn.execute(
                &format!(
                    "INSERT INTO \"{}\" ({}) VALUES ({})",
                    table.table_name,
                    column_names.join(", "),
                    placeholders.join(", ")
                ),
                params_from_iter(values.iter().map(|value| value.as_ref())),
            )?;
        }
        Ok(())
    }

    pub fn delete_record(&self, payload: DeleteRecordPayload) -> Result<(), DbError> {
        let table = self.get_table_summary(payload.table_id)?;

        // レコード本体を消す前に、タグ紐付けと閲覧ナビ上の配置を削除します。
        self.conn.execute(
            "DELETE FROM record_tag_links WHERE table_id = ? AND record_id = ?",
            params![payload.table_id, payload.record_id],
        )?;
        self.conn.execute(
            "
            DELETE FROM view_layout_record_template_assignments
            WHERE folder_record_id IN (
              SELECT id FROM view_nav_folder_records
              WHERE table_id = ? AND record_id = ?
            )
            ",
            params![payload.table_id, payload.record_id],
        )?;
        self.conn.execute(
            "DELETE FROM view_nav_folder_records WHERE table_id = ? AND record_id = ?",
            params![payload.table_id, payload.record_id],
        )?;
        self.conn.execute(
            &format!("DELETE FROM \"{}\" WHERE id = ?", table.table_name),
            params![payload.record_id],
        )?;

        Ok(())
    }

    pub fn get_reference_choices(&self, table_id: i64) -> Result<Vec<ReferenceChoice>, DbError> {
        let table = self.get_table_summary(table_id)?;
        let label_column = self.label_column_name(table_id)?;
        let mut stmt = self.conn.prepare(&format!(
            "SELECT id, COALESCE(CAST(\"{}\" AS TEXT), '') FROM \"{}\" ORDER BY id",
            label_column, table.table_name
        ))?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let label: String = row.get(1)?;
            Ok(ReferenceChoice {
                id,
                label: format!("{}:{}", id, label),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub(super) fn list_tables(&self) -> Result<Vec<AppTableSummary>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, table_name, display_name, label_column_id, sort_order FROM app_tables ORDER BY sort_order, id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(AppTableSummary {
                id: row.get(0)?,
                table_name: row.get(1)?,
                display_name: row.get(2)?,
                label_column_id: row.get(3)?,
                sort_order: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub(super) fn get_table_summary(&self, table_id: i64) -> Result<AppTableSummary, DbError> {
        self.conn
            .query_row(
                "SELECT id, table_name, display_name, label_column_id, sort_order FROM app_tables WHERE id = ?",
                [table_id],
                |row| {
                    Ok(AppTableSummary {
                        id: row.get(0)?,
                        table_name: row.get(1)?,
                        display_name: row.get(2)?,
                        label_column_id: row.get(3)?,
                        sort_order: row.get(4)?,
                    })
                },
            )
            .map_err(DbError::from)
    }

    pub(super) fn list_columns(&self, table_id: i64) -> Result<Vec<AppColumn>, DbError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT id, table_id, column_name, display_name, field_type, sort_order,
                   select_option_group_id, ref_table_id, is_required
            FROM app_table_columns
            WHERE table_id = ?
            ORDER BY sort_order, id
            ",
        )?;
        let rows = stmt.query_map([table_id], |row| {
            Ok(AppColumn {
                id: row.get(0)?,
                table_id: row.get(1)?,
                column_name: row.get(2)?,
                display_name: row.get(3)?,
                field_type: row.get(4)?,
                sort_order: row.get(5)?,
                select_option_group_id: row.get(6)?,
                ref_table_id: row.get(7)?,
                is_required: row.get::<_, i64>(8)? != 0,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub(super) fn list_option_groups(&self) -> Result<Vec<SelectOptionGroup>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, description FROM select_option_groups ORDER BY name")?;
        let groups = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;

        let mut result = Vec::new();
        for group in groups {
            let (id, name, description) = group?;
            result.push(SelectOptionGroup {
                id,
                name,
                description,
                options: self.list_options(id)?,
            });
        }
        Ok(result)
    }

    pub(super) fn list_options(&self, group_id: i64) -> Result<Vec<SelectOption>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, group_id, option_no, sort_order, label FROM select_options WHERE group_id = ? ORDER BY sort_order, option_no",
        )?;
        let rows = stmt.query_map([group_id], |row| {
            Ok(SelectOption {
                id: row.get(0)?,
                group_id: row.get(1)?,
                option_no: row.get(2)?,
                sort_order: row.get(3)?,
                label: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub(super) fn list_records(
        &self,
        table_name: &str,
        columns: &[AppColumn],
    ) -> Result<Vec<TableRecord>, DbError> {
        self.list_records_with_order(table_name, columns, "ASC")
    }

    pub(super) fn list_records_with_order(
        &self,
        table_name: &str,
        columns: &[AppColumn],
        id_order: &str,
    ) -> Result<Vec<TableRecord>, DbError> {
        let selected_columns = columns
            .iter()
            .map(|column| format!("\"{}\"", column.column_name))
            .collect::<Vec<_>>();
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {} FROM \"{}\" ORDER BY id {}",
            selected_columns.join(", "),
            table_name,
            id_order
        ))?;
        let mut rows = stmt.query([])?;
        let mut records = Vec::new();

        while let Some(row) = rows.next()? {
            let mut values = Map::new();
            let mut display_values = Map::new();
            let mut record_id = 0;

            for (index, column) in columns.iter().enumerate() {
                let value = sqlite_value_to_json(row.get_ref(index)?)?;
                if column.column_name == "id" {
                    record_id = value.as_i64().unwrap_or_default();
                }
                let display = self.display_value_for_column(column, &value)?;
                values.insert(column.column_name.clone(), value);
                display_values.insert(column.column_name.clone(), Value::String(display));
            }

            records.push(TableRecord {
                id: record_id,
                values: Value::Object(values),
                display_values: Value::Object(display_values),
            });
        }

        Ok(records)
    }

    pub(super) fn display_value_for_column(
        &self,
        column: &AppColumn,
        value: &Value,
    ) -> Result<String, DbError> {
        if value.is_null() {
            return Ok(String::new());
        }

        match column.field_type.as_str() {
            "boolean" => Ok(if value.as_i64().unwrap_or_default() == 0 {
                "false".into()
            } else {
                "true".into()
            }),
            "single_select" => {
                let group_id = column.select_option_group_id.ok_or_else(|| {
                    DbError::InvalidInput("single_select group is missing".into())
                })?;
                let option_no = value.as_i64().unwrap_or_default();
                let label: Option<String> = self
                    .conn
                    .query_row(
                        "SELECT label FROM select_options WHERE group_id = ? AND option_no = ?",
                        params![group_id, option_no],
                        |row| row.get(0),
                    )
                    .optional()?;
                Ok(label.unwrap_or_else(|| option_no.to_string()))
            }
            "reference" => {
                let ref_table_id = column
                    .ref_table_id
                    .ok_or_else(|| DbError::InvalidInput("reference target is missing".into()))?;
                let table = self.get_table_summary(ref_table_id)?;
                let label_column = self.label_column_name(ref_table_id)?;
                let ref_id = value.as_i64().unwrap_or_default();
                let label: Option<String> = self
                    .conn
                    .query_row(
                        &format!(
                            "SELECT COALESCE(CAST(\"{}\" AS TEXT), '') FROM \"{}\" WHERE id = ?",
                            label_column, table.table_name
                        ),
                        [ref_id],
                        |row| row.get(0),
                    )
                    .optional()?;
                Ok(format!("{}:{}", ref_id, label.unwrap_or_default()))
            }
            _ => Ok(match value {
                Value::String(text) => text.clone(),
                _ => value.to_string(),
            }),
        }
    }

    pub(super) fn csv_select_option_maps(
        &self,
        columns: &[AppColumn],
    ) -> Result<HashMap<String, HashMap<String, i64>>, DbError> {
        let mut maps = HashMap::new();
        // CSVには選択肢ラベルが入ることがあるため、ラベル/番号の両方からoption_noを引けるようにします。
        for column in columns
            .iter()
            .filter(|column| column.field_type == "single_select")
        {
            let group_id = column.select_option_group_id.ok_or_else(|| {
                DbError::InvalidInput(format!(
                    "{} is single_select but option group is missing",
                    column.display_name
                ))
            })?;
            let mut stmt = self
                .conn
                .prepare("SELECT option_no, label FROM select_options WHERE group_id = ?")?;
            let options = stmt.query_map([group_id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?;

            let mut option_map = HashMap::new();
            for option in options {
                let (option_no, label) = option?;
                option_map.insert(option_no.to_string(), option_no);
                option_map.insert(label, option_no);
            }
            maps.insert(column.column_name.clone(), option_map);
        }
        Ok(maps)
    }

    pub(super) fn table_name_by_id(&self, table_id: i64) -> Result<String, DbError> {
        self.conn
            .query_row(
                "SELECT table_name FROM app_tables WHERE id = ?",
                [table_id],
                |row| row.get(0),
            )
            .map_err(DbError::from)
    }

    pub(super) fn table_reference_sources(&self, table_id: i64) -> Result<Vec<String>, DbError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT t.display_name, t.table_name, c.display_name, c.column_name
            FROM app_table_columns c
            JOIN app_tables t ON t.id = c.table_id
            WHERE c.ref_table_id = ?
            ORDER BY t.sort_order, t.id, c.sort_order, c.id
            ",
        )?;
        let rows = stmt.query_map([table_id], |row| {
            let table_display_name: String = row.get(0)?;
            let table_name: String = row.get(1)?;
            let column_display_name: String = row.get(2)?;
            let column_name: String = row.get(3)?;
            Ok(format!(
                "{} ({}) / {} ({})",
                table_display_name, table_name, column_display_name, column_name
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub(super) fn label_column_name(&self, table_id: i64) -> Result<String, DbError> {
        let label_column_id: Option<i64> = self
            .conn
            .query_row(
                "SELECT label_column_id FROM app_tables WHERE id = ?",
                [table_id],
                |row| row.get(0),
            )
            .optional()?
            .flatten();

        if let Some(column_id) = label_column_id {
            let name: Option<String> = self
                .conn
                .query_row(
                    "SELECT column_name FROM app_table_columns WHERE id = ?",
                    [column_id],
                    |row| row.get(0),
                )
                .optional()?;
            let resolved_name = name.unwrap_or_else(|| "id".into());
            if resolved_name != "id" {
                return Ok(resolved_name);
            }
        } else {
            return self
                .first_non_id_column_name(table_id)
                .map(|name| name.unwrap_or_else(|| "id".into()));
        }

        Ok(self
            .first_non_id_column_name(table_id)?
            .unwrap_or_else(|| "id".into()))
    }

    pub(super) fn next_sort_order(
        &self,
        target: &str,
        table_id: Option<i64>,
    ) -> Result<i64, DbError> {
        match target {
            "app_tables" => self
                .conn
                .query_row("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM app_tables", [], |row| row.get(0))
                .map_err(DbError::from),
            "app_table_columns" => self
                .conn
                .query_row(
                    "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM app_table_columns WHERE table_id = ?",
                    [table_id.ok_or_else(|| DbError::InvalidInput("table_id is required".into()))?],
                    |row| row.get(0),
                )
                .map_err(DbError::from),
            _ => Err(DbError::InvalidInput("unsupported sort target".into())),
        }
    }

    pub(super) fn resequence_column_sort_order(&self, table_id: i64) -> Result<(), DbError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT id
            FROM app_table_columns
            WHERE table_id = ?
            ORDER BY sort_order, id
            ",
        )?;
        let column_ids = stmt
            .query_map([table_id], |row| row.get::<_, i64>(0))?
            .collect::<Result<Vec<_>, _>>()?;

        for (index, column_id) in column_ids.into_iter().enumerate() {
            self.conn.execute(
                "UPDATE app_table_columns SET sort_order = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                params![index as i64 + 1, column_id],
            )?;
        }

        Ok(())
    }

    pub(super) fn first_non_id_column_name(
        &self,
        table_id: i64,
    ) -> Result<Option<String>, DbError> {
        self.conn
            .query_row(
                "
                SELECT column_name
                FROM app_table_columns
                WHERE table_id = ? AND column_name != 'id'
                ORDER BY sort_order, id
                LIMIT 1
                ",
                [table_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(DbError::from)
    }
}
