//! 閲覧レイアウトcommandの入口です。
//!
//! テンプレート、カード、紐付け、個別overrideの詳細は子moduleへ分け、公開commandの粒度は維持します。

mod cards;
mod resolver;
mod templates;

use super::validation::*;
use super::*;
use rusqlite::{params, OptionalExtension};

impl Db {
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
                    preset_id: card.preset_id,
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
            let _ = &item.preset_id;
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
}
