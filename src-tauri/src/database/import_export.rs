use super::settings::{
    last_excel_import_tables_setting, notification_settings_setting,
    save_settings_with_app_settings, show_record_ids_in_navigation_setting,
};
use super::*;
use std::io::{Cursor, Read};
use std::path::Path;

impl Db {
    pub fn export_table_csv(&self, payload: ExportTableCsvPayload) -> Result<(), DbError> {
        let output_path = payload.output_path.trim();
        if output_path.is_empty() {
            return Err(DbError::InvalidInput("outputPath is required".into()));
        }

        // 画面表示と同じ文字列をCSVに出すため、表示値付きのレコード一覧を使います。
        let table = self.get_table_summary(payload.table_id)?;
        let columns = self.list_columns(payload.table_id)?;
        let records = self.list_records_with_order(&table.table_name, &columns, "ASC")?;
        // Excelで日本語を開きやすいよう、UTF-8 BOMを先頭に付けます。
        let mut csv = String::from("\u{feff}");
        csv.push_str(
            &columns
                .iter()
                .map(|column| csv_escape_field(&column.display_name))
                .collect::<Vec<_>>()
                .join(","),
        );
        csv.push_str("\r\n");

        for record in &records {
            let row = columns
                .iter()
                .map(|column| {
                    record
                        .display_values
                        .as_object()
                        .and_then(|values| values.get(&column.column_name))
                        .and_then(Value::as_str)
                        .map(csv_escape_field)
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
                .join(",");
            csv.push_str(&row);
            csv.push_str("\r\n");
        }

        fs::write(output_path, csv)?;
        Ok(())
    }

    pub fn import_table_csv(
        &mut self,
        payload: ImportTableCsvPayload,
    ) -> Result<ImportTableCsvResult, DbError> {
        let input_path = payload.input_path.trim();
        if input_path.is_empty() {
            return Err(DbError::InvalidInput("inputPath is required".into()));
        }

        let (headers, rows) = read_csv_import_source(input_path)?;
        let table = self.get_table_summary(payload.table_id)?;
        let columns = self.list_columns(payload.table_id)?;
        let select_option_maps = self.csv_select_option_maps(&columns)?;
        let resolved_mapping =
            resolve_import_column_mapping(&columns, &headers, &payload.column_mapping);
        let preview = build_csv_import_preview(
            &self.conn,
            &table.table_name,
            &columns,
            &headers,
            &rows,
            &resolved_mapping,
            &select_option_maps,
            payload.mode,
        )?;

        if !preview.errors.is_empty() {
            return Err(DbError::InvalidInput(preview.errors.join("\n")));
        }

        // インポートは途中失敗時に中途半端な行を残さないよう、全行を1トランザクションで処理します。
        let tx = self.conn.transaction()?;
        let mut inserted_count = 0;
        let mut updated_count = 0;
        let mut skipped_count = 0;

        for source_row in &rows {
            let values =
                csv_row_to_values(source_row, &columns, &resolved_mapping, &select_option_maps)?;
            match payload.mode {
                ImportTableCsvMode::SkipExistingPrimaryKeys => {
                    // 既存IDは触らず、CSVにしかないIDの行だけ追加します。
                    let id = csv_row_id(&values, source_row.row_number)?;
                    if csv_record_exists(&tx, &table.table_name, id)? {
                        skipped_count += 1;
                        continue;
                    }
                    csv_insert_record(&tx, &table.table_name, &columns, &values, true)?;
                    inserted_count += 1;
                }
                ImportTableCsvMode::AppendIgnoringPrimaryKeys => {
                    // CSVのIDを無視し、DB側に新しいIDを採番させて全行を追加します。
                    csv_insert_record(&tx, &table.table_name, &columns, &values, false)?;
                    inserted_count += 1;
                }
                ImportTableCsvMode::UpsertByPrimaryKey => {
                    // 同じIDがあれば更新、なければCSVのIDを維持して追加します。
                    let id = csv_row_id(&values, source_row.row_number)?;
                    if csv_record_exists(&tx, &table.table_name, id)? {
                        csv_update_record(&tx, &table.table_name, &columns, &values, id)?;
                        updated_count += 1;
                    } else {
                        csv_insert_record(&tx, &table.table_name, &columns, &values, true)?;
                        inserted_count += 1;
                    }
                }
            }
        }
        tx.commit()?;
        let mut details = preview.warnings;
        if skipped_count > 0 && details.is_empty() {
            details.push(format!(
                "{skipped_count}件は既存IDと重複したためスキップしました。"
            ));
        }

        Ok(ImportTableCsvResult {
            status: if skipped_count > 0 {
                "warning".into()
            } else {
                "success".into()
            },
            inserted_count,
            updated_count,
            skipped_count,
            error_count: 0,
            details,
        })
    }

    pub fn inspect_csv_import(
        &self,
        payload: InspectCsvImportPayload,
    ) -> Result<InspectCsvImportResult, DbError> {
        let input_path = payload.input_path.trim();
        if input_path.is_empty() {
            return Err(DbError::InvalidInput("inputPath is required".into()));
        }

        let columns = self.list_columns(payload.table_id)?;
        let (headers, rows) = read_csv_import_source(input_path)?;
        let mapping = resolve_import_column_mapping(&columns, &headers, &[]);

        Ok(InspectCsvImportResult {
            headers,
            row_count: rows.len(),
            column_mappings: import_column_mapping_suggestions(&mapping),
        })
    }

    pub fn preview_csv_import(
        &self,
        payload: PreviewCsvImportPayload,
    ) -> Result<PreviewCsvImportResult, DbError> {
        let input_path = payload.input_path.trim();
        if input_path.is_empty() {
            return Err(DbError::InvalidInput("inputPath is required".into()));
        }

        let table = self.get_table_summary(payload.table_id)?;
        let columns = self.list_columns(payload.table_id)?;
        let select_option_maps = self.csv_select_option_maps(&columns)?;
        let (headers, rows) = read_csv_import_source(input_path)?;
        let resolved_mapping =
            resolve_import_column_mapping(&columns, &headers, &payload.column_mapping);

        build_csv_import_preview(
            &self.conn,
            &table.table_name,
            &columns,
            &headers,
            &rows,
            &resolved_mapping,
            &select_option_maps,
            payload.mode,
        )
    }

    pub fn inspect_excel_tables(
        &self,
        payload: InspectExcelTablesPayload,
    ) -> Result<InspectExcelTablesResult, DbError> {
        let input_path = payload.input_path.trim();
        if input_path.is_empty() {
            return Err(DbError::InvalidInput("inputPath is required".into()));
        }

        let target_table = self.get_table_summary(payload.table_id)?;
        let workbook = ExcelWorkbook::open(input_path)?;
        let tables = workbook.table_infos();
        let last_used_table_name = last_excel_import_tables_setting()
            .get(&payload.table_id)
            .cloned();
        let suggested_table_name =
            suggest_excel_table_name(&target_table, &tables, last_used_table_name.as_deref());

        Ok(InspectExcelTablesResult {
            tables,
            suggested_table_name,
            last_used_table_name,
        })
    }

    pub fn preview_excel_table_import(
        &self,
        payload: PreviewExcelTableImportPayload,
    ) -> Result<PreviewExcelTableImportResult, DbError> {
        let table = self.get_table_summary(payload.table_id)?;
        let columns = self.list_columns(payload.table_id)?;
        let workbook = ExcelWorkbook::open(payload.input_path.trim())?;
        let excel_table = workbook.table_by_name(&payload.excel_table_name)?;
        let select_option_maps = self.csv_select_option_maps(&columns)?;
        let resolved_mapping = resolve_import_column_mapping(
            &columns,
            &excel_table.info.column_names,
            &payload.column_mapping,
        );
        let rows = workbook.table_rows(&excel_table);

        build_excel_import_preview(
            &self.conn,
            &table.table_name,
            &columns,
            &excel_table,
            &rows,
            &resolved_mapping,
            &select_option_maps,
            payload.mode,
        )
    }

    pub fn import_excel_table(
        &mut self,
        payload: ImportExcelTablePayload,
    ) -> Result<ImportTableCsvResult, DbError> {
        let table = self.get_table_summary(payload.table_id)?;
        let columns = self.list_columns(payload.table_id)?;
        let workbook = ExcelWorkbook::open(payload.input_path.trim())?;
        let excel_table = workbook.table_by_name(&payload.excel_table_name)?;
        let select_option_maps = self.csv_select_option_maps(&columns)?;
        let resolved_mapping = resolve_import_column_mapping(
            &columns,
            &excel_table.info.column_names,
            &payload.column_mapping,
        );
        let rows = workbook.table_rows(&excel_table);
        let preview = build_excel_import_preview(
            &self.conn,
            &table.table_name,
            &columns,
            &excel_table,
            &rows,
            &resolved_mapping,
            &select_option_maps,
            payload.mode,
        )?;

        if !preview.errors.is_empty() {
            return Err(DbError::InvalidInput(preview.errors.join("\n")));
        }

        let tx = self.conn.transaction()?;
        let mut inserted_count = 0;
        let mut updated_count = 0;
        let mut skipped_count = 0;

        for source_row in &rows {
            let values =
                excel_row_to_values(source_row, &columns, &resolved_mapping, &select_option_maps)?;
            match payload.mode {
                ImportTableCsvMode::SkipExistingPrimaryKeys => {
                    let id = csv_row_id(&values, source_row.row_number)?;
                    if csv_record_exists(&tx, &table.table_name, id)? {
                        skipped_count += 1;
                        continue;
                    }
                    csv_insert_record(&tx, &table.table_name, &columns, &values, true)?;
                    inserted_count += 1;
                }
                ImportTableCsvMode::AppendIgnoringPrimaryKeys => {
                    csv_insert_record(&tx, &table.table_name, &columns, &values, false)?;
                    inserted_count += 1;
                }
                ImportTableCsvMode::UpsertByPrimaryKey => {
                    let id = csv_row_id(&values, source_row.row_number)?;
                    if csv_record_exists(&tx, &table.table_name, id)? {
                        csv_update_record(&tx, &table.table_name, &columns, &values, id)?;
                        updated_count += 1;
                    } else {
                        csv_insert_record(&tx, &table.table_name, &columns, &values, true)?;
                        inserted_count += 1;
                    }
                }
            }
        }
        tx.commit()?;

        let mut settings = last_excel_import_tables_setting();
        settings.insert(payload.table_id, excel_table.name.clone());
        save_settings_with_app_settings(
            &self.db_path,
            show_record_ids_in_navigation_setting(),
            notification_settings_setting(),
            settings,
        )?;

        let mut details = preview.warnings;
        if skipped_count > 0 && details.is_empty() {
            details.push(format!(
                "{skipped_count}件は既存IDと重複したためスキップしました。"
            ));
        }

        Ok(ImportTableCsvResult {
            status: if skipped_count > 0 || !details.is_empty() {
                "warning".into()
            } else {
                "success".into()
            },
            inserted_count,
            updated_count,
            skipped_count,
            error_count: 0,
            details,
        })
    }
}

fn csv_escape_field(value: &str) -> String {
    // CSVの特殊文字を含む値だけダブルクォートで包み、内部のクォートは2つにします。
    if value.contains(|ch| matches!(ch, ',' | '"' | '\r' | '\n')) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// CSVヘッダーをテーブルカラムへ変換します。
/// 物理名または論理名の完全一致だけを許可し、曖昧な列はここで弾きます。
#[derive(Debug, Clone)]
struct ExcelWorkbook {
    tables: Vec<ExcelTable>,
}

#[derive(Debug, Clone)]
struct ExcelTable {
    info: ExcelTableInfo,
    name: String,
    min_col: usize,
    min_row: usize,
    max_row: usize,
    cells: HashMap<(usize, usize), String>,
}

#[derive(Debug, Clone)]
struct ExcelSourceRow {
    row_number: usize,
    values: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct CsvSourceRow {
    row_number: usize,
    values: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct ResolvedImportColumnMapping {
    column: AppColumn,
    source_column_name: Option<String>,
    matched_by: Option<String>,
}

#[derive(Debug)]
struct WorkbookSheet {
    name: String,
    relationship_id: String,
}

impl ExcelWorkbook {
    fn open(input_path: &str) -> Result<Self, DbError> {
        let path = Path::new(input_path);
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .unwrap_or_default();
        if !matches!(extension.as_str(), "xlsx" | "xlsm") {
            return Err(DbError::InvalidInput(
                "Excel import supports .xlsx and .xlsm files".into(),
            ));
        }

        let bytes = fs::read(path)?;
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
            .map_err(|e| DbError::InvalidInput(format!("Excel file cannot be opened: {e}")))?;
        let shared_strings = read_shared_strings(&mut archive)?;
        let sheets = read_workbook_sheets(&mut archive)?;
        let workbook_rels = read_relationships(&mut archive, "xl/_rels/workbook.xml.rels")?;
        let mut tables = Vec::new();

        for sheet in sheets {
            let Some(sheet_target) = workbook_rels.get(&sheet.relationship_id) else {
                continue;
            };
            let sheet_path = resolve_package_path("xl/workbook.xml", sheet_target);
            let sheet_rels_path = relationships_path(&sheet_path);
            let sheet_rels = read_relationships(&mut archive, &sheet_rels_path)?;
            let table_targets =
                read_worksheet_table_targets(&mut archive, &sheet_path, &sheet_rels)?;

            if table_targets.is_empty() {
                continue;
            }

            let sheet_cells = read_worksheet_cells(&mut archive, &sheet_path, &shared_strings)?;
            for table_path in table_targets {
                let table = read_excel_table(&mut archive, &table_path, &sheet.name, &sheet_cells)?;
                tables.push(table);
            }
        }

        Ok(Self { tables })
    }

    fn table_infos(&self) -> Vec<ExcelTableInfo> {
        self.tables.iter().map(|table| table.info.clone()).collect()
    }

    fn table_by_name(&self, name: &str) -> Result<ExcelTable, DbError> {
        self.tables
            .iter()
            .find(|table| table.name == name || table.info.display_name == name)
            .cloned()
            .ok_or_else(|| DbError::InvalidInput("Excel table was not found".into()))
    }

    fn table_rows(&self, table: &ExcelTable) -> Vec<ExcelSourceRow> {
        let mut rows = Vec::new();
        for row_index in (table.min_row + 1)..=table.max_row {
            let mut values = HashMap::new();
            for (offset, column_name) in table.info.column_names.iter().enumerate() {
                let col_index = table.min_col + offset;
                let value = table
                    .cells
                    .get(&(row_index, col_index))
                    .cloned()
                    .unwrap_or_default();
                values.insert(column_name.clone(), value);
            }
            rows.push(ExcelSourceRow {
                row_number: row_index,
                values,
            });
        }
        rows
    }
}

fn read_zip_text(
    archive: &mut zip::ZipArchive<Cursor<Vec<u8>>>,
    path: &str,
) -> Result<Option<String>, DbError> {
    let mut file = match archive.by_name(path) {
        Ok(file) => file,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(error) => {
            return Err(DbError::InvalidInput(format!(
                "Excel part `{path}` cannot be read: {error}"
            )))
        }
    };
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    Ok(Some(text))
}

fn read_required_zip_text(
    archive: &mut zip::ZipArchive<Cursor<Vec<u8>>>,
    path: &str,
) -> Result<String, DbError> {
    read_zip_text(archive, path)?
        .ok_or_else(|| DbError::InvalidInput(format!("Excel part `{path}` is missing")))
}

fn read_shared_strings(
    archive: &mut zip::ZipArchive<Cursor<Vec<u8>>>,
) -> Result<Vec<String>, DbError> {
    let Some(xml) = read_zip_text(archive, "xl/sharedStrings.xml")? else {
        return Ok(Vec::new());
    };
    let doc = parse_xml(&xml, "sharedStrings.xml")?;
    let mut strings = Vec::new();
    for item in doc.descendants().filter(|node| node.has_tag_name("si")) {
        strings.push(
            item.descendants()
                .filter(|node| node.has_tag_name("t"))
                .filter_map(|node| node.text())
                .collect::<String>(),
        );
    }
    Ok(strings)
}

fn read_workbook_sheets(
    archive: &mut zip::ZipArchive<Cursor<Vec<u8>>>,
) -> Result<Vec<WorkbookSheet>, DbError> {
    let xml = read_required_zip_text(archive, "xl/workbook.xml")?;
    let doc = parse_xml(&xml, "workbook.xml")?;
    let mut sheets = Vec::new();
    for sheet in doc.descendants().filter(|node| node.has_tag_name("sheet")) {
        let name = attr_value(sheet, "name").unwrap_or_default();
        let relationship_id = attr_value(sheet, "id").unwrap_or_default();
        if !name.is_empty() && !relationship_id.is_empty() {
            sheets.push(WorkbookSheet {
                name,
                relationship_id,
            });
        }
    }
    Ok(sheets)
}

fn read_relationships(
    archive: &mut zip::ZipArchive<Cursor<Vec<u8>>>,
    path: &str,
) -> Result<HashMap<String, String>, DbError> {
    let Some(xml) = read_zip_text(archive, path)? else {
        return Ok(HashMap::new());
    };
    let doc = parse_xml(&xml, path)?;
    let mut relationships = HashMap::new();
    for node in doc
        .descendants()
        .filter(|node| node.has_tag_name("Relationship"))
    {
        let id = attr_value(node, "Id").unwrap_or_default();
        let target = attr_value(node, "Target").unwrap_or_default();
        if !id.is_empty() && !target.is_empty() {
            relationships.insert(id, target);
        }
    }
    Ok(relationships)
}

fn read_worksheet_table_targets(
    archive: &mut zip::ZipArchive<Cursor<Vec<u8>>>,
    sheet_path: &str,
    sheet_rels: &HashMap<String, String>,
) -> Result<Vec<String>, DbError> {
    let xml = read_required_zip_text(archive, sheet_path)?;
    let doc = parse_xml(&xml, sheet_path)?;
    let mut targets = Vec::new();
    for table_part in doc
        .descendants()
        .filter(|node| node.has_tag_name("tablePart"))
    {
        let relationship_id = attr_value(table_part, "id").unwrap_or_default();
        if let Some(target) = sheet_rels.get(&relationship_id) {
            targets.push(resolve_package_path(sheet_path, target));
        }
    }
    Ok(targets)
}

fn read_worksheet_cells(
    archive: &mut zip::ZipArchive<Cursor<Vec<u8>>>,
    sheet_path: &str,
    shared_strings: &[String],
) -> Result<HashMap<(usize, usize), String>, DbError> {
    let xml = read_required_zip_text(archive, sheet_path)?;
    let doc = parse_xml(&xml, sheet_path)?;
    let mut cells = HashMap::new();

    for cell in doc.descendants().filter(|node| node.has_tag_name("c")) {
        let Some(address) = attr_value(cell, "r") else {
            continue;
        };
        let Some((col, row)) = split_cell_address(&address) else {
            continue;
        };
        let cell_type = attr_value(cell, "t").unwrap_or_default();
        let raw_value = if cell_type == "inlineStr" {
            cell.descendants()
                .filter(|node| node.has_tag_name("t"))
                .filter_map(|node| node.text())
                .collect::<String>()
        } else {
            cell.children()
                .find(|node| node.has_tag_name("v"))
                .and_then(|node| node.text())
                .unwrap_or_default()
                .to_string()
        };
        let value = match cell_type.as_str() {
            "s" => raw_value
                .parse::<usize>()
                .ok()
                .and_then(|index| shared_strings.get(index))
                .cloned()
                .unwrap_or_default(),
            "b" => {
                if raw_value.trim() == "1" {
                    "true".into()
                } else {
                    "false".into()
                }
            }
            _ => raw_value,
        };
        cells.insert((row, col), value);
    }
    Ok(cells)
}

fn read_excel_table(
    archive: &mut zip::ZipArchive<Cursor<Vec<u8>>>,
    table_path: &str,
    sheet_name: &str,
    sheet_cells: &HashMap<(usize, usize), String>,
) -> Result<ExcelTable, DbError> {
    let xml = read_required_zip_text(archive, table_path)?;
    let doc = parse_xml(&xml, table_path)?;
    let table_node = doc
        .descendants()
        .find(|node| node.has_tag_name("table"))
        .ok_or_else(|| DbError::InvalidInput(format!("Excel table `{table_path}` is invalid")))?;
    let name = attr_value(table_node, "name").unwrap_or_else(|| table_path.to_string());
    let display_name = attr_value(table_node, "displayName").unwrap_or_else(|| name.clone());
    let range = attr_value(table_node, "ref").ok_or_else(|| {
        DbError::InvalidInput(format!("Excel table `{display_name}` has no range"))
    })?;
    let (min_col, min_row, _max_col, max_row) = parse_excel_range(&range)?;
    let totals_row_count = attr_value(table_node, "totalsRowCount")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let data_max_row = max_row.saturating_sub(totals_row_count);
    let column_names = table_node
        .descendants()
        .filter(|node| node.has_tag_name("tableColumn"))
        .filter_map(|node| attr_value(node, "name"))
        .collect::<Vec<_>>();

    if column_names.is_empty() {
        return Err(DbError::InvalidInput(format!(
            "Excel table `{display_name}` has no columns"
        )));
    }

    let row_count = data_max_row.saturating_sub(min_row);
    let info = ExcelTableInfo {
        name: name.clone(),
        display_name,
        sheet_name: sheet_name.to_string(),
        range,
        column_names,
        row_count,
    };

    Ok(ExcelTable {
        info,
        name,
        min_col,
        min_row,
        max_row: data_max_row,
        cells: sheet_cells.clone(),
    })
}

fn parse_xml<'a>(text: &'a str, label: &str) -> Result<roxmltree::Document<'a>, DbError> {
    roxmltree::Document::parse(text)
        .map_err(|e| DbError::InvalidInput(format!("Excel XML `{label}` is invalid: {e}")))
}

fn attr_value(node: roxmltree::Node<'_, '_>, name: &str) -> Option<String> {
    node.attributes()
        .find(|attribute| attribute.name() == name)
        .map(|attribute| attribute.value().to_string())
}

fn relationships_path(part_path: &str) -> String {
    let path = Path::new(part_path);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(part_path);
    let parent = path
        .parent()
        .and_then(|parent| parent.to_str())
        .unwrap_or("");
    if parent.is_empty() {
        format!("_rels/{file_name}.rels")
    } else {
        format!("{parent}/_rels/{file_name}.rels").replace('\\', "/")
    }
}

fn resolve_package_path(base_part: &str, target: &str) -> String {
    if target.starts_with('/') {
        return target.trim_start_matches('/').replace('\\', "/");
    }
    let base_parent = Path::new(base_part)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    normalize_package_path(&base_parent.join(target))
}

fn normalize_package_path(path: &Path) -> String {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                parts.pop();
            }
            std::path::Component::Normal(value) => parts.push(value.to_string_lossy().to_string()),
            _ => {}
        }
    }
    parts.join("/")
}

