//! インポート列の対応付けと警告生成を担当します。
//!
//! 自動推定は補助に留め、曖昧な列はプレビューで確認できるようにします。

use super::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub(super) struct ResolvedImportColumnMapping {
    pub(super) column: AppColumn,
    pub(super) source_column_name: Option<String>,
    pub(super) matched_by: Option<String>,
}

pub(super) fn normalize_match_text(value: &str) -> String {
    value
        .chars()
        .filter(|char| char.is_ascii_alphanumeric())
        .flat_map(|char| char.to_lowercase())
        .collect()
}

pub(super) fn resolve_import_column_mapping(
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

pub(super) fn import_column_mapping_suggestions(
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

pub(super) fn validate_import_mapping(
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

pub(super) fn import_mapping_warnings(
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
