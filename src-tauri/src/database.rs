//! SQLite backend module facade.
//!
//! Public IPC types and Db methods are split by responsibility under database/.
mod import_export;
mod models;
mod schema;
mod settings;
mod validation;

pub use models::*;

use rusqlite::{params, params_from_iter, types::ValueRef, Connection, OptionalExtension, ToSql};
use serde_json::{Map, Value};
use std::{
    collections::{HashMap, HashSet},
    fs,
};

use validation::*;

fn require_trimmed<'a>(field_name: &str, value: &'a str) -> Result<&'a str, DbError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DbError::InvalidInput(format!("{field_name} is required")));
    }
    Ok(trimmed)
}

fn parse_group_ids(value: Option<String>) -> Vec<i64> {
    value
        .unwrap_or_default()
        .split(',')
        .filter_map(|item| item.parse::<i64>().ok())
        .collect()
}

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

    pub fn list_view_nav_nodes(&self) -> Result<Vec<ViewNavNode>, DbError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT id, node_type, parent_id, name, sort_order, created_at, updated_at
            FROM view_nav_nodes
            ORDER BY COALESCE(parent_id, 0), sort_order, id
            ",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ViewNavNode {
                id: row.get(0)?,
                node_type: row.get(1)?,
                parent_id: row.get(2)?,
                name: row.get(3)?,
                sort_order: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub fn create_view_nav_folder(
        &self,
        payload: CreateViewNavFolderPayload,
    ) -> Result<ViewNavNode, DbError> {
        let name = payload.name.trim();
        if name.is_empty() {
            return Err(DbError::InvalidInput("folder name is required".into()));
        }

        if let Some(parent_id) = payload.parent_id {
            let parent_exists: Option<i64> = self
                .conn
                .query_row(
                    "SELECT id FROM view_nav_nodes WHERE id = ? AND node_type = 'folder'",
                    [parent_id],
                    |row| row.get(0),
                )
                .optional()?;

            if parent_exists.is_none() {
                return Err(DbError::InvalidInput("parent folder does not exist".into()));
            }
        }

        let sort_order = self.next_view_nav_sort_order(payload.parent_id)?;
        self.conn.execute(
            "
            INSERT INTO view_nav_nodes (node_type, parent_id, name, sort_order)
            VALUES ('folder', ?, ?, ?)
            ",
            params![payload.parent_id, name, sort_order],
        )?;

        let node_id = self.conn.last_insert_rowid();
        self.conn
            .query_row(
                "
                SELECT id, node_type, parent_id, name, sort_order, created_at, updated_at
                FROM view_nav_nodes
                WHERE id = ?
                ",
                [node_id],
                |row| {
                    Ok(ViewNavNode {
                        id: row.get(0)?,
                        node_type: row.get(1)?,
                        parent_id: row.get(2)?,
                        name: row.get(3)?,
                        sort_order: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                },
            )
            .map_err(DbError::from)
    }

    pub fn delete_view_nav_folder(
        &self,
        payload: DeleteViewNavFolderPayload,
    ) -> Result<(), DbError> {
        let folder_exists: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM view_nav_nodes WHERE id = ? AND node_type = 'folder'",
                [payload.folder_id],
                |row| row.get(0),
            )
            .optional()?;

        if folder_exists.is_none() {
            return Err(DbError::InvalidInput("folder does not exist".into()));
        }

        let tx = self.conn.unchecked_transaction()?;

        tx.execute(
            "
            WITH RECURSIVE descendants(id) AS (
              SELECT id FROM view_nav_nodes WHERE id = ?
              UNION ALL
              SELECT child.id
              FROM view_nav_nodes child
              INNER JOIN descendants ON child.parent_id = descendants.id
            ),
            folder_templates(id) AS (
              SELECT id
              FROM view_layout_templates
              WHERE scope_type = 'folder'
                AND folder_id IN (SELECT id FROM descendants)
            )
            DELETE FROM view_layout_card_overrides
            WHERE template_id IN (SELECT id FROM folder_templates)
            ",
            [payload.folder_id],
        )?;

        tx.execute(
            "
            WITH RECURSIVE descendants(id) AS (
              SELECT id FROM view_nav_nodes WHERE id = ?
              UNION ALL
              SELECT child.id
              FROM view_nav_nodes child
              INNER JOIN descendants ON child.parent_id = descendants.id
            ),
            folder_templates(id) AS (
              SELECT id
              FROM view_layout_templates
              WHERE scope_type = 'folder'
                AND folder_id IN (SELECT id FROM descendants)
            )
            DELETE FROM view_layout_record_template_assignments
            WHERE folder_record_id IN (
              SELECT id
              FROM view_nav_folder_records
              WHERE folder_id IN (SELECT id FROM descendants)
            )
              OR template_id IN (SELECT id FROM folder_templates)
            ",
            [payload.folder_id],
        )?;

        tx.execute(
            "
            WITH RECURSIVE descendants(id) AS (
              SELECT id FROM view_nav_nodes WHERE id = ?
              UNION ALL
              SELECT child.id
              FROM view_nav_nodes child
              INNER JOIN descendants ON child.parent_id = descendants.id
            ),
            folder_templates(id) AS (
              SELECT id
              FROM view_layout_templates
              WHERE scope_type = 'folder'
                AND folder_id IN (SELECT id FROM descendants)
            )
            DELETE FROM view_layout_card_column_bindings
            WHERE template_id IN (SELECT id FROM folder_templates)
            ",
            [payload.folder_id],
        )?;

        tx.execute(
            "
            WITH RECURSIVE descendants(id) AS (
              SELECT id FROM view_nav_nodes WHERE id = ?
              UNION ALL
              SELECT child.id
              FROM view_nav_nodes child
              INNER JOIN descendants ON child.parent_id = descendants.id
            ),
            folder_templates(id) AS (
              SELECT id
              FROM view_layout_templates
              WHERE scope_type = 'folder'
                AND folder_id IN (SELECT id FROM descendants)
            )
            DELETE FROM view_layout_template_cards
            WHERE template_id IN (SELECT id FROM folder_templates)
            ",
            [payload.folder_id],
        )?;

        tx.execute(
            "
            WITH RECURSIVE descendants(id) AS (
              SELECT id FROM view_nav_nodes WHERE id = ?
              UNION ALL
              SELECT child.id
              FROM view_nav_nodes child
              INNER JOIN descendants ON child.parent_id = descendants.id
            ),
            folder_templates(id) AS (
              SELECT id
              FROM view_layout_templates
              WHERE scope_type = 'folder'
                AND folder_id IN (SELECT id FROM descendants)
            )
            DELETE FROM view_layout_folder_template_assignments
            WHERE folder_id IN (SELECT id FROM descendants)
              OR template_id IN (SELECT id FROM folder_templates)
            ",
            [payload.folder_id],
        )?;

        tx.execute(
            "
            WITH RECURSIVE descendants(id) AS (
              SELECT id FROM view_nav_nodes WHERE id = ?
              UNION ALL
              SELECT child.id
              FROM view_nav_nodes child
              INNER JOIN descendants ON child.parent_id = descendants.id
            )
            DELETE FROM view_layout_templates
            WHERE scope_type = 'folder'
              AND folder_id IN (SELECT id FROM descendants)
            ",
            [payload.folder_id],
        )?;

        tx.execute(
            "
            WITH RECURSIVE descendants(id) AS (
              SELECT id FROM view_nav_nodes WHERE id = ?
              UNION ALL
              SELECT child.id
              FROM view_nav_nodes child
              INNER JOIN descendants ON child.parent_id = descendants.id
            )
            DELETE FROM view_nav_folder_records
            WHERE folder_id IN (SELECT id FROM descendants)
            ",
            [payload.folder_id],
        )?;

        tx.execute(
            "
            WITH RECURSIVE descendants(id) AS (
              SELECT id FROM view_nav_nodes WHERE id = ?
              UNION ALL
              SELECT child.id
              FROM view_nav_nodes child
              INNER JOIN descendants ON child.parent_id = descendants.id
            )
            DELETE FROM view_nav_nodes
            WHERE id IN (SELECT id FROM descendants)
            ",
            [payload.folder_id],
        )?;

        tx.commit()?;

        Ok(())
    }

    pub fn list_view_nav_folder_records(&self) -> Result<Vec<ViewNavFolderRecord>, DbError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT
              r.id,
              r.folder_id,
              r.table_id,
              t.table_name,
              t.display_name,
              r.record_id,
              r.record_label,
              assignment.template_id,
              r.sort_order,
              r.created_at,
              r.updated_at
            FROM view_nav_folder_records r
            JOIN app_tables t ON t.id = r.table_id
            LEFT JOIN view_layout_record_template_assignments assignment
              ON assignment.folder_record_id = r.id
            ORDER BY r.folder_id, r.sort_order, r.id
            ",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ViewNavFolderRecord {
                id: row.get(0)?,
                folder_id: row.get(1)?,
                table_id: row.get(2)?,
                table_name: row.get(3)?,
                table_display_name: row.get(4)?,
                record_id: row.get(5)?,
                record_label: row.get(6)?,
                record_template_id: row.get(7)?,
                sort_order: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub fn add_view_nav_folder_records(
        &self,
        payload: AddViewNavFolderRecordsPayload,
    ) -> Result<Vec<ViewNavFolderRecord>, DbError> {
        self.ensure_view_nav_folder(payload.folder_id)?;
        let table = self.get_table_summary(payload.table_id)?;

        let mut seen_record_ids = HashSet::new();
        let mut pending_records = Vec::new();
        for record in payload.records {
            if !seen_record_ids.insert(record.record_id) {
                continue;
            }

            if self.view_nav_folder_record_exists(
                payload.folder_id,
                payload.table_id,
                record.record_id,
            )? {
                continue;
            }

            let record_label = record.record_label.trim().to_string();
            if record_label.is_empty() {
                return Err(DbError::InvalidInput("record label is required".into()));
            }

            self.ensure_table_record_exists(&table.table_name, record.record_id)?;
            pending_records.push((record.record_id, record_label));
        }

        if pending_records.is_empty() {
            return Ok(Vec::new());
        }

        let first_sort_order = self.next_view_nav_folder_record_sort_order(payload.folder_id)?;
        let tx = self.conn.unchecked_transaction()?;
        let mut created_ids = Vec::with_capacity(pending_records.len());
        {
            let mut stmt = tx.prepare(
                "
                INSERT INTO view_nav_folder_records (
                  folder_id, table_id, record_id, record_label, sort_order
                )
                VALUES (?, ?, ?, ?, ?)
                ",
            )?;

            for (index, (record_id, record_label)) in pending_records.iter().enumerate() {
                stmt.execute(params![
                    payload.folder_id,
                    payload.table_id,
                    record_id,
                    record_label,
                    first_sort_order + index as i64
                ])?;
                created_ids.push(tx.last_insert_rowid());
            }
        }
        tx.commit()?;

        created_ids
            .into_iter()
            .map(|folder_record_id| self.get_view_nav_folder_record(folder_record_id))
            .collect()
    }

    pub fn remove_view_nav_folder_record(
        &self,
        payload: RemoveViewNavFolderRecordPayload,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM view_layout_record_template_assignments WHERE folder_record_id = ?",
            [payload.folder_record_id],
        )?;

        let affected = self.conn.execute(
            "DELETE FROM view_nav_folder_records WHERE id = ?",
            [payload.folder_record_id],
        )?;

        if affected == 0 {
            return Err(DbError::InvalidInput("folder record does not exist".into()));
        }

        Ok(())
    }

    pub fn reorder_view_nav_folder_records(
        &self,
        payload: ReorderViewNavFolderRecordsPayload,
    ) -> Result<Vec<ViewNavFolderRecord>, DbError> {
        // フォルダー内の全レコードが一度ずつ含まれる順序だけを受け付け、sort_order を再採番します。
        self.ensure_view_nav_folder(payload.folder_id)?;

        // 既存のフォルダー内レコード集合と、フロントから届いた ID 集合が完全一致するか確認します。
        // これにより、別フォルダーの ID 混入や一部欠落、重複指定をここで止めます。
        let current_ids = self.list_view_nav_folder_record_ids(payload.folder_id)?;
        let current_id_set: HashSet<i64> = current_ids.iter().copied().collect();
        let ordered_id_set: HashSet<i64> =
            payload.ordered_folder_record_ids.iter().copied().collect();

        if ordered_id_set.len() != payload.ordered_folder_record_ids.len()
            || current_id_set != ordered_id_set
        {
            return Err(DbError::InvalidInput(
                "folder record order must include all records in the folder exactly once".into(),
            ));
        }

        // 途中で失敗したときに順序が半端に保存されないよう、更新は 1 トランザクションで行います。
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(
                "
                UPDATE view_nav_folder_records
                SET sort_order = ?, updated_at = CURRENT_TIMESTAMP
                WHERE id = ? AND folder_id = ?
                ",
            )?;

            for (index, folder_record_id) in payload.ordered_folder_record_ids.iter().enumerate() {
                // sort_order は表示順として扱いやすいよう、0 始まりではなく 1 始まりにそろえます。
                let affected = stmt.execute(params![
                    index as i64 + 1,
                    folder_record_id,
                    payload.folder_id
                ])?;

                if affected == 0 {
                    return Err(DbError::InvalidInput(
                        "folder record does not exist in the target folder".into(),
                    ));
                }
            }
        }
        tx.commit()?;

        // 保存後の sort_order をフロント状態へ同期できるよう、更新済みの一覧を返します。
        self.list_view_nav_folder_records_for_folder(payload.folder_id)
    }

    pub fn list_record_tags(&self) -> Result<RecordTagBundle, DbError> {
        Ok(RecordTagBundle {
            groups: self.list_record_tag_groups()?,
            tags: self.list_record_tag_items()?,
        })
    }

    pub fn list_record_tags_for_record(
        &self,
        table_id: i64,
        record_id: i64,
    ) -> Result<Vec<RecordTag>, DbError> {
        let table = self.get_table_summary(table_id)?;
        self.ensure_table_record_exists(&table.table_name, record_id)?;

        let mut stmt = self.conn.prepare(
            "
            SELECT
              tag.id,
              tag.group_id,
              GROUP_CONCAT(DISTINCT group_link.group_id) AS group_ids,
              tag.name,
              tag.sort_order,
              COUNT(DISTINCT all_links.id) AS usage_count,
              tag.created_at,
              tag.updated_at
            FROM record_tags tag
            JOIN record_tag_links selected_link
              ON selected_link.tag_id = tag.id
             AND selected_link.table_id = ?
             AND selected_link.record_id = ?
            LEFT JOIN record_tag_links all_links ON all_links.tag_id = tag.id
            LEFT JOIN record_tag_group_links group_link ON group_link.tag_id = tag.id
            GROUP BY tag.id
            ORDER BY tag.sort_order, tag.name, tag.id
            ",
        )?;
        let rows = stmt.query_map(params![table_id, record_id], |row| {
            Ok(RecordTag {
                id: row.get(0)?,
                group_id: row.get(1)?,
                group_ids: parse_group_ids(row.get(2)?),
                name: row.get(3)?,
                sort_order: row.get(4)?,
                usage_count: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub fn save_record_tag_group(
        &self,
        payload: SaveRecordTagGroupPayload,
    ) -> Result<RecordTagGroup, DbError> {
        let name = require_trimmed("tag group name", &payload.name)?;

        let group_id = match payload.id {
            Some(group_id) => {
                self.ensure_record_tag_group(group_id)?;
                self.conn.execute(
                    "UPDATE record_tag_groups SET name = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                    params![name, group_id],
                )?;
                group_id
            }
            None => {
                let sort_order = self.next_record_tag_group_sort_order()?;
                self.conn.execute(
                    "INSERT INTO record_tag_groups (name, sort_order) VALUES (?, ?)",
                    params![name, sort_order],
                )?;
                self.conn.last_insert_rowid()
            }
        };

        self.get_record_tag_group(group_id)
    }

    pub fn save_record_tag(&self, payload: SaveRecordTagPayload) -> Result<RecordTag, DbError> {
        let name = require_trimmed("tag name", &payload.name)?;
        if let Some(group_id) = payload.group_id {
            self.ensure_record_tag_group(group_id)?;
        }

        let tag_id = match payload.id {
            Some(tag_id) => {
                self.ensure_record_tag(tag_id)?;
                self.conn.execute(
                    "
                    UPDATE record_tags
                    SET name = ?, updated_at = CURRENT_TIMESTAMP
                    WHERE id = ?
                    ",
                    params![name, tag_id],
                )?;
                if let Some(group_id) = payload.group_id {
                    self.attach_record_tag_group(RecordTagGroupLinkPayload { tag_id, group_id })?;
                }
                tag_id
            }
            None => {
                let sort_order = self.next_record_tag_sort_order(payload.group_id)?;
                self.conn.execute(
                    "INSERT INTO record_tags (name, group_id, sort_order) VALUES (?, ?, ?)",
                    params![name, payload.group_id, sort_order],
                )?;
                let tag_id = self.conn.last_insert_rowid();
                if let Some(group_id) = payload.group_id {
                    self.attach_record_tag_group(RecordTagGroupLinkPayload { tag_id, group_id })?;
                }
                tag_id
            }
        };

        self.get_record_tag(tag_id)
    }

    pub fn delete_record_tag(&self, payload: DeleteRecordTagPayload) -> Result<(), DbError> {
        self.ensure_record_tag(payload.tag_id)?;
        self.conn.execute(
            "DELETE FROM record_tag_group_links WHERE tag_id = ?",
            [payload.tag_id],
        )?;
        self.conn.execute(
            "DELETE FROM record_tag_links WHERE tag_id = ?",
            [payload.tag_id],
        )?;
        self.conn
            .execute("DELETE FROM record_tags WHERE id = ?", [payload.tag_id])?;
        Ok(())
    }

    pub fn attach_record_tag_group(
        &self,
        payload: RecordTagGroupLinkPayload,
    ) -> Result<RecordTag, DbError> {
        self.ensure_record_tag(payload.tag_id)?;
        self.ensure_record_tag_group(payload.group_id)?;
        self.conn.execute(
            "
            INSERT OR IGNORE INTO record_tag_group_links (tag_id, group_id)
            VALUES (?, ?)
            ",
            params![payload.tag_id, payload.group_id],
        )?;
        self.conn.execute(
            "
            UPDATE record_tags
            SET group_id = COALESCE(group_id, ?), updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            ",
            params![payload.group_id, payload.tag_id],
        )?;
        self.get_record_tag(payload.tag_id)
    }

    pub fn detach_record_tag_group(
        &self,
        payload: RecordTagGroupLinkPayload,
    ) -> Result<RecordTag, DbError> {
        self.ensure_record_tag(payload.tag_id)?;
        self.ensure_record_tag_group(payload.group_id)?;
        self.conn.execute(
            "
            DELETE FROM record_tag_group_links
            WHERE tag_id = ? AND group_id = ?
            ",
            params![payload.tag_id, payload.group_id],
        )?;
        let next_group_id: Option<i64> = self
            .conn
            .query_row(
                "
                SELECT group_id
                FROM record_tag_group_links
                WHERE tag_id = ?
                ORDER BY group_id
                LIMIT 1
                ",
                [payload.tag_id],
                |row| row.get(0),
            )
            .optional()?;
        self.conn.execute(
            "
            UPDATE record_tags
            SET group_id = CASE WHEN group_id = ? THEN ? ELSE group_id END,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            ",
            params![payload.group_id, next_group_id, payload.tag_id],
        )?;
        self.get_record_tag(payload.tag_id)
    }

    pub fn attach_record_tag(&self, payload: AttachRecordTagPayload) -> Result<RecordTag, DbError> {
        self.ensure_record_tag(payload.tag_id)?;
        let table = self.get_table_summary(payload.table_id)?;
        self.ensure_table_record_exists(&table.table_name, payload.record_id)?;

        self.conn.execute(
            "
            INSERT OR IGNORE INTO record_tag_links (tag_id, table_id, record_id)
            VALUES (?, ?, ?)
            ",
            params![payload.tag_id, payload.table_id, payload.record_id],
        )?;

        self.get_record_tag(payload.tag_id)
    }

    pub fn create_and_attach_record_tag(
        &self,
        payload: CreateAndAttachRecordTagPayload,
    ) -> Result<RecordTag, DbError> {
        let name = require_trimmed("tag name", &payload.name)?;
        let tag_id = match self.find_record_tag_id_by_name(name)? {
            Some(tag_id) => tag_id,
            None => {
                let sort_order = self.next_record_tag_sort_order(None)?;
                self.conn.execute(
                    "INSERT INTO record_tags (name, group_id, sort_order) VALUES (?, NULL, ?)",
                    params![name, sort_order],
                )?;
                self.conn.last_insert_rowid()
            }
        };

        self.attach_record_tag(AttachRecordTagPayload {
            table_id: payload.table_id,
            record_id: payload.record_id,
            tag_id,
        })
    }

    pub fn detach_record_tag(&self, payload: DetachRecordTagPayload) -> Result<(), DbError> {
        let table = self.get_table_summary(payload.table_id)?;
        self.ensure_table_record_exists(&table.table_name, payload.record_id)?;
        self.ensure_record_tag(payload.tag_id)?;
        self.conn.execute(
            "
            DELETE FROM record_tag_links
            WHERE tag_id = ? AND table_id = ? AND record_id = ?
            ",
            params![payload.tag_id, payload.table_id, payload.record_id],
        )?;
        Ok(())
    }

    pub fn get_view_table_sections(&self) -> Result<Vec<ViewTableSection>, DbError> {
        let tables = self.list_tables()?;
        let mut sections = Vec::with_capacity(tables.len());

        for table in tables {
            let label_column_name = self.label_column_name(table.id)?;
            let records = self.list_view_table_records(&table.table_name, &label_column_name)?;

            sections.push(ViewTableSection {
                table_id: table.id,
                table_name: table.table_name,
                display_name: table.display_name,
                records,
            });
        }

        Ok(sections)
    }

    pub fn list_all_folder_layout_templates(&self) -> Result<Vec<ViewLayoutTemplate>, DbError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT id, name, scope_type, folder_id, created_at, updated_at
            FROM view_layout_templates
            WHERE scope_type = 'folder'
            ORDER BY folder_id IS NOT NULL, name, id
            ",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ViewLayoutTemplate {
                id: row.get(0)?,
                name: row.get(1)?,
                scope_type: row.get(2)?,
                folder_id: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub fn list_view_layout_templates_for_folder(
        &self,
        payload: ListViewLayoutTemplatesForFolderPayload,
    ) -> Result<FolderViewLayoutTemplates, DbError> {
        self.ensure_view_nav_folder(payload.folder_id)?;
        let active_template_id = self.assigned_folder_template_id(payload.folder_id)?;
        Ok(FolderViewLayoutTemplates {
            templates: self.list_view_layout_templates_for_folder_id(payload.folder_id)?,
            active_template_id,
        })
    }

    pub fn create_view_layout_template(
        &self,
        payload: CreateViewLayoutTemplatePayload,
    ) -> Result<ViewLayoutTemplate, DbError> {
        let name = require_trimmed("layout template name", &payload.name)?;
        if let Some(folder_id) = payload.folder_id {
            self.ensure_view_nav_folder(folder_id)?;
        }
        let scope_type = payload.scope_type.as_deref().unwrap_or("folder");
        if scope_type != "folder" {
            return Err(DbError::InvalidInput(
                "unsupported layout template scope type".into(),
            ));
        }
        self.conn.execute(
            "INSERT INTO view_layout_templates (name, scope_type, folder_id) VALUES (?, ?, ?)",
            params![name, scope_type, payload.folder_id],
        )?;
        let template_id = self.conn.last_insert_rowid();
        self.get_view_layout_template(template_id)
    }

    pub fn rename_view_layout_template(
        &self,
        payload: RenameViewLayoutTemplatePayload,
    ) -> Result<ViewLayoutTemplate, DbError> {
        let name = require_trimmed("layout template name", &payload.name)?;
        self.ensure_view_layout_template(payload.template_id)?;
        self.conn.execute(
            "UPDATE view_layout_templates SET name = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            params![name, payload.template_id],
        )?;
        self.get_view_layout_template(payload.template_id)
    }

    pub fn duplicate_view_layout_template(
        &self,
        payload: DuplicateViewLayoutTemplatePayload,
    ) -> Result<ViewLayoutTemplate, DbError> {
        let name = require_trimmed("layout template name", &payload.name)?;
        let source = self.get_view_layout_template(payload.template_id)?;
        self.conn.execute(
            "INSERT INTO view_layout_templates (name, scope_type, folder_id) VALUES (?, 'folder', NULL)",
            params![name],
        )?;
        let template_id = self.conn.last_insert_rowid();
        self.copy_view_layout_cards(source.id, template_id)?;
        self.get_view_layout_template(template_id)
    }

    pub fn delete_view_layout_template(
        &self,
        payload: DeleteViewLayoutTemplatePayload,
    ) -> Result<(), DbError> {
        self.ensure_view_layout_template(payload.template_id)?;
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM view_layout_card_overrides WHERE template_id = ?",
            [payload.template_id],
        )?;
        tx.execute(
            "DELETE FROM view_layout_card_column_bindings WHERE template_id = ?",
            [payload.template_id],
        )?;
        tx.execute(
            "DELETE FROM view_layout_template_cards WHERE template_id = ?",
            [payload.template_id],
        )?;
        tx.execute(
            "DELETE FROM view_layout_folder_template_assignments WHERE template_id = ?",
            [payload.template_id],
        )?;
        tx.execute(
            "DELETE FROM view_layout_record_template_assignments WHERE template_id = ?",
            [payload.template_id],
        )?;
        tx.execute(
            "DELETE FROM view_layout_templates WHERE id = ?",
            [payload.template_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn assign_view_layout_folder_template(
        &self,
        payload: AssignViewLayoutFolderTemplatePayload,
    ) -> Result<ViewLayoutTemplate, DbError> {
        self.ensure_view_nav_folder(payload.folder_id)?;
        let template = self.get_view_layout_template(payload.template_id)?;
        if template.scope_type != "folder"
            || (template.folder_id.is_some() && template.folder_id != Some(payload.folder_id))
        {
            return Err(DbError::InvalidInput(
                "layout template folder mismatch".into(),
            ));
        }
        self.conn.execute(
            "
            INSERT INTO view_layout_folder_template_assignments
              (folder_id, template_id, updated_at)
            VALUES (?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(folder_id) DO UPDATE SET
              template_id = excluded.template_id,
              updated_at = CURRENT_TIMESTAMP
            ",
            params![payload.folder_id, payload.template_id],
        )?;
        Ok(template)
    }

    pub fn assign_view_layout_record_template(
        &self,
        payload: AssignViewLayoutRecordTemplatePayload,
    ) -> Result<ViewNavFolderRecord, DbError> {
        let folder_record = self.get_view_nav_folder_record(payload.folder_record_id)?;
        let template = self.get_view_layout_template(payload.template_id)?;
        if template.scope_type != "folder"
            || (template.folder_id.is_some() && template.folder_id != Some(folder_record.folder_id))
        {
            return Err(DbError::InvalidInput(
                "layout template folder mismatch".into(),
            ));
        }

        if self.assigned_folder_template_id(folder_record.folder_id)? == Some(payload.template_id) {
            self.conn.execute(
                "DELETE FROM view_layout_record_template_assignments WHERE folder_record_id = ?",
                [payload.folder_record_id],
            )?;
            return self.get_view_nav_folder_record(payload.folder_record_id);
        }

        self.conn.execute(
            "
            INSERT INTO view_layout_record_template_assignments
              (folder_record_id, template_id, updated_at)
            VALUES (?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(folder_record_id) DO UPDATE SET
              template_id = excluded.template_id,
              updated_at = CURRENT_TIMESTAMP
            ",
            params![payload.folder_record_id, payload.template_id],
        )?;

        self.get_view_nav_folder_record(payload.folder_record_id)
    }

    pub fn clear_view_layout_record_template(
        &self,
        payload: ClearViewLayoutRecordTemplatePayload,
    ) -> Result<ViewNavFolderRecord, DbError> {
        self.get_view_nav_folder_record(payload.folder_record_id)?;
        self.conn.execute(
            "DELETE FROM view_layout_record_template_assignments WHERE folder_record_id = ?",
            [payload.folder_record_id],
        )?;
        self.get_view_nav_folder_record(payload.folder_record_id)
    }

    pub fn get_resolved_view_field_layout(
        &self,
        payload: GetResolvedViewFieldLayoutPayload,
    ) -> Result<ResolvedViewFieldLayout, DbError> {
        let table = self.get_table_summary(payload.table_id)?;
        self.ensure_table_record_exists(&table.table_name, payload.record_id)?;
        let active_template_id = self
            .resolve_record_view_layout_template_id(payload.folder_record_id, payload.folder_id)?;
        let active_template = active_template_id
            .map(|template_id| self.get_view_layout_template(template_id))
            .transpose()?;
        let items = match active_template_id {
            Some(template_id) => {
                self.resolve_view_layout_items(template_id, payload.table_id, payload.record_id)?
            }
            None => Vec::new(),
        };

        Ok(ResolvedViewFieldLayout {
            templates: active_template.iter().cloned().collect(),
            active_template_id,
            active_template_name: active_template.map(|template| template.name),
            items,
        })
    }

    pub fn get_view_layout_template_cards(
        &self,
        payload: GetViewLayoutTemplateCardsPayload,
    ) -> Result<Vec<ViewLayoutTemplateCard>, DbError> {
        self.ensure_view_layout_template(payload.template_id)?;
        self.list_view_layout_template_cards(payload.template_id)
    }

    pub fn list_view_layout_card_column_bindings(
        &self,
        payload: ListViewLayoutCardColumnBindingsPayload,
    ) -> Result<Vec<ViewLayoutCardColumnBinding>, DbError> {
        self.ensure_view_layout_template(payload.template_id)?;
        self.get_table_summary(payload.table_id)?;
        let mut stmt = self.conn.prepare(
            "
            SELECT binding.card_id, binding.column_id
            FROM view_layout_card_column_bindings binding
            JOIN view_layout_template_cards card
              ON card.template_id = binding.template_id
             AND card.card_id = binding.card_id
            WHERE binding.template_id = ? AND binding.table_id = ?
            ORDER BY card.sort_order, binding.card_id
            ",
        )?;
        let rows = stmt.query_map(params![payload.template_id, payload.table_id], |row| {
            Ok(ViewLayoutCardColumnBinding {
                card_id: row.get(0)?,
                column_id: row.get(1)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub fn save_view_layout_template_cards(
        &self,
        payload: SaveViewLayoutTemplateCardsPayload,
    ) -> Result<(), DbError> {
        self.ensure_view_layout_template(payload.template_id)?;
        let existing_card_ids = self.list_view_layout_template_card_ids(payload.template_id)?;
        let kept_card_ids = payload
            .cards
            .iter()
            .filter_map(|card| (card.card_id > 0).then_some(card.card_id))
            .collect::<Vec<_>>();
        let tx = self.conn.unchecked_transaction()?;

        for card_id in &existing_card_ids {
            if kept_card_ids.contains(card_id) {
                continue;
            }
            tx.execute(
                "DELETE FROM view_layout_card_overrides WHERE template_id = ? AND card_id = ?",
                params![payload.template_id, *card_id],
            )?;
            tx.execute(
                "DELETE FROM view_layout_card_column_bindings WHERE template_id = ? AND card_id = ?",
                params![payload.template_id, *card_id],
            )?;
            tx.execute(
                "DELETE FROM view_layout_template_cards WHERE template_id = ? AND card_id = ?",
                params![payload.template_id, *card_id],
            )?;
        }
        tx.commit()?;

        for (index, card) in payload.cards.into_iter().enumerate() {
            let explicit_card_id = existing_card_ids
                .contains(&card.card_id)
                .then_some(card.card_id);
            self.insert_view_layout_template_card(
                payload.template_id,
                SaveViewLayoutCardItem {
                    card_id: card.card_id,
                    x: card.x,
                    y: card.y,
                    width: card.width,
                    height: card.height,
                    visible: card.visible,
                    background_color: card.background_color,
                    text_color: card.text_color,
                    font_size: card.font_size,
                    text_direction: card.text_direction,
                    font_weight: card.font_weight,
                    text_align: card.text_align,
                    padding: card.padding,
                    padding_top: card.padding_top,
                    padding_right: card.padding_right,
                    padding_bottom: card.padding_bottom,
                    padding_left: card.padding_left,
                    border_radius: card.border_radius,
                    show_label: card.show_label,
                },
                index as i64,
                explicit_card_id,
            )?;
        }

        Ok(())
    }

    pub fn save_view_layout_card_column_bindings(
        &self,
        payload: SaveViewLayoutCardColumnBindingsPayload,
    ) -> Result<(), DbError> {
        self.ensure_view_layout_template(payload.template_id)?;
        self.get_table_summary(payload.table_id)?;
        let tx = self.conn.unchecked_transaction()?;

        tx.execute(
            "
            DELETE FROM view_layout_card_column_bindings
            WHERE template_id = ? AND table_id = ?
            ",
            params![payload.template_id, payload.table_id],
        )?;

        for binding in payload.bindings {
            let card_exists: Option<i64> = tx
                .query_row(
                    "
                    SELECT card_id
                    FROM view_layout_template_cards
                    WHERE template_id = ? AND card_id = ?
                    ",
                    params![payload.template_id, binding.card_id],
                    |row| row.get(0),
                )
                .optional()?;

            if card_exists.is_none() {
                return Err(DbError::InvalidInput("layout card does not exist".into()));
            }

            let column_exists: Option<i64> = tx
                .query_row(
                    "
                    SELECT id
                    FROM app_table_columns
                    WHERE table_id = ? AND id = ? AND column_name <> 'id'
                    ",
                    params![payload.table_id, binding.column_id],
                    |row| row.get(0),
                )
                .optional()?;

            if column_exists.is_none() {
                return Err(DbError::InvalidInput("column does not exist".into()));
            }

            tx.execute(
                "
                INSERT INTO view_layout_card_column_bindings
                  (template_id, table_id, card_id, column_id, updated_at)
                VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)
                ",
                params![
                    payload.template_id,
                    payload.table_id,
                    binding.card_id,
                    binding.column_id
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn save_view_layout_card_overrides(
        &self,
        payload: SaveViewLayoutCardOverridesPayload,
    ) -> Result<(), DbError> {
        let table = self.get_table_summary(payload.table_id)?;
        self.ensure_table_record_exists(&table.table_name, payload.record_id)?;
        let template = self.get_view_layout_template(payload.template_id)?;
        if template.scope_type != "folder" {
            return Err(DbError::InvalidInput(
                "layout template scope mismatch".into(),
            ));
        }
        self.conn.execute(
            "DELETE FROM view_layout_card_overrides WHERE template_id = ? AND table_id = ? AND record_id = ?",
            params![payload.template_id, payload.table_id, payload.record_id],
        )?;
        for item in payload.items {
            let Some(template_item) =
                self.get_view_layout_template_card(payload.template_id, item.card_id)?
            else {
                continue;
            };
            if item.card_id <= 0 {
                continue;
            }
            let offset_x = item.x.max(0.0) - template_item.x;
            let offset_y = item.y.max(0.0) - template_item.y;
            let offset_width = item.width.max(80.0) - template_item.width;
            let offset_height = item.height.max(56.0) - template_item.height;
            let visible =
                (item.visible != template_item.visible).then(|| bool_to_i64(item.visible));
            let background_color =
                override_text(item.background_color, template_item.background_color);
            let text_color = override_text(item.text_color, template_item.text_color);
            let font_size = override_number(item.font_size, template_item.font_size);
            let text_direction = override_text(item.text_direction, template_item.text_direction);
            let font_weight = override_text(item.font_weight, template_item.font_weight);
            let text_align = override_text(item.text_align, template_item.text_align);
            let padding = override_number(item.padding, template_item.padding);
            let padding_top = override_number(item.padding_top, template_item.padding_top);
            let padding_right = override_number(item.padding_right, template_item.padding_right);
            let padding_bottom = override_number(item.padding_bottom, template_item.padding_bottom);
            let padding_left = override_number(item.padding_left, template_item.padding_left);
            let border_radius = override_number(item.border_radius, template_item.border_radius);
            let show_label = if item.show_label != template_item.show_label {
                item.show_label.map(bool_to_i64)
            } else {
                None
            };
            let has_override = offset_x.abs() > 0.001
                || offset_y.abs() > 0.001
                || offset_width.abs() > 0.001
                || offset_height.abs() > 0.001
                || visible.is_some()
                || background_color.is_some()
                || text_color.is_some()
                || font_size.is_some()
                || text_direction.is_some()
                || font_weight.is_some()
                || text_align.is_some()
                || padding.is_some()
                || padding_top.is_some()
                || padding_right.is_some()
                || padding_bottom.is_some()
                || padding_left.is_some()
                || border_radius.is_some()
                || show_label.is_some();
            if !has_override {
                continue;
            }
            self.conn.execute(
                "
                INSERT INTO view_layout_card_overrides (
                  template_id, table_id, record_id, card_id,
                  offset_x, offset_y, offset_width, offset_height, visible,
                  background_color, text_color, font_size, text_direction,
                  font_weight, text_align, padding, padding_top, padding_right,
                  padding_bottom, padding_left, border_radius, show_label,
                  updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
                ",
                params![
                    payload.template_id,
                    payload.table_id,
                    payload.record_id,
                    item.card_id,
                    offset_x,
                    offset_y,
                    offset_width,
                    offset_height,
                    visible,
                    background_color,
                    text_color,
                    font_size,
                    text_direction,
                    font_weight,
                    text_align,
                    padding,
                    padding_top,
                    padding_right,
                    padding_bottom,
                    padding_left,
                    border_radius,
                    show_label
                ],
            )?;
        }
        Ok(())
    }

    pub fn reset_view_layout_card_override(
        &self,
        payload: ResetViewLayoutCardOverridePayload,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM view_layout_card_overrides WHERE template_id = ? AND table_id = ? AND record_id = ? AND card_id = ?",
            params![payload.template_id, payload.table_id, payload.record_id, payload.card_id],
        )?;
        Ok(())
    }

    pub fn reset_view_layout_card_overrides(
        &self,
        payload: ResetViewLayoutCardOverridesPayload,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM view_layout_card_overrides WHERE template_id = ? AND table_id = ? AND record_id = ?",
            params![payload.template_id, payload.table_id, payload.record_id],
        )?;
        Ok(())
    }

    fn list_tables(&self) -> Result<Vec<AppTableSummary>, DbError> {
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

    fn get_table_summary(&self, table_id: i64) -> Result<AppTableSummary, DbError> {
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

    fn list_columns(&self, table_id: i64) -> Result<Vec<AppColumn>, DbError> {
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

    fn list_option_groups(&self) -> Result<Vec<SelectOptionGroup>, DbError> {
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

    fn list_options(&self, group_id: i64) -> Result<Vec<SelectOption>, DbError> {
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

    fn list_records(
        &self,
        table_name: &str,
        columns: &[AppColumn],
    ) -> Result<Vec<TableRecord>, DbError> {
        self.list_records_with_order(table_name, columns, "ASC")
    }

    fn list_records_with_order(
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

    fn display_value_for_column(
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

    fn csv_select_option_maps(
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

    fn table_name_by_id(&self, table_id: i64) -> Result<String, DbError> {
        self.conn
            .query_row(
                "SELECT table_name FROM app_tables WHERE id = ?",
                [table_id],
                |row| row.get(0),
            )
            .map_err(DbError::from)
    }

    fn table_reference_sources(&self, table_id: i64) -> Result<Vec<String>, DbError> {
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

    fn label_column_name(&self, table_id: i64) -> Result<String, DbError> {
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

    fn next_sort_order(&self, target: &str, table_id: Option<i64>) -> Result<i64, DbError> {
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

    fn next_view_nav_sort_order(&self, parent_id: Option<i64>) -> Result<i64, DbError> {
        match parent_id {
            Some(parent_id) => self
                .conn
                .query_row(
                    "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM view_nav_nodes WHERE parent_id = ?",
                    [parent_id],
                    |row| row.get(0),
                )
                .map_err(DbError::from),
            None => self
                .conn
                .query_row(
                    "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM view_nav_nodes WHERE parent_id IS NULL",
                    [],
                    |row| row.get(0),
                )
                .map_err(DbError::from),
        }
    }

    fn next_view_nav_folder_record_sort_order(&self, folder_id: i64) -> Result<i64, DbError> {
        self.conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM view_nav_folder_records WHERE folder_id = ?",
                [folder_id],
                |row| row.get(0),
            )
            .map_err(DbError::from)
    }

    fn view_nav_folder_record_exists(
        &self,
        folder_id: i64,
        table_id: i64,
        record_id: i64,
    ) -> Result<bool, DbError> {
        self.conn
            .query_row(
                "
                SELECT 1
                FROM view_nav_folder_records
                WHERE folder_id = ? AND table_id = ? AND record_id = ?
                ",
                params![folder_id, table_id, record_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map(|value| value.is_some())
            .map_err(DbError::from)
    }

    fn ensure_view_nav_folder(&self, folder_id: i64) -> Result<(), DbError> {
        let folder_exists: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM view_nav_nodes WHERE id = ? AND node_type = 'folder'",
                [folder_id],
                |row| row.get(0),
            )
            .optional()?;

        if folder_exists.is_none() {
            return Err(DbError::InvalidInput("folder does not exist".into()));
        }

        Ok(())
    }

    fn ensure_table_record_exists(&self, table_name: &str, record_id: i64) -> Result<(), DbError> {
        let record_exists: Option<i64> = self
            .conn
            .query_row(
                &format!("SELECT id FROM \"{}\" WHERE id = ?", table_name),
                [record_id],
                |row| row.get(0),
            )
            .optional()?;

        if record_exists.is_none() {
            return Err(DbError::InvalidInput("record does not exist".into()));
        }

        Ok(())
    }

    fn list_view_layout_templates_for_folder_id(
        &self,
        folder_id: i64,
    ) -> Result<Vec<ViewLayoutTemplate>, DbError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT id, name, scope_type, folder_id, created_at, updated_at
            FROM view_layout_templates
            WHERE scope_type = 'folder' AND (folder_id = ? OR folder_id IS NULL)
            ORDER BY folder_id IS NOT NULL DESC, name, id
            ",
        )?;
        let rows = stmt.query_map([folder_id], |row| {
            Ok(ViewLayoutTemplate {
                id: row.get(0)?,
                name: row.get(1)?,
                scope_type: row.get(2)?,
                folder_id: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    fn get_view_layout_template(&self, template_id: i64) -> Result<ViewLayoutTemplate, DbError> {
        self.conn
            .query_row(
                "
                SELECT id, name, scope_type, folder_id, created_at, updated_at
                FROM view_layout_templates
                WHERE id = ?
                ",
                [template_id],
                |row| {
                    Ok(ViewLayoutTemplate {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        scope_type: row.get(2)?,
                        folder_id: row.get(3)?,
                        created_at: row.get(4)?,
                        updated_at: row.get(5)?,
                    })
                },
            )
            .map_err(DbError::from)
    }

    fn list_view_layout_template_cards(
        &self,
        template_id: i64,
    ) -> Result<Vec<ViewLayoutTemplateCard>, DbError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT
              card.card_id,
              card.x,
              card.y,
              card.width,
              card.height,
              card.visible,
              NULL AS label,
              card.background_color,
              card.text_color,
              card.font_size,
              card.text_direction,
              card.font_weight,
              card.text_align,
              card.padding,
              card.padding_top,
              card.padding_right,
              card.padding_bottom,
              card.padding_left,
              card.border_radius,
              card.show_label
            FROM view_layout_template_cards card
            WHERE card.template_id = ?
            ORDER BY card.sort_order, card.card_id
            ",
        )?;
        let rows = stmt.query_map([template_id], |row| {
            Ok(ViewLayoutTemplateCard {
                card_id: row.get(0)?,
                x: row.get(1)?,
                y: row.get(2)?,
                width: row.get(3)?,
                height: row.get(4)?,
                visible: row.get::<_, i64>(5)? != 0,
                label: row.get(6)?,
                background_color: row.get(7)?,
                text_color: row.get(8)?,
                font_size: row.get(9)?,
                text_direction: row.get(10)?,
                font_weight: row.get(11)?,
                text_align: row.get(12)?,
                padding: row.get(13)?,
                padding_top: row.get(14)?,
                padding_right: row.get(15)?,
                padding_bottom: row.get(16)?,
                padding_left: row.get(17)?,
                border_radius: row.get(18)?,
                show_label: row.get::<_, Option<i64>>(19)?.map(|value| value != 0),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    fn list_view_layout_template_card_ids(&self, template_id: i64) -> Result<Vec<i64>, DbError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT card_id
            FROM view_layout_template_cards
            WHERE template_id = ?
            ORDER BY sort_order, card_id
            ",
        )?;
        let rows = stmt.query_map([template_id], |row| row.get::<_, i64>(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    fn ensure_view_layout_template(&self, template_id: i64) -> Result<(), DbError> {
        self.get_view_layout_template(template_id).map(|_| ())
    }

    fn assigned_folder_template_id(&self, folder_id: i64) -> Result<Option<i64>, DbError> {
        self.conn
            .query_row(
                "
                SELECT template_id FROM view_layout_folder_template_assignments
                WHERE folder_id = ?
                ",
                [folder_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(DbError::from)
    }

    fn assigned_record_template_id(&self, folder_record_id: i64) -> Result<Option<i64>, DbError> {
        self.conn
            .query_row(
                "
                SELECT template_id FROM view_layout_record_template_assignments
                WHERE folder_record_id = ?
                ",
                [folder_record_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(DbError::from)
    }

    fn resolve_record_view_layout_template_id(
        &self,
        folder_record_id: Option<i64>,
        folder_id: Option<i64>,
    ) -> Result<Option<i64>, DbError> {
        if let Some(folder_record_id) = folder_record_id {
            let folder_record = self.get_view_nav_folder_record(folder_record_id)?;
            if let Some(payload_folder_id) = folder_id {
                if payload_folder_id != folder_record.folder_id {
                    return Err(DbError::InvalidInput(
                        "folder record does not belong to the target folder".into(),
                    ));
                }
            }

            if let Some(template_id) = self.assigned_record_template_id(folder_record_id)? {
                return Ok(Some(template_id));
            }

            return self.assigned_folder_template_id(folder_record.folder_id);
        }

        match folder_id {
            Some(folder_id) => {
                self.ensure_view_nav_folder(folder_id)?;
                self.assigned_folder_template_id(folder_id)
            }
            None => Err(DbError::InvalidInput(
                "folder layout template requires a folder".into(),
            )),
        }
    }

    fn copy_view_layout_cards(
        &self,
        source_template_id: i64,
        target_template_id: i64,
    ) -> Result<(), DbError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT card_id, x, y, width, height, visible,
                   background_color, text_color, font_size, text_direction,
                   font_weight, text_align, padding, padding_top, padding_right,
                   padding_bottom, padding_left, border_radius, show_label, sort_order
            FROM view_layout_template_cards
            WHERE template_id = ?
            ORDER BY sort_order, card_id
            ",
        )?;
        let cards = stmt
            .query_map([source_template_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    SaveViewLayoutCardItem {
                        card_id: 0,
                        x: row.get(1)?,
                        y: row.get(2)?,
                        width: row.get(3)?,
                        height: row.get(4)?,
                        visible: row.get::<_, i64>(5)? != 0,
                        background_color: row.get(6)?,
                        text_color: row.get(7)?,
                        font_size: row.get(8)?,
                        text_direction: row.get(9)?,
                        font_weight: row.get(10)?,
                        text_align: row.get(11)?,
                        padding: row.get(12)?,
                        padding_top: row.get(13)?,
                        padding_right: row.get(14)?,
                        padding_bottom: row.get(15)?,
                        padding_left: row.get(16)?,
                        border_radius: row.get(17)?,
                        show_label: row.get::<_, Option<i64>>(18)?.map(|value| value != 0),
                    },
                    row.get::<_, i64>(19)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(stmt);

        for (source_card_id, item, sort_order) in cards {
            let card_id =
                self.insert_view_layout_template_card(target_template_id, item, sort_order, None)?;
            self.conn.execute(
                "
                INSERT OR IGNORE INTO view_layout_card_column_bindings
                  (template_id, table_id, card_id, column_id, updated_at)
                SELECT ?, table_id, ?, column_id, CURRENT_TIMESTAMP
                FROM view_layout_card_column_bindings
                WHERE template_id = ? AND card_id = ?
                ",
                params![
                    target_template_id,
                    card_id,
                    source_template_id,
                    source_card_id
                ],
            )?;
        }
        Ok(())
    }

    fn insert_view_layout_template_card(
        &self,
        template_id: i64,
        item: SaveViewLayoutCardItem,
        sort_order: i64,
        explicit_card_id: Option<i64>,
    ) -> Result<i64, DbError> {
        if let Some(card_id) = explicit_card_id {
            self.conn.execute(
                "
                INSERT INTO view_layout_template_cards (
                  card_id, template_id, x, y, width, height, visible,
                  background_color, text_color, font_size, text_direction,
                  font_weight, text_align, padding, padding_top, padding_right,
                  padding_bottom, padding_left, border_radius, show_label,
                  sort_order, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
                ON CONFLICT(card_id) DO UPDATE SET
                  x = excluded.x,
                  y = excluded.y,
                  width = excluded.width,
                  height = excluded.height,
                  visible = excluded.visible,
                  background_color = excluded.background_color,
                  text_color = excluded.text_color,
                  font_size = excluded.font_size,
                  text_direction = excluded.text_direction,
                  font_weight = excluded.font_weight,
                  text_align = excluded.text_align,
                  padding = excluded.padding,
                  padding_top = excluded.padding_top,
                  padding_right = excluded.padding_right,
                  padding_bottom = excluded.padding_bottom,
                  padding_left = excluded.padding_left,
                  border_radius = excluded.border_radius,
                  show_label = excluded.show_label,
                  sort_order = excluded.sort_order,
                  updated_at = CURRENT_TIMESTAMP
                ",
                params![
                    card_id,
                    template_id,
                    item.x.max(0.0),
                    item.y.max(0.0),
                    item.width.max(80.0),
                    item.height.max(56.0),
                    bool_to_i64(item.visible),
                    item.background_color,
                    item.text_color,
                    item.font_size,
                    item.text_direction,
                    item.font_weight,
                    item.text_align,
                    item.padding,
                    item.padding_top,
                    item.padding_right,
                    item.padding_bottom,
                    item.padding_left,
                    item.border_radius,
                    item.show_label.map(bool_to_i64),
                    sort_order
                ],
            )?;
            return Ok(card_id);
        }

        self.conn.execute(
            "
            INSERT INTO view_layout_template_cards (
              template_id, x, y, width, height, visible,
              background_color, text_color, font_size, text_direction,
              font_weight, text_align, padding, padding_top, padding_right,
              padding_bottom, padding_left, border_radius, show_label,
              sort_order, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            ",
            params![
                template_id,
                item.x.max(0.0),
                item.y.max(0.0),
                item.width.max(80.0),
                item.height.max(56.0),
                bool_to_i64(item.visible),
                item.background_color,
                item.text_color,
                item.font_size,
                item.text_direction,
                item.font_weight,
                item.text_align,
                item.padding,
                item.padding_top,
                item.padding_right,
                item.padding_bottom,
                item.padding_left,
                item.border_radius,
                item.show_label.map(bool_to_i64),
                sort_order
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    fn get_view_layout_template_card(
        &self,
        template_id: i64,
        card_id: i64,
    ) -> Result<Option<SaveViewLayoutCardItem>, DbError> {
        self.conn
            .query_row(
                "
                SELECT card_id, x, y, width, height, visible,
                       background_color, text_color, font_size, text_direction,
                       font_weight, text_align, padding, padding_top, padding_right,
                       padding_bottom, padding_left, border_radius, show_label
                FROM view_layout_template_cards
                WHERE template_id = ? AND card_id = ?
                ",
                params![template_id, card_id],
                |row| {
                    Ok(SaveViewLayoutCardItem {
                        card_id: row.get(0)?,
                        x: row.get(1)?,
                        y: row.get(2)?,
                        width: row.get(3)?,
                        height: row.get(4)?,
                        visible: row.get::<_, i64>(5)? != 0,
                        background_color: row.get(6)?,
                        text_color: row.get(7)?,
                        font_size: row.get(8)?,
                        text_direction: row.get(9)?,
                        font_weight: row.get(10)?,
                        text_align: row.get(11)?,
                        padding: row.get(12)?,
                        padding_top: row.get(13)?,
                        padding_right: row.get(14)?,
                        padding_bottom: row.get(15)?,
                        padding_left: row.get(16)?,
                        border_radius: row.get(17)?,
                        show_label: row.get::<_, Option<i64>>(18)?.map(|value| value != 0),
                    })
                },
            )
            .optional()
            .map_err(DbError::from)
    }

    fn resolve_view_layout_items(
        &self,
        template_id: i64,
        table_id: i64,
        record_id: i64,
    ) -> Result<Vec<ViewLayoutCardItem>, DbError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT
              ?,
              card.card_id,
              binding.column_id,
              card.x + COALESCE(override.offset_x, 0),
              card.y + COALESCE(override.offset_y, 0),
              card.width + COALESCE(override.offset_width, 0),
              card.height + COALESCE(override.offset_height, 0),
              COALESCE(override.visible, card.visible),
              COALESCE(override.background_color, card.background_color),
              COALESCE(override.text_color, card.text_color),
              COALESCE(override.font_size, card.font_size),
              COALESCE(override.text_direction, card.text_direction),
              COALESCE(override.font_weight, card.font_weight),
              COALESCE(override.text_align, card.text_align),
              COALESCE(override.padding, card.padding),
              COALESCE(override.padding_top, card.padding_top),
              COALESCE(override.padding_right, card.padding_right),
              COALESCE(override.padding_bottom, card.padding_bottom),
              COALESCE(override.padding_left, card.padding_left),
              COALESCE(override.border_radius, card.border_radius),
              COALESCE(override.show_label, card.show_label),
              override.card_id IS NOT NULL
            FROM view_layout_template_cards card
            LEFT JOIN view_layout_card_column_bindings binding
              ON binding.template_id = card.template_id
             AND binding.table_id = ?
             AND binding.card_id = card.card_id
            LEFT JOIN view_layout_card_overrides override
              ON override.template_id = card.template_id
             AND override.table_id = ?
             AND override.record_id = ?
             AND override.card_id = card.card_id
            WHERE card.template_id = ?
            ORDER BY card.sort_order, card.card_id
            ",
        )?;
        let rows = stmt.query_map(
            params![table_id, table_id, table_id, record_id, template_id],
            |row| {
                Ok(ViewLayoutCardItem {
                    table_id: row.get(0)?,
                    card_id: row.get(1)?,
                    column_id: row.get(2)?,
                    x: row.get(3)?,
                    y: row.get(4)?,
                    width: row.get(5)?,
                    height: row.get(6)?,
                    visible: row.get::<_, i64>(7)? != 0,
                    background_color: row.get(8)?,
                    text_color: row.get(9)?,
                    font_size: row.get(10)?,
                    text_direction: row.get(11)?,
                    font_weight: row.get(12)?,
                    text_align: row.get(13)?,
                    padding: row.get(14)?,
                    padding_top: row.get(15)?,
                    padding_right: row.get(16)?,
                    padding_bottom: row.get(17)?,
                    padding_left: row.get(18)?,
                    border_radius: row.get(19)?,
                    show_label: row.get::<_, Option<i64>>(20)?.map(|value| value != 0),
                    has_override: row.get::<_, bool>(21)?,
                })
            },
        )?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    fn get_view_nav_folder_record(
        &self,
        folder_record_id: i64,
    ) -> Result<ViewNavFolderRecord, DbError> {
        self.conn
            .query_row(
                "
                SELECT
                  r.id,
                  r.folder_id,
                  r.table_id,
                  t.table_name,
                  t.display_name,
                  r.record_id,
                  r.record_label,
                  assignment.template_id,
                  r.sort_order,
                  r.created_at,
                  r.updated_at
                FROM view_nav_folder_records r
                JOIN app_tables t ON t.id = r.table_id
                LEFT JOIN view_layout_record_template_assignments assignment
                  ON assignment.folder_record_id = r.id
                WHERE r.id = ?
                ",
                [folder_record_id],
                |row| {
                    Ok(ViewNavFolderRecord {
                        id: row.get(0)?,
                        folder_id: row.get(1)?,
                        table_id: row.get(2)?,
                        table_name: row.get(3)?,
                        table_display_name: row.get(4)?,
                        record_id: row.get(5)?,
                        record_label: row.get(6)?,
                        record_template_id: row.get(7)?,
                        sort_order: row.get(8)?,
                        created_at: row.get(9)?,
                        updated_at: row.get(10)?,
                    })
                },
            )
            .map_err(DbError::from)
    }

    fn list_view_nav_folder_record_ids(&self, folder_id: i64) -> Result<Vec<i64>, DbError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT id
            FROM view_nav_folder_records
            WHERE folder_id = ?
            ORDER BY sort_order, id
            ",
        )?;

        let ids = stmt
            .query_map([folder_id], |row| row.get::<_, i64>(0))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(DbError::from)?;

        Ok(ids)
    }

    fn list_view_nav_folder_records_for_folder(
        &self,
        folder_id: i64,
    ) -> Result<Vec<ViewNavFolderRecord>, DbError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT
              r.id,
              r.folder_id,
              r.table_id,
              t.table_name,
              t.display_name,
              r.record_id,
              r.record_label,
              assignment.template_id,
              r.sort_order,
              r.created_at,
              r.updated_at
            FROM view_nav_folder_records r
            JOIN app_tables t ON t.id = r.table_id
            LEFT JOIN view_layout_record_template_assignments assignment
              ON assignment.folder_record_id = r.id
            WHERE r.folder_id = ?
            ORDER BY r.sort_order, r.id
            ",
        )?;

        let rows = stmt.query_map([folder_id], |row| {
            Ok(ViewNavFolderRecord {
                id: row.get(0)?,
                folder_id: row.get(1)?,
                table_id: row.get(2)?,
                table_name: row.get(3)?,
                table_display_name: row.get(4)?,
                record_id: row.get(5)?,
                record_label: row.get(6)?,
                record_template_id: row.get(7)?,
                sort_order: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    fn list_record_tag_groups(&self) -> Result<Vec<RecordTagGroup>, DbError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT
              tag_group.id,
              tag_group.name,
              tag_group.sort_order,
              COUNT(DISTINCT link.id) AS usage_count,
              COUNT(DISTINCT group_link.tag_id) AS tag_count,
              tag_group.created_at,
              tag_group.updated_at
            FROM record_tag_groups tag_group
            LEFT JOIN record_tag_group_links group_link ON group_link.group_id = tag_group.id
            LEFT JOIN record_tag_links link ON link.tag_id = group_link.tag_id
            GROUP BY tag_group.id
            ORDER BY tag_group.sort_order, tag_group.name, tag_group.id
            ",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(RecordTagGroup {
                id: row.get(0)?,
                name: row.get(1)?,
                sort_order: row.get(2)?,
                usage_count: row.get(3)?,
                tag_count: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    fn list_record_tag_items(&self) -> Result<Vec<RecordTag>, DbError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT
              tag.id,
              tag.group_id,
              GROUP_CONCAT(DISTINCT group_link.group_id) AS group_ids,
              tag.name,
              tag.sort_order,
              COUNT(DISTINCT link.id) AS usage_count,
              tag.created_at,
              tag.updated_at
            FROM record_tags tag
            LEFT JOIN record_tag_links link ON link.tag_id = tag.id
            LEFT JOIN record_tag_group_links group_link ON group_link.tag_id = tag.id
            GROUP BY tag.id
            ORDER BY tag.group_id IS NOT NULL, tag.group_id, tag.sort_order, tag.name, tag.id
            ",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(RecordTag {
                id: row.get(0)?,
                group_id: row.get(1)?,
                group_ids: parse_group_ids(row.get(2)?),
                name: row.get(3)?,
                sort_order: row.get(4)?,
                usage_count: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    fn get_record_tag_group(&self, group_id: i64) -> Result<RecordTagGroup, DbError> {
        self.conn
            .query_row(
                "
                SELECT
                  tag_group.id,
                  tag_group.name,
                  tag_group.sort_order,
                  COUNT(DISTINCT link.id) AS usage_count,
                  COUNT(DISTINCT group_link.tag_id) AS tag_count,
                  tag_group.created_at,
                  tag_group.updated_at
                FROM record_tag_groups tag_group
                LEFT JOIN record_tag_group_links group_link ON group_link.group_id = tag_group.id
                LEFT JOIN record_tag_links link ON link.tag_id = group_link.tag_id
                WHERE tag_group.id = ?
                GROUP BY tag_group.id
                ",
                [group_id],
                |row| {
                    Ok(RecordTagGroup {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        sort_order: row.get(2)?,
                        usage_count: row.get(3)?,
                        tag_count: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                },
            )
            .map_err(DbError::from)
    }

    fn get_record_tag(&self, tag_id: i64) -> Result<RecordTag, DbError> {
        self.conn
            .query_row(
                "
                SELECT
                  tag.id,
                  tag.group_id,
                  GROUP_CONCAT(DISTINCT group_link.group_id) AS group_ids,
                  tag.name,
                  tag.sort_order,
                  COUNT(DISTINCT link.id) AS usage_count,
                  tag.created_at,
                  tag.updated_at
                FROM record_tags tag
                LEFT JOIN record_tag_links link ON link.tag_id = tag.id
                LEFT JOIN record_tag_group_links group_link ON group_link.tag_id = tag.id
                WHERE tag.id = ?
                GROUP BY tag.id
                ",
                [tag_id],
                |row| {
                    Ok(RecordTag {
                        id: row.get(0)?,
                        group_id: row.get(1)?,
                        group_ids: parse_group_ids(row.get(2)?),
                        name: row.get(3)?,
                        sort_order: row.get(4)?,
                        usage_count: row.get(5)?,
                        created_at: row.get(6)?,
                        updated_at: row.get(7)?,
                    })
                },
            )
            .map_err(DbError::from)
    }

    fn find_record_tag_id_by_name(&self, name: &str) -> Result<Option<i64>, DbError> {
        self.conn
            .query_row("SELECT id FROM record_tags WHERE name = ?", [name], |row| {
                row.get(0)
            })
            .optional()
            .map_err(DbError::from)
    }

    fn ensure_record_tag_group(&self, group_id: i64) -> Result<(), DbError> {
        let exists: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM record_tag_groups WHERE id = ?",
                [group_id],
                |row| row.get(0),
            )
            .optional()?;

        if exists.is_none() {
            return Err(DbError::InvalidInput("tag group does not exist".into()));
        }

        Ok(())
    }

    fn ensure_record_tag(&self, tag_id: i64) -> Result<(), DbError> {
        let exists: Option<i64> = self
            .conn
            .query_row("SELECT id FROM record_tags WHERE id = ?", [tag_id], |row| {
                row.get(0)
            })
            .optional()?;

        if exists.is_none() {
            return Err(DbError::InvalidInput("tag does not exist".into()));
        }

        Ok(())
    }

    fn next_record_tag_group_sort_order(&self) -> Result<i64, DbError> {
        self.conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM record_tag_groups",
                [],
                |row| row.get(0),
            )
            .map_err(DbError::from)
    }

    fn next_record_tag_sort_order(&self, group_id: Option<i64>) -> Result<i64, DbError> {
        match group_id {
            Some(group_id) => self
                .conn
                .query_row(
                    "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM record_tags WHERE group_id = ?",
                    [group_id],
                    |row| row.get(0),
                )
                .map_err(DbError::from),
            None => self
                .conn
                .query_row(
                    "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM record_tags WHERE group_id IS NULL",
                    [],
                    |row| row.get(0),
                )
                .map_err(DbError::from),
        }
    }

    fn resequence_column_sort_order(&self, table_id: i64) -> Result<(), DbError> {
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

    fn first_non_id_column_name(&self, table_id: i64) -> Result<Option<String>, DbError> {
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

    fn list_view_table_records(
        &self,
        table_name: &str,
        label_column_name: &str,
    ) -> Result<Vec<ViewTableRecordSummary>, DbError> {
        let mut stmt = self.conn.prepare(&format!(
            "
            SELECT id, COALESCE(CAST(\"{}\" AS TEXT), '')
            FROM \"{}\"
            ORDER BY id ASC
            ",
            label_column_name, table_name
        ))?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let label: String = row.get(1)?;

            Ok(ViewTableRecordSummary {
                id,
                label: format!("{}:{}", id, label),
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }
}

fn sqlite_value_to_json(value: ValueRef<'_>) -> Result<Value, rusqlite::Error> {
    Ok(match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(v) => Value::from(v),
        ValueRef::Real(v) => Value::from(v),
        ValueRef::Text(v) => Value::from(String::from_utf8_lossy(v).to_string()),
        ValueRef::Blob(_) => Value::Null,
    })
}