fn split_cell_address(address: &str) -> Option<(usize, usize)> {
    let letters = address
        .chars()
        .take_while(|char| char.is_ascii_alphabetic())
        .collect::<String>();
    let digits = address
        .chars()
        .skip_while(|char| char.is_ascii_alphabetic())
        .take_while(|char| char.is_ascii_digit())
        .collect::<String>();
    if letters.is_empty() || digits.is_empty() {
        return None;
    }
    Some((excel_column_to_index(&letters)?, digits.parse().ok()?))
}

fn excel_column_to_index(label: &str) -> Option<usize> {
    let mut value = 0usize;
    for char in label.chars() {
        if !char.is_ascii_alphabetic() {
            return None;
        }
        value = value * 26 + (char.to_ascii_uppercase() as usize - 'A' as usize + 1);
    }
    Some(value)
}

fn parse_excel_range(range: &str) -> Result<(usize, usize, usize, usize), DbError> {
    let (start, end) = range
        .split_once(':')
        .ok_or_else(|| DbError::InvalidInput(format!("Excel range `{range}` is invalid")))?;
    let (min_col, min_row) = split_cell_address(start)
        .ok_or_else(|| DbError::InvalidInput(format!("Excel range `{range}` is invalid")))?;
    let (max_col, max_row) = split_cell_address(end)
        .ok_or_else(|| DbError::InvalidInput(format!("Excel range `{range}` is invalid")))?;
    Ok((min_col, min_row, max_col, max_row))
}

