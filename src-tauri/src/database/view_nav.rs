//! 閲覧モードのフォルダツリーと、フォルダ内レコード配置を担当します。
//!
//! レコードの実データやレイアウト保存は別moduleに任せ、ここでは目次上の所属と順序だけを扱います。

use super::*;
use rusqlite::{params, OptionalExtension};
use std::collections::HashSet;

impl Db {
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

    pub(super) fn next_view_nav_sort_order(&self, parent_id: Option<i64>) -> Result<i64, DbError> {
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

    pub(super) fn next_view_nav_folder_record_sort_order(
        &self,
        folder_id: i64,
    ) -> Result<i64, DbError> {
        self.conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM view_nav_folder_records WHERE folder_id = ?",
                [folder_id],
                |row| row.get(0),
            )
            .map_err(DbError::from)
    }

    pub(super) fn view_nav_folder_record_exists(
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

    pub(super) fn ensure_view_nav_folder(&self, folder_id: i64) -> Result<(), DbError> {
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

    pub(super) fn ensure_table_record_exists(
        &self,
        table_name: &str,
        record_id: i64,
    ) -> Result<(), DbError> {
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

    pub(super) fn get_view_nav_folder_record(
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

    pub(super) fn list_view_nav_folder_record_ids(
        &self,
        folder_id: i64,
    ) -> Result<Vec<i64>, DbError> {
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

    pub(super) fn list_view_nav_folder_records_for_folder(
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

    pub(super) fn list_view_table_records(
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
