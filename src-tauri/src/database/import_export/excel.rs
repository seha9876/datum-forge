//! Excel workbook parsing helpers for table import.

use super::mapping::normalize_match_text;
use super::*;
use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::path::Path;

/// CSVヘッダーをテーブルカラムへ変換します。
/// 物理名または論理名の完全一致だけを許可し、曖昧な列はここで弾きます。
#[derive(Debug, Clone)]
pub(super) struct ExcelWorkbook {
    pub(super) tables: Vec<ExcelTable>,
}

#[derive(Debug, Clone)]
pub(super) struct ExcelTable {
    pub(super) info: ExcelTableInfo,
    pub(super) name: String,
    min_col: usize,
    min_row: usize,
    max_row: usize,
    pub(super) cells: HashMap<(usize, usize), String>,
}

#[derive(Debug, Clone)]
pub(super) struct ExcelSourceRow {
    pub(super) row_number: usize,
    pub(super) values: HashMap<String, String>,
}

#[derive(Debug)]
pub(super) struct WorkbookSheet {
    pub(super) name: String,
    relationship_id: String,
}

impl ExcelWorkbook {
    pub(super) fn open(input_path: &str) -> Result<Self, DbError> {
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

    pub(super) fn table_infos(&self) -> Vec<ExcelTableInfo> {
        self.tables.iter().map(|table| table.info.clone()).collect()
    }

    pub(super) fn table_by_name(&self, name: &str) -> Result<ExcelTable, DbError> {
        self.tables
            .iter()
            .find(|table| table.name == name || table.info.display_name == name)
            .cloned()
            .ok_or_else(|| DbError::InvalidInput("Excel table was not found".into()))
    }

    pub(super) fn table_rows(&self, table: &ExcelTable) -> Vec<ExcelSourceRow> {
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

pub(super) fn suggest_excel_table_name(
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