fn normalize_match_text(value: &str) -> String {
    value
        .chars()
        .filter(|char| char.is_ascii_alphanumeric())
        .flat_map(|char| char.to_lowercase())
        .collect()
}

fn suggest_excel_table_name(
    table: &AppTableSummary,
    excel_tables: &[ExcelTableInfo],
    last_used_table_name: Option<&str>,
) -> Option<String> {
    if let Some(last_used) = last_used_table_name {
        if excel_tables.iter().any(|item| item.name == last_used) {
            return Some(last_used.to_string());
        }
    }

    for target in [table.table_name.as_str(), table.display_name.as_str()] {
        if let Some(excel_table) = excel_tables
            .iter()
            .find(|item| item.name == target || item.display_name == target)
        {
            return Some(excel_table.name.clone());
        }
    }

    let normalized_targets = [table.table_name.as_str(), table.display_name.as_str()]
        .iter()
        .map(|target| normalize_match_text(target))
        .collect::<Vec<_>>();
    excel_tables
        .iter()
        .find(|item| {
            let normalized_name = normalize_match_text(&item.name);
            let normalized_display_name = normalize_match_text(&item.display_name);
            normalized_targets.iter().any(|target| {
                !target.is_empty()
                    && (normalized_name.contains(target)
                        || normalized_display_name.contains(target)
                        || target.contains(&normalized_name)
                        || target.contains(&normalized_display_name))
            })
        })
        .map(|item| item.name.clone())
}

