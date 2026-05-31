//! レイアウトカードの取得と保存を担当します。
//!
//! テンプレート上の基準配置だけを扱い、レコード個別overrideの反映はresolver側で行います。

use super::*;
use crate::database::validation::bool_to_i64;
use rusqlite::{params, OptionalExtension};

impl Db {
    pub(super) fn list_view_layout_template_cards(
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

    pub(super) fn list_view_layout_template_card_ids(
        &self,
        template_id: i64,
    ) -> Result<Vec<i64>, DbError> {
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

    pub(super) fn insert_view_layout_template_card(
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

    pub(super) fn get_view_layout_template_card(
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
}
