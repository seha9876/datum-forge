//! Resolved layout item builder for record view rendering.

use super::*;
use rusqlite::params;

impl Db {
    pub(super) fn resolve_view_layout_items(
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
}