fn resolve_import_column_mapping(
    columns: &[AppColumn],
    excel_column_names: &[String],
    payload: &[ImportColumnMappingPayload],
) -> Vec<ResolvedImportColumnMapping> {
    let manual = payload
        .iter()
        .filter(|mapping| !mapping.source_column_name.trim().is_empty())
        .map(|mapping| {
            (
                mapping.target_column_name.clone(),
                mapping.source_column_name.trim().to_string(),
            )
        })
        .collect::<HashMap<_, _>>();

    columns
        .iter()
        .map(|column| {
            if let Some(source) = manual.get(&column.column_name) {
                return ResolvedImportColumnMapping {
                    column: column.clone(),
                    source_column_name: Some(source.clone()),
                    matched_by: Some("manual".into()),
                };
            }
            if let Some(source) = excel_column_names
                .iter()
                .find(|name| **name == column.column_name)
            {
                return ResolvedImportColumnMapping {
                    column: column.clone(),
                    source_column_name: Some(source.clone()),
                    matched_by: Some("物理名".into()),
                };
            }
            if let Some(source) = excel_column_names
                .iter()
                .find(|name| **name == column.display_name)
            {
                return ResolvedImportColumnMapping {
                    column: column.clone(),
                    source_column_name: Some(source.clone()),
                    matched_by: Some("論理名".into()),
                };
            }
            ResolvedImportColumnMapping {
                column: column.clone(),
                source_column_name: None,
                matched_by: None,
            }
        })
        .collect()
}

