//! レコードタグ、タググループ、タグ紐付けを担当します。
//!
//! タグは複数グループに所属できるため、一覧取得では所属グループと利用件数を同時に返します。

use super::validation::*;
use super::*;
use rusqlite::{params, OptionalExtension};

impl Db {
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

        // タグの所属グループと利用件数を同時に返し、フロント側で追加問い合わせせず表示できるようにします。
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

    pub(super) fn list_record_tag_groups(&self) -> Result<Vec<RecordTagGroup>, DbError> {
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

    pub(super) fn list_record_tag_items(&self) -> Result<Vec<RecordTag>, DbError> {
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

    pub(super) fn get_record_tag_group(&self, group_id: i64) -> Result<RecordTagGroup, DbError> {
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

    pub(super) fn get_record_tag(&self, tag_id: i64) -> Result<RecordTag, DbError> {
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

    pub(super) fn find_record_tag_id_by_name(&self, name: &str) -> Result<Option<i64>, DbError> {
        self.conn
            .query_row("SELECT id FROM record_tags WHERE name = ?", [name], |row| {
                row.get(0)
            })
            .optional()
            .map_err(DbError::from)
    }

    pub(super) fn ensure_record_tag_group(&self, group_id: i64) -> Result<(), DbError> {
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

    pub(super) fn ensure_record_tag(&self, tag_id: i64) -> Result<(), DbError> {
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

    pub(super) fn next_record_tag_group_sort_order(&self) -> Result<i64, DbError> {
        self.conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM record_tag_groups",
                [],
                |row| row.get(0),
            )
            .map_err(DbError::from)
    }

    pub(super) fn next_record_tag_sort_order(&self, group_id: Option<i64>) -> Result<i64, DbError> {
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
}
