//! レイアウトテンプレートの検索、割り当て、複製を担当します。
//!
//! フォルダ共通テンプレートとフォルダ専用テンプレートの優先関係をここに閉じ込めます。

use super::*;
use rusqlite::{params, OptionalExtension};

impl Db {
    pub(super) fn list_view_layout_templates_for_folder_id(
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

    pub(super) fn get_view_layout_template(
        &self,
        template_id: i64,
    ) -> Result<ViewLayoutTemplate, DbError> {
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

    pub(super) fn ensure_view_layout_template(&self, template_id: i64) -> Result<(), DbError> {
        self.get_view_layout_template(template_id).map(|_| ())
    }

    pub(super) fn assigned_folder_template_id(
        &self,
        folder_id: i64,
    ) -> Result<Option<i64>, DbError> {
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

    pub(super) fn assigned_record_template_id(
        &self,
        folder_record_id: i64,
    ) -> Result<Option<i64>, DbError> {
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

    pub(super) fn resolve_record_view_layout_template_id(
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

    pub(super) fn copy_view_layout_cards(
        &self,
        source_template_id: i64,
        target_template_id: i64,
    ) -> Result<(), DbError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT card_id, preset_id, x, y, width, height, visible,
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
                        preset_id: row.get(1)?,
                        x: row.get(2)?,
                        y: row.get(3)?,
                        width: row.get(4)?,
                        height: row.get(5)?,
                        visible: row.get::<_, i64>(6)? != 0,
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
                    },
                    row.get::<_, i64>(20)?,
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
                  (template_id, table_id, card_id, sort_order, column_id, updated_at)
                SELECT ?, table_id, ?, sort_order, column_id, CURRENT_TIMESTAMP
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
}