fn import_column_mapping_suggestions(
    mapping: &[ResolvedImportColumnMapping],
) -> Vec<ImportColumnMappingSuggestion> {
    mapping
        .iter()
        .map(|mapping| ImportColumnMappingSuggestion {
            target_column_name: mapping.column.column_name.clone(),
            target_display_name: mapping.column.display_name.clone(),
            source_column_name: mapping.source_column_name.clone(),
            matched_by: mapping.matched_by.clone(),
            is_required: mapping.column.is_required || mapping.column.column_name == "id",
        })
        .collect()
}

fn build_csv_import_preview(
    conn: &Connection,
    table_name: &str,
    columns: &[AppColumn],
    headers: &[String],
    rows: &[CsvSourceRow],
    mapping: &[ResolvedImportColumnMapping],
    select_option_maps: &HashMap<String, HashMap<String, i64>>,
    mode: ImportTableCsvMode,
) -> Result<PreviewCsvImportResult, DbError> {
    let mut errors = validate_import_mapping("CSV", headers, mapping);
    let mut warnings = import_mapping_warnings("CSV", headers, mapping);
    let mut preview_rows = Vec::new();
    let mut inserted_count = 0;
    let mut updated_count = 0;
    let mut unchanged_count = 0;
    let mut skipped_count = 0;

    if errors.is_empty() {
        for row in rows {
            match csv_row_to_values(row, columns, mapping, select_option_maps) {
                Ok(values) => {
                    if preview_rows.len() < 10 {
                        preview_rows.push(csv_preview_row(row, mapping));
                    }
                    match mode {
                        ImportTableCsvMode::SkipExistingPrimaryKeys => {
                            let id = csv_row_id(&values, row.row_number)?;
                            if record_exists(conn, table_name, id)? {
                                skipped_count += 1;
                            } else {
                                inserted_count += 1;
                            }
                        }
                        ImportTableCsvMode::AppendIgnoringPrimaryKeys => {
                            inserted_count += 1;
                        }
                        ImportTableCsvMode::UpsertByPrimaryKey => {
                            let id = csv_row_id(&values, row.row_number)?;
                            if record_exists(conn, table_name, id)? {
                                if record_values_match(conn, table_name, columns, id, &values)? {
                                    unchanged_count += 1;
                                } else {
                                    updated_count += 1;
                                }
                            } else {
                                inserted_count += 1;
                            }
                        }
                    }
                }
                Err(error) => errors.push(error.to_string()),
            }
        }
    }

    if skipped_count > 0 {
        warnings.push(format!(
            "{skipped_count}件は既存IDと重複するため、選択中の方式ではスキップされます。"
        ));
    }
    if unchanged_count > 0 {
        warnings.push(format!(
            "{unchanged_count}件は既存データと同じ内容のため、更新されません。"
        ));
    }

    Ok(PreviewCsvImportResult {
        column_mappings: import_column_mapping_suggestions(mapping),
        preview_rows,
        total_rows: rows.len(),
        inserted_count,
        updated_count,
        unchanged_count,
        skipped_count,
        error_count: errors.len(),
        warnings,
        errors,
    })
}

