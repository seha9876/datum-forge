use super::*;

impl Db {
    pub(super) fn initialize(&self) -> Result<(), DbError> {
        self.conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS app_tables (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              table_name TEXT NOT NULL UNIQUE,
              display_name TEXT NOT NULL,
              label_column_id INTEGER,
              sort_order INTEGER NOT NULL DEFAULT 0,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS select_option_groups (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              name TEXT NOT NULL UNIQUE,
              description TEXT,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS select_options (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              group_id INTEGER NOT NULL,
              option_no INTEGER NOT NULL,
              sort_order INTEGER NOT NULL DEFAULT 0,
              label TEXT NOT NULL,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              UNIQUE(group_id, option_no),
              FOREIGN KEY(group_id) REFERENCES select_option_groups(id)
            );

            CREATE TABLE IF NOT EXISTS app_table_columns (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              table_id INTEGER NOT NULL,
              column_name TEXT NOT NULL,
              display_name TEXT NOT NULL,
              field_type TEXT NOT NULL,
              sort_order INTEGER NOT NULL DEFAULT 0,
              select_option_group_id INTEGER,
              ref_table_id INTEGER,
              is_required INTEGER NOT NULL DEFAULT 0,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              UNIQUE(table_id, column_name),
              FOREIGN KEY(table_id) REFERENCES app_tables(id),
              FOREIGN KEY(select_option_group_id) REFERENCES select_option_groups(id),
              FOREIGN KEY(ref_table_id) REFERENCES app_tables(id)
            );

            CREATE TABLE IF NOT EXISTS view_nav_nodes (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              node_type TEXT NOT NULL,
              parent_id INTEGER,
              name TEXT NOT NULL,
              sort_order INTEGER NOT NULL DEFAULT 0,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              FOREIGN KEY(parent_id) REFERENCES view_nav_nodes(id)
            );

            CREATE INDEX IF NOT EXISTS idx_view_nav_nodes_parent_sort
              ON view_nav_nodes(parent_id, sort_order, id);

            CREATE TABLE IF NOT EXISTS view_nav_folder_records (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              folder_id INTEGER NOT NULL,
              table_id INTEGER NOT NULL,
              record_id INTEGER NOT NULL,
              record_label TEXT NOT NULL,
              sort_order INTEGER NOT NULL DEFAULT 0,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              UNIQUE(folder_id, table_id, record_id),
              FOREIGN KEY(folder_id) REFERENCES view_nav_nodes(id),
              FOREIGN KEY(table_id) REFERENCES app_tables(id)
            );

            CREATE INDEX IF NOT EXISTS idx_view_nav_folder_records_folder_sort
              ON view_nav_folder_records(folder_id, sort_order, id);

            CREATE TABLE IF NOT EXISTS record_tag_groups (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              name TEXT NOT NULL UNIQUE,
              sort_order INTEGER NOT NULL DEFAULT 0,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS record_tags (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              name TEXT NOT NULL UNIQUE,
              group_id INTEGER,
              sort_order INTEGER NOT NULL DEFAULT 0,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              FOREIGN KEY(group_id) REFERENCES record_tag_groups(id)
            );

            CREATE INDEX IF NOT EXISTS idx_record_tags_group_sort
              ON record_tags(group_id, sort_order, id);

            CREATE TABLE IF NOT EXISTS record_tag_group_links (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              tag_id INTEGER NOT NULL,
              group_id INTEGER NOT NULL,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              UNIQUE(tag_id, group_id),
              FOREIGN KEY(tag_id) REFERENCES record_tags(id),
              FOREIGN KEY(group_id) REFERENCES record_tag_groups(id)
            );

            CREATE INDEX IF NOT EXISTS idx_record_tag_group_links_group
              ON record_tag_group_links(group_id, tag_id);

            CREATE INDEX IF NOT EXISTS idx_record_tag_group_links_tag
              ON record_tag_group_links(tag_id, group_id);

            CREATE TABLE IF NOT EXISTS record_tag_links (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              tag_id INTEGER NOT NULL,
              table_id INTEGER NOT NULL,
              record_id INTEGER NOT NULL,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              UNIQUE(tag_id, table_id, record_id),
              FOREIGN KEY(tag_id) REFERENCES record_tags(id),
              FOREIGN KEY(table_id) REFERENCES app_tables(id)
            );

            CREATE INDEX IF NOT EXISTS idx_record_tag_links_record
              ON record_tag_links(table_id, record_id, tag_id);

            CREATE INDEX IF NOT EXISTS idx_record_tag_links_tag
              ON record_tag_links(tag_id);

            CREATE TABLE IF NOT EXISTS view_layout_templates (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              name TEXT NOT NULL,
              scope_type TEXT NOT NULL DEFAULT 'folder',
              folder_id INTEGER,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              FOREIGN KEY(folder_id) REFERENCES view_nav_nodes(id)
            );

            CREATE INDEX IF NOT EXISTS idx_view_layout_templates_folder
              ON view_layout_templates(folder_id, scope_type, id);

            CREATE TABLE IF NOT EXISTS view_layout_folder_template_assignments (
              folder_id INTEGER PRIMARY KEY,
              template_id INTEGER NOT NULL,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              FOREIGN KEY(folder_id) REFERENCES view_nav_nodes(id),
              FOREIGN KEY(template_id) REFERENCES view_layout_templates(id)
            );

            CREATE TABLE IF NOT EXISTS view_layout_record_template_assignments (
              folder_record_id INTEGER PRIMARY KEY,
              template_id INTEGER NOT NULL,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              FOREIGN KEY(folder_record_id) REFERENCES view_nav_folder_records(id),
              FOREIGN KEY(template_id) REFERENCES view_layout_templates(id)
            );

            CREATE TABLE IF NOT EXISTS view_layout_template_cards (
              card_id INTEGER PRIMARY KEY AUTOINCREMENT,
              template_id INTEGER NOT NULL,
              preset_id TEXT,
              x REAL NOT NULL,
              y REAL NOT NULL,
              width REAL NOT NULL,
              height REAL NOT NULL,
              visible INTEGER NOT NULL DEFAULT 1,
              background_color TEXT,
              text_color TEXT,
              font_size REAL,
              text_direction TEXT,
              font_weight TEXT,
              text_align TEXT,
              padding REAL,
              padding_top REAL,
              padding_right REAL,
              padding_bottom REAL,
              padding_left REAL,
              border_radius REAL,
              show_label INTEGER,
              auto_height_enabled INTEGER NOT NULL DEFAULT 0,
              push_down_siblings INTEGER NOT NULL DEFAULT 0,
              max_auto_height REAL,
              max_auto_height_behavior TEXT NOT NULL DEFAULT 'scaleToFit',
              sort_order INTEGER NOT NULL DEFAULT 0,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              FOREIGN KEY(template_id) REFERENCES view_layout_templates(id)
            );

            CREATE TABLE IF NOT EXISTS view_layout_template_card_slots (
              slot_id INTEGER PRIMARY KEY AUTOINCREMENT,
              template_id INTEGER NOT NULL,
              card_id INTEGER NOT NULL,
              sort_order INTEGER NOT NULL DEFAULT 0,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              FOREIGN KEY(template_id) REFERENCES view_layout_templates(id),
              FOREIGN KEY(card_id) REFERENCES view_layout_template_cards(card_id)
            );

            CREATE TABLE IF NOT EXISTS view_layout_card_column_bindings (
              template_id INTEGER NOT NULL,
              table_id INTEGER NOT NULL,
              card_id INTEGER NOT NULL,
              sort_order INTEGER NOT NULL,
              column_id INTEGER NOT NULL,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              PRIMARY KEY(template_id, table_id, card_id, sort_order),
              FOREIGN KEY(template_id) REFERENCES view_layout_templates(id),
              FOREIGN KEY(table_id) REFERENCES app_tables(id),
              FOREIGN KEY(card_id) REFERENCES view_layout_template_cards(card_id),
              FOREIGN KEY(column_id) REFERENCES app_table_columns(id)
            );

            CREATE TABLE IF NOT EXISTS view_layout_card_overrides (
              template_id INTEGER NOT NULL,
              table_id INTEGER NOT NULL,
              record_id INTEGER NOT NULL,
              card_id INTEGER NOT NULL,
              offset_x REAL NOT NULL DEFAULT 0,
              offset_y REAL NOT NULL DEFAULT 0,
              offset_width REAL NOT NULL DEFAULT 0,
              offset_height REAL NOT NULL DEFAULT 0,
              visible INTEGER,
              background_color TEXT,
              text_color TEXT,
              font_size REAL,
              text_direction TEXT,
              font_weight TEXT,
              text_align TEXT,
              padding REAL,
              padding_top REAL,
              padding_right REAL,
              padding_bottom REAL,
              padding_left REAL,
              border_radius REAL,
              show_label INTEGER,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              PRIMARY KEY(template_id, table_id, record_id, card_id),
              FOREIGN KEY(template_id) REFERENCES view_layout_templates(id),
              FOREIGN KEY(table_id) REFERENCES app_tables(id),
              FOREIGN KEY(card_id) REFERENCES view_layout_template_cards(card_id)
            );
            ",
        )?;
        self.migrate_record_tag_group_links()?;
        self.migrate_view_layout_template_card_preset_id()?;
        self.migrate_view_layout_template_card_auto_height_settings()?;
        Ok(())
    }

    fn migrate_record_tag_group_links(&self) -> Result<(), DbError> {
        self.conn.execute(
            "
            INSERT OR IGNORE INTO record_tag_group_links (tag_id, group_id)
            SELECT id, group_id
            FROM record_tags
            WHERE group_id IS NOT NULL
            ",
            [],
        )?;
        Ok(())
    }

    fn migrate_view_layout_template_card_preset_id(&self) -> Result<(), DbError> {
        let column_names = self
            .view_layout_template_card_column_names()?;
        let has_preset_id = column_names
            .iter()
            .any(|column_name| column_name == "preset_id");

        if !has_preset_id {
            self.conn.execute(
                "ALTER TABLE view_layout_template_cards ADD COLUMN preset_id TEXT",
                [],
            )?;
        }

        Ok(())
    }

    fn migrate_view_layout_template_card_auto_height_settings(
        &self,
    ) -> Result<(), DbError> {
        let column_names = self.view_layout_template_card_column_names()?;
        let pending_columns = [
            (
                "auto_height_enabled",
                "ALTER TABLE view_layout_template_cards ADD COLUMN auto_height_enabled INTEGER NOT NULL DEFAULT 0",
            ),
            (
                "push_down_siblings",
                "ALTER TABLE view_layout_template_cards ADD COLUMN push_down_siblings INTEGER NOT NULL DEFAULT 0",
            ),
            (
                "max_auto_height",
                "ALTER TABLE view_layout_template_cards ADD COLUMN max_auto_height REAL",
            ),
            (
                "max_auto_height_behavior",
                "ALTER TABLE view_layout_template_cards ADD COLUMN max_auto_height_behavior TEXT NOT NULL DEFAULT 'scaleToFit'",
            ),
        ];

        for (column_name, statement) in pending_columns {
            if column_names.iter().any(|existing| existing == column_name) {
                continue;
            }
            self.conn.execute(statement, [])?;
        }

        Ok(())
    }

    fn view_layout_template_card_column_names(&self) -> Result<Vec<String>, DbError> {
        self.conn
            .prepare("PRAGMA table_info(view_layout_template_cards)")?
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(DbError::from)
    }
}