fn build_excel_import_preview(
    conn: &Connection,
    table_name: &str,
    columns: &[AppColumn],
    excel_table: &ExcelTable,
    rows: &[ExcelSourceRow],
    mapping: &[ResolvedImportColumnMapping],
    select_option_maps: &HashMap<String, HashMap<String, i64>>,
    mode: ImportTableCsvMode,
) -> Result<PreviewExcelTableImportResult, DbError> {
    let mut errors = validate_import_mapping("Excel", &excel_table.info.column_names, mapping);
    let mut warnings = import_mapping_warnings("Excel", &excel_table.info.column_names, mapping);
    let mut preview_rows = Vec::new();
    let mut inserted_count = 0;
    let mut updated_count = 0;
    let mut unchanged_count = 0;
    let mut skipped_count = 0;

    if errors.is_empty() {
        for row in rows {
            match excel_row_to_values(row, columns, mapping, select_option_maps) {
                Ok(values) => {
                    if preview_rows.len() < 10 {
                        preview_rows.push(excel_preview_row(row, mapping));
                    }
                    match mode {
                        ImportTableCsvMode::SkipExistingPrimaryKeys => {
                            let id = csv_row_id(&values, row.row_number)?;
                            if record_exists(conn, table_name, id)? {
                                skipped_count += 1;
                            } else {
                                inserted_count += 1;
                            }
                        }
                        ImportTableCsvMode::AppendIgnoringPrimaryKeys => {
                            inserted_count += 1;
                        }
                        ImportTableCsvMode::UpsertByPrimaryKey => {
                            let id = csv_row_id(&values, row.row_number)?;
                            if record_exists(conn, table_name, id)? {
                                if record_values_match(conn, table_name, columns, id, &values)? {
                                    unchanged_count += 1;
                                } else {
                                    updated_count += 1;
                                }
                            } else {
                                inserted_count += 1;
                            }
                        }
                    }
                }
                Err(error) => errors.push(error.to_string()),
            }
        }
    }

    if skipped_count > 0 {
        warnings.push(format!(
            "{skipped_count}件は既存IDと重複するため、選択中の方式ではスキップされます。"
        ));
    }
    if unchanged_count > 0 {
        warnings.push(format!(
            "{unchanged_count}件は既存データと同じ内容のため、更新されません。"
        ));
    }

    Ok(PreviewExcelTableImportResult {
        excel_table: excel_table.info.clone(),
        column_mappings: import_column_mapping_suggestions(mapping),
        preview_rows,
        total_rows: rows.len(),
        inserted_count,
        updated_count,
        unchanged_count,
        skipped_count,
        error_count: errors.len(),
        warnings,
        errors,
    })
}

fn validate_import_mapping(
    source_label: &str,
    source_names: &[String],
    mapping: &[ResolvedImportColumnMapping],
) -> Vec<String> {
    let mut errors = Vec::new();
    let mut used_sources = HashSet::new();
    for item in mapping {
        let Some(source) = item.source_column_name.as_ref() else {
            errors.push(format!(
                "row 1: {} has no {} column mapping",
                item.column.display_name, source_label
            ));
            continue;
        };
        if !source_names.iter().any(|name| name == source) {
            errors.push(format!("{source_label} column `{source}` was not found"));
            continue;
        }
        if !used_sources.insert(source.clone()) {
            errors.push(format!(
                "{source_label} column `{source}` is mapped more than once"
            ));
        }
    }
    if !mapping
        .iter()
        .any(|item| item.column.column_name == "id" && item.source_column_name.is_some())
    {
        errors.push("id column mapping is required".into());
    }
    errors
}

fn import_mapping_warnings(
    source_label: &str,
    source_names: &[String],
    mapping: &[ResolvedImportColumnMapping],
) -> Vec<String> {
    let used_sources = mapping
        .iter()
        .filter_map(|item| item.source_column_name.as_ref())
        .collect::<HashSet<_>>();
    let unused = source_names
        .iter()
        .filter(|name| !used_sources.contains(name))
        .cloned()
        .collect::<Vec<_>>();
    if unused.is_empty() {
        Vec::new()
    } else {
        vec![format!(
            "{source_label}側の未使用列があります: {}",
            unused.join(", ")
        )]
    }
}

fn read_csv_import_source(input_path: &str) -> Result<(Vec<String>, Vec<CsvSourceRow>), DbError> {
    let mut reader = csv::ReaderBuilder::new().from_path(input_path)?;
    let headers = reader
        .headers()?
        .iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut rows = Vec::new();
    for (row_index, record) in reader.records().enumerate() {
        let record = record?;
        let values = headers
            .iter()
            .enumerate()
            .map(|(index, header)| {
                (
                    header.clone(),
                    record.get(index).unwrap_or_default().to_string(),
                )
            })
            .collect::<HashMap<_, _>>();
        rows.push(CsvSourceRow {
            row_number: row_index + 2,
            values,
        });
    }
    Ok((headers, rows))
}

fn csv_row_to_values(
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

fn csv_preview_row(
    row: &CsvSourceRow,
    mapping: &[ResolvedImportColumnMapping],
) -> HashMap<String, String> {
    mapping
        .iter()
        .filter_map(|item| {
            let source = item.source_column_name.as_ref()?;
            Some((
                item.column.column_name.clone(),
                row.values.get(source).cloned().unwrap_or_default(),
            ))
        })
        .collect()
}

fn excel_row_to_values(
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

fn excel_preview_row(
    row: &ExcelSourceRow,
    mapping: &[ResolvedImportColumnMapping],
) -> HashMap<String, String> {
    mapping
        .iter()
        .filter_map(|item| {
            let source = item.source_column_name.as_ref()?;
            Some((
                item.column.column_name.clone(),
                row.values.get(source).cloned().unwrap_or_default(),
            ))
        })
        .collect()
}

fn record_exists(conn: &Connection, table_name: &str, id: i64) -> Result<bool, DbError> {
    conn.query_row(
        &format!("SELECT 1 FROM \"{}\" WHERE id = ?", table_name),
        [id],
        |_| Ok(()),
    )
    .optional()
    .map(|value| value.is_some())
    .map_err(DbError::from)
}

fn record_values_match(
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

fn csv_row_id(values: &HashMap<String, Value>, row_number: usize) -> Result<i64, DbError> {
    // IDを使う方式では、CSV行のidが空または数値以外なら処理できません。
    values
        .get("id")
        .and_then(Value::as_i64)
        .ok_or_else(|| DbError::InvalidInput(format!("row {}: id is required", row_number)))
}

fn csv_record_exists(
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

fn csv_insert_record(
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

fn csv_update_record(
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
