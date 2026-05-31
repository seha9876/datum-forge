//! Tauri アプリで使う SQLite バックエンド実装です。
//!
//! このモジュールはフロントエンド IPC の入出力型定義、メタデータ用テーブルの
//! スキーマ初期化、テーブル/カラム/タグ/閲覧レイアウトの永続化を担当します。

use rusqlite::{params, params_from_iter, types::ValueRef, Connection, OptionalExtension, ToSql};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::{HashMap, HashSet},
    env, fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    #[error("csv error: {0}")]
    Csv(#[from] csv::Error),
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppSettingsFile {
    db_path: PathBuf,
    #[serde(default)]
    show_record_ids_in_navigation: Option<bool>,
    #[serde(default)]
    notification_settings: Option<NotificationSettings>,
    #[serde(default)]
    last_excel_import_tables: Option<HashMap<i64, String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NotificationSettings {
    #[serde(default)]
    pub use_per_kind_durations: bool,
    #[serde(default = "default_notification_duration_seconds")]
    pub common_duration_seconds: i64,
    #[serde(default = "default_notification_duration_seconds")]
    pub success_duration_seconds: i64,
    #[serde(default = "default_notification_duration_seconds")]
    pub warning_duration_seconds: i64,
    #[serde(default = "default_notification_duration_seconds")]
    pub error_duration_seconds: i64,
}

/// Frontend settings payload for the database location.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub db_path: String,
    pub show_record_ids_in_navigation: bool,
    pub notification_settings: NotificationSettings,
    pub last_excel_import_tables: HashMap<i64, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNotificationSettingsPayload {
    pub notification_settings: NotificationSettings,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupDbStatus {
    pub state: String,
    pub db_path: Option<String>,
    pub default_db_directory: String,
    pub default_db_file_name: String,
    pub missing_db_path: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDatabasePayload {
    pub db_directory: String,
    pub db_file_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBootstrap {
    pub tables: Vec<AppTableSummary>,
    pub option_groups: Vec<SelectOptionGroup>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppTableSummary {
    pub id: i64,
    pub table_name: String,
    pub display_name: String,
    pub label_column_id: Option<i64>,
    pub sort_order: i64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppColumn {
    pub id: i64,
    pub table_id: i64,
    pub column_name: String,
    pub display_name: String,
    pub field_type: String,
    pub sort_order: i64,
    pub select_option_group_id: Option<i64>,
    pub ref_table_id: Option<i64>,
    pub is_required: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectOptionGroup {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub options: Vec<SelectOption>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectOption {
    pub id: i64,
    pub group_id: i64,
    pub option_no: i64,
    pub sort_order: i64,
    pub label: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableRecord {
    pub id: i64,
    pub values: Value,
    pub display_values: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableDetail {
    pub table: AppTableSummary,
    pub columns: Vec<AppColumn>,
    pub records: Vec<TableRecord>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceChoice {
    pub id: i64,
    pub label: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewNavNode {
    pub id: i64,
    pub node_type: String,
    pub parent_id: Option<i64>,
    pub name: String,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewTableRecordSummary {
    pub id: i64,
    pub label: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewTableSection {
    pub table_id: i64,
    pub table_name: String,
    pub display_name: String,
    pub records: Vec<ViewTableRecordSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewNavFolderRecord {
    pub id: i64,
    pub folder_id: i64,
    pub table_id: i64,
    pub table_name: String,
    pub table_display_name: String,
    pub record_id: i64,
    pub record_label: String,
    pub record_template_id: Option<i64>,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordTagGroup {
    pub id: i64,
    pub name: String,
    pub sort_order: i64,
    pub usage_count: i64,
    pub tag_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordTag {
    pub id: i64,
    pub group_id: Option<i64>,
    pub group_ids: Vec<i64>,
    pub name: String,
    pub sort_order: i64,
    pub usage_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordTagBundle {
    pub groups: Vec<RecordTagGroup>,
    pub tags: Vec<RecordTag>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewLayoutCardItem {
    pub table_id: i64,
    pub card_id: i64,
    pub column_id: Option<i64>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub visible: bool,
    pub background_color: Option<String>,
    pub text_color: Option<String>,
    pub font_size: Option<f64>,
    pub text_direction: Option<String>,
    pub font_weight: Option<String>,
    pub text_align: Option<String>,
    pub padding: Option<f64>,
    pub padding_top: Option<f64>,
    pub padding_right: Option<f64>,
    pub padding_bottom: Option<f64>,
    pub padding_left: Option<f64>,
    pub border_radius: Option<f64>,
    pub show_label: Option<bool>,
    pub has_override: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewLayoutTemplate {
    pub id: i64,
    pub name: String,
    pub scope_type: String,
    pub folder_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewLayoutTemplateCard {
    pub card_id: i64,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub visible: bool,
    pub label: Option<String>,
    pub background_color: Option<String>,
    pub text_color: Option<String>,
    pub font_size: Option<f64>,
    pub text_direction: Option<String>,
    pub font_weight: Option<String>,
    pub text_align: Option<String>,
    pub padding: Option<f64>,
    pub padding_top: Option<f64>,
    pub padding_right: Option<f64>,
    pub padding_bottom: Option<f64>,
    pub padding_left: Option<f64>,
    pub border_radius: Option<f64>,
    pub show_label: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedViewFieldLayout {
    pub templates: Vec<ViewLayoutTemplate>,
    pub active_template_id: Option<i64>,
    pub active_template_name: Option<String>,
    pub items: Vec<ViewLayoutCardItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTablePayload {
    pub table_name: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTablePayload {
    pub table_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportTableCsvPayload {
    /// CSVを書き出す対象テーブルのIDです。
    pub table_id: i64,
    /// 保存ダイアログで選ばれた出力先パスです。
    pub output_path: String,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum ImportTableCsvMode {
    /// CSV内のIDが既存レコードと重複しない行だけ追加します。
    SkipExistingPrimaryKeys,
    /// CSV内のIDを使わず、SQLiteの自動採番で全行を追加します。
    AppendIgnoringPrimaryKeys,
    /// CSV内のIDが既存なら更新し、なければそのIDで追加します。
    UpsertByPrimaryKey,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportTableCsvPayload {
    /// CSVを取り込む対象テーブルのIDです。
    pub table_id: i64,
    /// ファイル選択ダイアログで選ばれたCSVパスです。
    pub input_path: String,
    /// ID重複時の扱いを決めるインポート方式です。
    pub mode: ImportTableCsvMode,
    /// Datum Forge列とCSVヘッダーの対応付けです。
    pub column_mapping: Vec<ExcelColumnMappingPayload>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportTableCsvResult {
    /// フロント側の通知色を決めるための最終結果です。
    pub status: String,
    /// CSVから新しく追加できた行数です。
    pub inserted_count: usize,
    /// 既存IDに対して更新できた行数です。
    pub updated_count: usize,
    /// 既存IDと重複したため取り込まなかった行数です。
    pub skipped_count: usize,
    /// 現在はall-or-nothingなので、成功時は常に0です。
    pub error_count: usize,
    /// 警告や詳細確認に使う補足メッセージです。
    pub details: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectExcelTablesPayload {
    pub table_id: i64,
    pub input_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectCsvImportPayload {
    pub table_id: i64,
    pub input_path: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExcelColumnMappingPayload {
    pub target_column_name: String,
    pub source_column_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectCsvImportResult {
    pub headers: Vec<String>,
    pub row_count: usize,
    pub column_mappings: Vec<ExcelColumnMappingSuggestion>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewCsvImportPayload {
    pub table_id: i64,
    pub input_path: String,
    pub mode: ImportTableCsvMode,
    pub column_mapping: Vec<ExcelColumnMappingPayload>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewExcelTableImportPayload {
    pub table_id: i64,
    pub input_path: String,
    pub excel_table_name: String,
    pub mode: ImportTableCsvMode,
    pub column_mapping: Vec<ExcelColumnMappingPayload>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportExcelTablePayload {
    pub table_id: i64,
    pub input_path: String,
    pub excel_table_name: String,
    pub mode: ImportTableCsvMode,
    pub column_mapping: Vec<ExcelColumnMappingPayload>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExcelTableInfo {
    pub name: String,
    pub display_name: String,
    pub sheet_name: String,
    pub range: String,
    pub column_names: Vec<String>,
    pub row_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectExcelTablesResult {
    pub tables: Vec<ExcelTableInfo>,
    pub suggested_table_name: Option<String>,
    pub last_used_table_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExcelColumnMappingSuggestion {
    pub target_column_name: String,
    pub target_display_name: String,
    pub source_column_name: Option<String>,
    pub matched_by: Option<String>,
    pub is_required: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewExcelTableImportResult {
    pub excel_table: ExcelTableInfo,
    pub column_mappings: Vec<ExcelColumnMappingSuggestion>,
    pub preview_rows: Vec<HashMap<String, String>>,
    pub total_rows: usize,
    pub inserted_count: usize,
    pub updated_count: usize,
    pub unchanged_count: usize,
    pub skipped_count: usize,
    pub error_count: usize,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewCsvImportResult {
    pub column_mappings: Vec<ExcelColumnMappingSuggestion>,
    pub preview_rows: Vec<HashMap<String, String>>,
    pub total_rows: usize,
    pub inserted_count: usize,
    pub updated_count: usize,
    pub unchanged_count: usize,
    pub skipped_count: usize,
    pub error_count: usize,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddColumnPayload {
    pub table_id: i64,
    pub column_name: String,
    pub display_name: String,
    pub field_type: String,
    pub is_required: bool,
    pub select_option_group_id: Option<i64>,
    pub ref_table_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteColumnPayload {
    pub table_id: i64,
    pub column_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateColumnPayload {
    pub table_id: i64,
    pub column_id: i64,
    pub column_name: String,
    pub display_name: String,
    pub is_required: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLabelColumnPayload {
    pub table_id: i64,
    pub label_column_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderColumnsPayload {
    pub table_id: i64,
    pub ordered_column_ids: Vec<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveOption {
    pub option_no: i64,
    pub sort_order: i64,
    pub label: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveOptionGroupPayload {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub options: Vec<SaveOption>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveRecordPayload {
    pub table_id: i64,
    pub record_id: Option<i64>,
    pub values: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteRecordPayload {
    pub table_id: i64,
    pub record_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateViewNavFolderPayload {
    pub parent_id: Option<i64>,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteViewNavFolderPayload {
    pub folder_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddViewNavFolderRecordsPayload {
    pub folder_id: i64,
    pub table_id: i64,
    pub records: Vec<AddViewNavFolderRecordsItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddViewNavFolderRecordsItem {
    pub record_id: i64,
    pub record_label: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveViewNavFolderRecordPayload {
    pub folder_record_id: i64,
}

/// 閲覧目次で、指定フォルダー内レコードを保存したい順番に並べた payload です。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderViewNavFolderRecordsPayload {
    pub folder_id: i64,
    pub ordered_folder_record_ids: Vec<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveRecordTagGroupPayload {
    pub id: Option<i64>,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveRecordTagPayload {
    pub id: Option<i64>,
    pub name: String,
    pub group_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteRecordTagPayload {
    pub tag_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordTagGroupLinkPayload {
    pub tag_id: i64,
    pub group_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachRecordTagPayload {
    pub table_id: i64,
    pub record_id: i64,
    pub tag_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAndAttachRecordTagPayload {
    pub table_id: i64,
    pub record_id: i64,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetachRecordTagPayload {
    pub table_id: i64,
    pub record_id: i64,
    pub tag_id: i64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveViewLayoutCardItem {
    pub card_id: i64,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub visible: bool,
    pub background_color: Option<String>,
    pub text_color: Option<String>,
    pub font_size: Option<f64>,
    pub text_direction: Option<String>,
    pub font_weight: Option<String>,
    pub text_align: Option<String>,
    pub padding: Option<f64>,
    pub padding_top: Option<f64>,
    pub padding_right: Option<f64>,
    pub padding_bottom: Option<f64>,
    pub padding_left: Option<f64>,
    pub border_radius: Option<f64>,
    pub show_label: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateViewLayoutTemplatePayload {
    pub name: String,
    pub scope_type: Option<String>,
    pub folder_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameViewLayoutTemplatePayload {
    pub template_id: i64,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateViewLayoutTemplatePayload {
    pub template_id: i64,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteViewLayoutTemplatePayload {
    pub template_id: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderViewLayoutTemplates {
    pub templates: Vec<ViewLayoutTemplate>,
    pub active_template_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListViewLayoutTemplatesForFolderPayload {
    pub folder_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignViewLayoutFolderTemplatePayload {
    pub folder_id: i64,
    pub template_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignViewLayoutRecordTemplatePayload {
    pub folder_record_id: i64,
    pub template_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearViewLayoutRecordTemplatePayload {
    pub folder_record_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetResolvedViewFieldLayoutPayload {
    pub table_id: i64,
    pub record_id: i64,
    pub folder_id: Option<i64>,
    pub folder_record_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetViewLayoutTemplateCardsPayload {
    pub template_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListViewLayoutCardColumnBindingsPayload {
    pub template_id: i64,
    pub table_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveViewLayoutTemplateCardsPayload {
    pub template_id: i64,
    pub cards: Vec<ViewLayoutTemplateCard>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewLayoutCardColumnBinding {
    pub card_id: i64,
    pub column_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewLayoutCardColumnBindingPayload {
    pub card_id: i64,
    pub column_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveViewLayoutCardColumnBindingsPayload {
    pub template_id: i64,
    pub table_id: i64,
    pub bindings: Vec<ViewLayoutCardColumnBindingPayload>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveViewLayoutCardOverridesPayload {
    pub template_id: i64,
    pub table_id: i64,
    pub record_id: i64,
    pub items: Vec<SaveViewLayoutCardItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetViewLayoutCardOverridePayload {
    pub template_id: i64,
    pub table_id: i64,
    pub record_id: i64,
    pub card_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetViewLayoutCardOverridesPayload {
    pub template_id: i64,
    pub table_id: i64,
    pub record_id: i64,
}

pub struct Db {
    conn: Connection,
    db_path: PathBuf,
}

fn settings_path() -> Result<PathBuf, DbError> {
    Ok(normalize_path(Path::new(".local"))?.join("settings.json"))
}

fn default_db_path() -> Result<PathBuf, DbError> {
    Ok(normalize_path(Path::new(".local"))?.join("datum-forge.sqlite"))
}

fn default_db_directory() -> Result<PathBuf, DbError> {
    Ok(default_db_path()?
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(".")))
}

fn normalize_path(path: &Path) -> Result<PathBuf, DbError> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    Ok(env::current_dir()?.join(path))
}

fn ensure_parent_dir(path: &Path) -> Result<(), DbError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn load_settings() -> Result<AppSettingsFile, DbError> {
    let path = settings_path()?;
    if !path.exists() {
        return Err(DbError::InvalidInput(
            "settings file does not exist".to_string(),
        ));
    }

    let text = fs::read_to_string(path)?;
    let mut settings: AppSettingsFile = serde_json::from_str(&text)
        .map_err(|e| DbError::InvalidInput(format!("settings file is invalid: {e}")))?;
    settings.db_path = normalize_path(&settings.db_path)?;
    Ok(settings)
}

fn save_settings(db_path: &Path) -> Result<(), DbError> {
    let show_record_ids_in_navigation = load_settings()
        .ok()
        .and_then(|settings| settings.show_record_ids_in_navigation)
        .unwrap_or(true);
    let notification_settings = notification_settings_setting();
    let last_excel_import_tables = last_excel_import_tables_setting();

    save_settings_with_app_settings(
        db_path,
        show_record_ids_in_navigation,
        notification_settings,
        last_excel_import_tables,
    )
}

fn save_settings_with_app_settings(
    db_path: &Path,
    show_record_ids_in_navigation: bool,
    notification_settings: NotificationSettings,
    last_excel_import_tables: HashMap<i64, String>,
) -> Result<(), DbError> {
    let path = settings_path()?;
    ensure_parent_dir(&path)?;
    let settings = AppSettingsFile {
        db_path: db_path.to_path_buf(),
        show_record_ids_in_navigation: Some(show_record_ids_in_navigation),
        notification_settings: Some(normalize_notification_settings(notification_settings)),
        last_excel_import_tables: Some(last_excel_import_tables),
    };
    let text = serde_json::to_string_pretty(&settings)
        .map_err(|e| DbError::InvalidInput(format!("settings file cannot be written: {e}")))?;
    fs::write(path, text)?;
    Ok(())
}

fn show_record_ids_in_navigation_setting() -> bool {
    load_settings()
        .ok()
        .and_then(|settings| settings.show_record_ids_in_navigation)
        .unwrap_or(true)
}

fn default_notification_duration_seconds() -> i64 {
    4
}

fn default_notification_settings() -> NotificationSettings {
    NotificationSettings {
        use_per_kind_durations: false,
        common_duration_seconds: default_notification_duration_seconds(),
        success_duration_seconds: default_notification_duration_seconds(),
        warning_duration_seconds: default_notification_duration_seconds(),
        error_duration_seconds: default_notification_duration_seconds(),
    }
}

fn normalize_notification_duration_seconds(value: i64) -> i64 {
    value.clamp(0, 60)
}

fn normalize_notification_settings(settings: NotificationSettings) -> NotificationSettings {
    NotificationSettings {
        use_per_kind_durations: settings.use_per_kind_durations,
        common_duration_seconds: normalize_notification_duration_seconds(
            settings.common_duration_seconds,
        ),
        success_duration_seconds: normalize_notification_duration_seconds(
            settings.success_duration_seconds,
        ),
        warning_duration_seconds: normalize_notification_duration_seconds(
            settings.warning_duration_seconds,
        ),
        error_duration_seconds: normalize_notification_duration_seconds(
            settings.error_duration_seconds,
        ),
    }
}

fn notification_settings_setting() -> NotificationSettings {
    load_settings()
        .ok()
        .and_then(|settings| settings.notification_settings)
        .map(normalize_notification_settings)
        .unwrap_or_else(default_notification_settings)
}

fn last_excel_import_tables_setting() -> HashMap<i64, String> {
    load_settings()
        .ok()
        .and_then(|settings| settings.last_excel_import_tables)
        .unwrap_or_default()
}

fn db_file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("datum-forge")
        .to_string()
}

fn setup_defaults(path: &Path) -> (String, String) {
    let directory = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| default_db_directory().unwrap_or_else(|_| PathBuf::from(".")));
    (directory.to_string_lossy().into_owned(), db_file_stem(path))
}

fn is_supported_db_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "sqlite" | "db"))
        .unwrap_or(false)
}

fn build_db_file_name(input: &str) -> Result<String, DbError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(DbError::InvalidInput("invalid input".to_string()));
    }
    if trimmed == "." || trimmed == ".." || contains_path_separator(trimmed) {
        return Err(DbError::InvalidInput("invalid input".to_string()));
    }

    let path = Path::new(trimmed);
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) if matches!(ext.to_ascii_lowercase().as_str(), "sqlite" | "db") => {
            Ok(trimmed.to_string())
        }
        Some(_) => Err(DbError::InvalidInput("invalid input".to_string())),
        None => Ok(format!("{trimmed}.sqlite")),
    }
}

fn contains_path_separator(value: &str) -> bool {
    value.contains('/') || value.contains('\\')
}

fn require_trimmed<'a>(field_name: &str, value: &'a str) -> Result<&'a str, DbError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DbError::InvalidInput(format!("{field_name} is required")));
    }
    Ok(trimmed)
}

fn parse_group_ids(value: Option<String>) -> Vec<i64> {
    value
        .unwrap_or_default()
        .split(',')
        .filter_map(|item| item.parse::<i64>().ok())
        .collect()
}

fn move_file(source: &Path, target: &Path) -> Result<(), DbError> {
    match fs::rename(source, target) {
        Ok(()) => Ok(()),
        Err(rename_error) => {
            if target.exists() {
                return Err(DbError::Io(rename_error));
            }

            fs::copy(source, target)?;
            if let Err(remove_error) = fs::remove_file(source) {
                let _ = fs::remove_file(target);
                return Err(DbError::Io(remove_error));
            }
            Ok(())
        }
    }
}

impl Db {
    pub fn open_configured() -> Result<Option<Self>, DbError> {
        let settings_path = settings_path()?;
        if !settings_path.exists() {
            return Ok(None);
        }

        let db_path = load_settings()?.db_path;
        if !db_path.exists() {
            return Ok(None);
        }
        if !db_path.is_file() {
            return Err(DbError::InvalidInput("invalid input".to_string()));
        }

        Self::open_path(db_path).map(Some)
    }

    fn open_path(db_path: PathBuf) -> Result<Self, DbError> {
        ensure_parent_dir(&db_path)?;
        let conn = Connection::open(&db_path)?;
        let db = Self { conn, db_path };
        db.initialize()?;
        Ok(db)
    }

    pub fn startup_status(db: Option<&Self>) -> Result<StartupDbStatus, DbError> {
        let default_path = default_db_path()?;
        let (default_db_directory, default_db_file_name) = setup_defaults(&default_path);

        if let Some(db) = db {
            return Ok(StartupDbStatus {
                state: "ready".to_string(),
                db_path: Some(db.db_path.to_string_lossy().into_owned()),
                default_db_directory,
                default_db_file_name,
                missing_db_path: None,
                message: None,
            });
        }

        let settings_path = settings_path()?;
        if !settings_path.exists() {
            return Ok(StartupDbStatus {
                state: "firstLaunch".to_string(),
                db_path: None,
                default_db_directory,
                default_db_file_name,
                missing_db_path: None,
                message: None,
            });
        }

        match load_settings() {
            Ok(settings) => {
                if settings.db_path.exists() {
                    Ok(StartupDbStatus {
                        state: "ready".to_string(),
                        db_path: Some(settings.db_path.to_string_lossy().into_owned()),
                        default_db_directory,
                        default_db_file_name,
                        missing_db_path: None,
                        message: None,
                    })
                } else {
                    let (directory, file_name) = setup_defaults(&settings.db_path);
                    Ok(StartupDbStatus {
                        state: "missingDb".to_string(),
                        db_path: None,
                        default_db_directory: directory,
                        default_db_file_name: file_name,
                        missing_db_path: Some(settings.db_path.to_string_lossy().into_owned()),
                        message: Some("invalid input".to_string()),
                    })
                }
            }
            Err(error) => Ok(StartupDbStatus {
                state: "error".to_string(),
                db_path: None,
                default_db_directory,
                default_db_file_name,
                missing_db_path: None,
                message: Some(error.to_string()),
            }),
        }
    }

    pub fn create_database(payload: CreateDatabasePayload) -> Result<Self, DbError> {
        let directory = payload.db_directory.trim();
        if directory.is_empty() {
            return Err(DbError::InvalidInput("invalid input".to_string()));
        }

        let db_directory = normalize_path(Path::new(directory))?;
        let metadata = fs::metadata(&db_directory)
            .map_err(|_| DbError::InvalidInput("invalid input".to_string()))?;
        if !metadata.is_dir() {
            return Err(DbError::InvalidInput("invalid input".to_string()));
        }

        let file_name = build_db_file_name(&payload.db_file_name)?;
        let db_path = db_directory.join(file_name);
        if db_path.exists() {
            return Err(DbError::InvalidInput("invalid path".to_string()));
        }

        let db = Self::open_path(db_path)?;
        save_settings(&db.db_path)?;
        Ok(db)
    }

    pub fn open_existing_database(db_file: String) -> Result<Self, DbError> {
        let trimmed = db_file.trim();
        if trimmed.is_empty() {
            return Err(DbError::InvalidInput("invalid input".to_string()));
        }

        let db_path = normalize_path(Path::new(trimmed))?;
        let metadata = fs::metadata(&db_path)
            .map_err(|_| DbError::InvalidInput("invalid input".to_string()))?;
        if !metadata.is_file() {
            return Err(DbError::InvalidInput("invalid input".to_string()));
        }
        if !is_supported_db_file(&db_path) {
            return Err(DbError::InvalidInput("invalid input".to_string()));
        }

        let db = Self::open_path(db_path)?;
        save_settings(&db.db_path)?;
        Ok(db)
    }

    pub fn settings(&self) -> AppSettings {
        AppSettings {
            db_path: self.db_path.to_string_lossy().into_owned(),
            show_record_ids_in_navigation: show_record_ids_in_navigation_setting(),
            notification_settings: notification_settings_setting(),
            last_excel_import_tables: last_excel_import_tables_setting(),
        }
    }

    pub fn update_record_id_visibility(&mut self, show: bool) -> Result<AppSettings, DbError> {
        save_settings_with_app_settings(
            &self.db_path,
            show,
            notification_settings_setting(),
            last_excel_import_tables_setting(),
        )?;
        Ok(self.settings())
    }

    pub fn update_notification_settings(
        &mut self,
        payload: UpdateNotificationSettingsPayload,
    ) -> Result<AppSettings, DbError> {
        save_settings_with_app_settings(
            &self.db_path,
            show_record_ids_in_navigation_setting(),
            payload.notification_settings,
            last_excel_import_tables_setting(),
        )?;
        Ok(self.settings())
    }

    pub fn update_db_directory(&mut self, db_directory: String) -> Result<AppSettings, DbError> {
        let trimmed = db_directory.trim();
        if trimmed.is_empty() {
            return Err(DbError::InvalidInput("invalid input".to_string()));
        }

        let next_directory = normalize_path(Path::new(trimmed))?;
        let metadata = fs::metadata(&next_directory)
            .map_err(|_| DbError::InvalidInput("invalid input".to_string()))?;
        if !metadata.is_dir() {
            return Err(DbError::InvalidInput("invalid input".to_string()));
        }

        let db_file_name = self
            .db_path
            .file_name()
            .ok_or_else(|| DbError::InvalidInput("invalid input".to_string()))?;
        let next_path = next_directory.join(db_file_name);
        if self.db_path == next_path {
            save_settings(&next_path)?;
            return Ok(self.settings());
        }

        if next_path.exists() {
            return Err(DbError::InvalidInput("invalid path".to_string()));
        }

        self.conn
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        let fallback_conn = Connection::open_in_memory()?;
        let old_conn = std::mem::replace(&mut self.conn, fallback_conn);
        drop(old_conn);

        if let Err(error) = move_file(&self.db_path, &next_path) {
            if self.db_path.exists() {
                self.conn = Connection::open(&self.db_path)?;
            }
            return Err(error);
        }

        let next_conn = match Connection::open(&next_path) {
            Ok(conn) => conn,
            Err(error) => {
                let _ = fs::rename(&next_path, &self.db_path);
                if self.db_path.exists() {
                    self.conn = Connection::open(&self.db_path)?;
                }
                return Err(DbError::Sql(error));
            }
        };
        self.conn = next_conn;
        self.db_path = next_path;
        save_settings(&self.db_path)?;
        self.initialize()?;
        Ok(self.settings())
    }

    pub fn rename_db_file(&mut self, db_file_name: String) -> Result<AppSettings, DbError> {
        let trimmed = db_file_name.trim();
        if trimmed.is_empty() {
            return Err(DbError::InvalidInput("invalid input".to_string()));
        }
        if trimmed == "." || trimmed == ".." || contains_path_separator(trimmed) {
            return Err(DbError::InvalidInput("invalid input".to_string()));
        }

        let current_extension = self
            .db_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("sqlite");
        let next_file_name = format!("{trimmed}.{current_extension}");

        let parent = self
            .db_path
            .parent()
            .ok_or_else(|| DbError::InvalidInput("invalid input".to_string()))?;
        let next_path = parent.join(next_file_name);
        if self.db_path == next_path {
            save_settings(&next_path)?;
            return Ok(self.settings());
        }
        if next_path.exists() {
            return Err(DbError::InvalidInput("invalid path".to_string()));
        }

        self.conn
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        let fallback_conn = Connection::open_in_memory()?;
        let old_conn = std::mem::replace(&mut self.conn, fallback_conn);
        drop(old_conn);

        if let Err(error) = move_file(&self.db_path, &next_path) {
            if self.db_path.exists() {
                self.conn = Connection::open(&self.db_path)?;
            }
            return Err(error);
        }

        let next_conn = match Connection::open(&next_path) {
            Ok(conn) => conn,
            Err(error) => {
                let _ = fs::rename(&next_path, &self.db_path);
                if self.db_path.exists() {
                    self.conn = Connection::open(&self.db_path)?;
                }
                return Err(DbError::Sql(error));
            }
        };
        self.conn = next_conn;
        self.db_path = next_path;
        save_settings(&self.db_path)?;
        self.initialize()?;
        Ok(self.settings())
    }

    pub fn open_db_file(&mut self, db_file: String) -> Result<AppSettings, DbError> {
        let trimmed = db_file.trim();
        if trimmed.is_empty() {
            return Err(DbError::InvalidInput("invalid input".to_string()));
        }

        let next_path = normalize_path(Path::new(trimmed))?;
        let metadata = fs::metadata(&next_path)
            .map_err(|_| DbError::InvalidInput("invalid input".to_string()))?;
        if !metadata.is_file() {
            return Err(DbError::InvalidInput("invalid input".to_string()));
        }
        if !is_supported_db_file(&next_path) {
            return Err(DbError::InvalidInput("invalid input".to_string()));
        }

        if self.db_path == next_path {
            save_settings(&next_path)?;
            return Ok(self.settings());
        }

        let next_conn = Connection::open(&next_path)?;
        let old_conn = std::mem::replace(&mut self.conn, next_conn);
        drop(old_conn);

        self.db_path = next_path;
        save_settings(&self.db_path)?;
        self.initialize()?;
        Ok(self.settings())
    }

    fn initialize(&self) -> Result<(), DbError> {
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
              sort_order INTEGER NOT NULL DEFAULT 0,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              FOREIGN KEY(template_id) REFERENCES view_layout_templates(id)
            );

            CREATE TABLE IF NOT EXISTS view_layout_card_column_bindings (
              template_id INTEGER NOT NULL,
              table_id INTEGER NOT NULL,
              card_id INTEGER NOT NULL,
              column_id INTEGER NOT NULL,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              PRIMARY KEY(template_id, table_id, card_id),
              UNIQUE(template_id, table_id, column_id),
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

    pub fn bootstrap(&self) -> Result<AppBootstrap, DbError> {
        Ok(AppBootstrap {
            tables: self.list_tables()?,
            option_groups: self.list_option_groups()?,
        })
    }

    pub fn create_table(&self, payload: CreateTablePayload) -> Result<i64, DbError> {
        validate_identifier(&payload.table_name)?;
        let sort_order = self.next_sort_order("app_tables", None)?;
        self.conn.execute(
            "INSERT INTO app_tables (table_name, display_name, sort_order) VALUES (?, ?, ?)",
            params![payload.table_name, payload.display_name, sort_order],
        )?;
        let table_id = self.conn.last_insert_rowid();
        let table_name = self.table_name_by_id(table_id)?;
        self.conn.execute(
            &format!(
                "CREATE TABLE \"{}\" (id INTEGER PRIMARY KEY AUTOINCREMENT)",
                table_name
            ),
            [],
        )?;

        let sort_order = self.next_sort_order("app_table_columns", Some(table_id))?;
        self.conn.execute(
            "
            INSERT INTO app_table_columns (
              table_id, column_name, display_name, field_type, sort_order, is_required
            ) VALUES (?, 'id', 'ID', 'integer', ?, 1)
            ",
            params![table_id, sort_order],
        )?;
        let id_column_id = self.conn.last_insert_rowid();
        self.conn.execute(
            "UPDATE app_tables SET label_column_id = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            params![id_column_id, table_id],
        )?;
        Ok(table_id)
    }

    pub fn delete_table(&self, payload: DeleteTablePayload) -> Result<(), DbError> {
        let table = self.get_table_summary(payload.table_id)?;
        let referrers = self.table_reference_sources(payload.table_id)?;
        if !referrers.is_empty() {
            return Err(DbError::InvalidInput(format!(
                "他のテーブルから参照されているため削除できません: {}",
                referrers.join(", ")
            )));
        }

        self.conn.execute_batch("BEGIN IMMEDIATE TRANSACTION")?;
        let result = (|| -> Result<(), DbError> {
            self.conn.execute(
                "DELETE FROM record_tag_links WHERE table_id = ?",
                [payload.table_id],
            )?;
            self.conn.execute(
                "
                DELETE FROM view_layout_record_template_assignments
                WHERE folder_record_id IN (
                  SELECT id FROM view_nav_folder_records WHERE table_id = ?
                )
                ",
                [payload.table_id],
            )?;
            self.conn.execute(
                "DELETE FROM view_nav_folder_records WHERE table_id = ?",
                [payload.table_id],
            )?;
            self.conn.execute(
                "DELETE FROM view_layout_card_overrides WHERE table_id = ?",
                [payload.table_id],
            )?;
            self.conn.execute(
                "DELETE FROM view_layout_card_column_bindings WHERE table_id = ?",
                [payload.table_id],
            )?;
            self.conn.execute(
                "DELETE FROM app_table_columns WHERE table_id = ?",
                [payload.table_id],
            )?;
            self.conn
                .execute("DELETE FROM app_tables WHERE id = ?", [payload.table_id])?;
            self.conn
                .execute(&format!("DROP TABLE \"{}\"", table.table_name), [])?;
            Ok(())
        })();

        match result {
            Ok(()) => {
                self.conn.execute_batch("COMMIT")?;
                Ok(())
            }
            Err(error) => {
                let _ = self.conn.execute_batch("ROLLBACK");
                Err(error)
            }
        }
    }

    pub fn add_column(&self, payload: AddColumnPayload) -> Result<(), DbError> {
        validate_identifier(&payload.column_name)?;
        validate_field_type(&payload.field_type)?;
        if payload.field_type == "single_select" && payload.select_option_group_id.is_none() {
            return Err(DbError::InvalidInput(
                "single_select requires selectOptionGroupId".into(),
            ));
        }
        if payload.field_type == "reference" && payload.ref_table_id.is_none() {
            return Err(DbError::InvalidInput(
                "reference requires refTableId".into(),
            ));
        }

        let table_name = self.table_name_by_id(payload.table_id)?;
        let sort_order = self.next_sort_order("app_table_columns", Some(payload.table_id))?;
        let current_label_column_name = self.label_column_name(payload.table_id)?;
        let tx = self.conn.unchecked_transaction()?;
        // 物理列の追加とメタ情報登録をまとめ、途中失敗時に実テーブルだけ変わる状態を避けます。
        tx.execute(
            &format!(
                "ALTER TABLE \"{}\" ADD COLUMN \"{}\" {}",
                table_name,
                payload.column_name,
                sqlite_type_for(&payload.field_type)
            ),
            [],
        )?;

        tx.execute(
            "
            INSERT INTO app_table_columns (
              table_id, column_name, display_name, field_type, sort_order,
              select_option_group_id, ref_table_id, is_required
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ",
            params![
                payload.table_id,
                payload.column_name,
                payload.display_name,
                payload.field_type,
                sort_order,
                payload.select_option_group_id,
                payload.ref_table_id,
                bool_to_i64(payload.is_required)
            ],
        )?;

        if current_label_column_name == "id" && payload.column_name != "id" {
            let new_column_id = tx.last_insert_rowid();
            tx.execute(
                "UPDATE app_tables SET label_column_id = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                params![new_column_id, payload.table_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn delete_column(&self, payload: DeleteColumnPayload) -> Result<(), DbError> {
        let table_name = self.table_name_by_id(payload.table_id)?;
        let (column_name, current_label_column_id): (String, Option<i64>) = self.conn.query_row(
            "
            SELECT c.column_name, t.label_column_id
            FROM app_table_columns c
            JOIN app_tables t ON t.id = c.table_id
            WHERE c.id = ? AND c.table_id = ?
            ",
            params![payload.column_id, payload.table_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        if column_name == "id" {
            return Err(DbError::InvalidInput("id column cannot be deleted".into()));
        }

        self.conn.execute(
            &format!(
                "ALTER TABLE \"{}\" DROP COLUMN \"{}\"",
                table_name, column_name
            ),
            [],
        )?;

        self.conn.execute(
            "DELETE FROM app_table_columns WHERE id = ?",
            [payload.column_id],
        )?;

        if current_label_column_id == Some(payload.column_id) {
            let next_label_column_id: Option<i64> = self
                .conn
                .query_row(
                    "
                    SELECT id
                    FROM app_table_columns
                    WHERE table_id = ? AND column_name != 'id'
                    ORDER BY sort_order, id
                    LIMIT 1
                    ",
                    [payload.table_id],
                    |row| row.get(0),
                )
                .optional()?;

            self.conn.execute(
                "UPDATE app_tables SET label_column_id = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                params![next_label_column_id, payload.table_id],
            )?;
        }

        self.resequence_column_sort_order(payload.table_id)?;

        Ok(())
    }

    pub fn update_column(&self, payload: UpdateColumnPayload) -> Result<(), DbError> {
        if payload.display_name.trim().is_empty() {
            return Err(DbError::InvalidInput("display name is required".into()));
        }

        let table_name = self.table_name_by_id(payload.table_id)?;
        let (current_column_name, field_type): (String, String) = self.conn.query_row(
            "
            SELECT column_name, field_type
            FROM app_table_columns
            WHERE id = ? AND table_id = ?
            ",
            params![payload.column_id, payload.table_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        if current_column_name == "id" && payload.column_name != "id" {
            return Err(DbError::InvalidInput(
                "id column name cannot be changed".into(),
            ));
        }

        if current_column_name != payload.column_name {
            validate_identifier(&payload.column_name)?;
            self.conn.execute(
                &format!(
                    "ALTER TABLE \"{}\" RENAME COLUMN \"{}\" TO \"{}\"",
                    table_name, current_column_name, payload.column_name
                ),
                [],
            )?;
        }

        validate_field_type(&field_type)?;
        self.conn.execute(
            "
            UPDATE app_table_columns
            SET column_name = ?, display_name = ?, is_required = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ? AND table_id = ?
            ",
            params![
                payload.column_name,
                payload.display_name,
                bool_to_i64(payload.is_required),
                payload.column_id,
                payload.table_id
            ],
        )?;

        Ok(())
    }

    pub fn update_label_column(&self, payload: UpdateLabelColumnPayload) -> Result<(), DbError> {
        if let Some(label_column_id) = payload.label_column_id {
            let column_exists: Option<i64> = self
                .conn
                .query_row(
                    "SELECT id FROM app_table_columns WHERE id = ? AND table_id = ?",
                    params![label_column_id, payload.table_id],
                    |row| row.get(0),
                )
                .optional()?;

            if column_exists.is_none() {
                return Err(DbError::InvalidInput(
                    "label column must belong to the selected table".into(),
                ));
            }
        }

        self.conn.execute(
            "UPDATE app_tables SET label_column_id = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            params![payload.label_column_id, payload.table_id],
        )?;

        Ok(())
    }

    pub fn reorder_columns(&self, payload: ReorderColumnsPayload) -> Result<(), DbError> {
        let current_ids = self
            .list_columns(payload.table_id)?
            .into_iter()
            .map(|column| column.id)
            .collect::<Vec<_>>();

        if current_ids.len() != payload.ordered_column_ids.len() {
            return Err(DbError::InvalidInput(
                "orderedColumnIds length mismatch".into(),
            ));
        }

        let mut sorted_current = current_ids.clone();
        let mut sorted_requested = payload.ordered_column_ids.clone();
        sorted_current.sort_unstable();
        sorted_requested.sort_unstable();

        if sorted_current != sorted_requested {
            return Err(DbError::InvalidInput(
                "orderedColumnIds must match current table columns".into(),
            ));
        }

        for (index, column_id) in payload.ordered_column_ids.into_iter().enumerate() {
            self.conn.execute(
                "UPDATE app_table_columns SET sort_order = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND table_id = ?",
                params![index as i64 + 1, column_id, payload.table_id],
            )?;
        }

        Ok(())
    }

    pub fn save_option_group(&self, payload: SaveOptionGroupPayload) -> Result<i64, DbError> {
        if payload.name.trim().is_empty() {
            return Err(DbError::InvalidInput(
                "option group name is required".into(),
            ));
        }
        let group_id = if let Some(id) = payload.id {
            self.conn.execute(
                "UPDATE select_option_groups SET name = ?, description = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                params![payload.name, payload.description, id],
            )?;
            self.conn
                .execute("DELETE FROM select_options WHERE group_id = ?", [id])?;
            id
        } else {
            self.conn.execute(
                "INSERT INTO select_option_groups (name, description) VALUES (?, ?)",
                params![payload.name, payload.description],
            )?;
            self.conn.last_insert_rowid()
        };

        for option in payload
            .options
            .into_iter()
            .filter(|item| !item.label.trim().is_empty())
        {
            self.conn.execute(
                "INSERT INTO select_options (group_id, option_no, sort_order, label) VALUES (?, ?, ?, ?)",
                params![group_id, option.option_no, option.sort_order, option.label],
            )?;
        }
        Ok(group_id)
    }

    pub fn get_table_detail(&self, table_id: i64) -> Result<TableDetail, DbError> {
        let table = self.get_table_summary(table_id)?;
        let columns = self.list_columns(table_id)?;
        let records = self.list_records(&table.table_name, &columns)?;
        Ok(TableDetail {
            table,
            columns,
            records,
        })
    }

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
            resolve_excel_column_mapping(&columns, &headers, &payload.column_mapping);
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
        let mapping = resolve_excel_column_mapping(&columns, &headers, &[]);

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
            resolve_excel_column_mapping(&columns, &headers, &payload.column_mapping);

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
        let resolved_mapping = resolve_excel_column_mapping(
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
        let resolved_mapping = resolve_excel_column_mapping(
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

    pub fn save_record(&self, payload: SaveRecordPayload) -> Result<(), DbError> {
        let table = self.get_table_summary(payload.table_id)?;
        let columns = self
            .list_columns(payload.table_id)?
            .into_iter()
            .filter(|column| column.column_name != "id")
            .collect::<Vec<_>>();
        if columns.is_empty() {
            return Ok(());
        }

        let object = payload
            .values
            .as_object()
            .ok_or_else(|| DbError::InvalidInput("values must be an object".into()))?;

        if let Some(column) = columns.iter().find(|column| {
            column.is_required && is_required_value_empty(object.get(&column.column_name))
        }) {
            return Err(DbError::InvalidInput(format!(
                "{} is required",
                column.display_name
            )));
        }

        if let Some(record_id) = payload.record_id {
            let assignments = columns
                .iter()
                .map(|column| format!("\"{}\" = ?", column.column_name))
                .collect::<Vec<_>>();
            let mut values = columns
                .iter()
                .map(|column| to_sql_value(object.get(&column.column_name), &column.field_type))
                .collect::<Vec<_>>();
            values.push(Box::new(record_id));
            self.conn.execute(
                &format!(
                    "UPDATE \"{}\" SET {} WHERE id = ?",
                    table.table_name,
                    assignments.join(", ")
                ),
                params_from_iter(values.iter().map(|value| value.as_ref())),
            )?;
        } else {
            let column_names = columns
                .iter()
                .map(|column| format!("\"{}\"", column.column_name))
                .collect::<Vec<_>>();
            let placeholders = std::iter::repeat("?")
                .take(columns.len())
                .collect::<Vec<_>>();
            let values = columns
                .iter()
                .map(|column| to_sql_value(object.get(&column.column_name), &column.field_type))
                .collect::<Vec<_>>();
            self.conn.execute(
                &format!(
                    "INSERT INTO \"{}\" ({}) VALUES ({})",
                    table.table_name,
                    column_names.join(", "),
                    placeholders.join(", ")
                ),
                params_from_iter(values.iter().map(|value| value.as_ref())),
            )?;
        }
        Ok(())
    }

    pub fn delete_record(&self, payload: DeleteRecordPayload) -> Result<(), DbError> {
        let table = self.get_table_summary(payload.table_id)?;

        // レコード本体を消す前に、タグ紐付けと閲覧ナビ上の配置を削除します。
        self.conn.execute(
            "DELETE FROM record_tag_links WHERE table_id = ? AND record_id = ?",
            params![payload.table_id, payload.record_id],
        )?;
        self.conn.execute(
            "
            DELETE FROM view_layout_record_template_assignments
            WHERE folder_record_id IN (
              SELECT id FROM view_nav_folder_records
              WHERE table_id = ? AND record_id = ?
            )
            ",
            params![payload.table_id, payload.record_id],
        )?;
        self.conn.execute(
            "DELETE FROM view_nav_folder_records WHERE table_id = ? AND record_id = ?",
            params![payload.table_id, payload.record_id],
        )?;
        self.conn.execute(
            &format!("DELETE FROM \"{}\" WHERE id = ?", table.table_name),
            params![payload.record_id],
        )?;

        Ok(())
    }

    pub fn get_reference_choices(&self, table_id: i64) -> Result<Vec<ReferenceChoice>, DbError> {
        let table = self.get_table_summary(table_id)?;
        let label_column = self.label_column_name(table_id)?;
        let mut stmt = self.conn.prepare(&format!(
            "SELECT id, COALESCE(CAST(\"{}\" AS TEXT), '') FROM \"{}\" ORDER BY id",
            label_column, table.table_name
        ))?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let label: String = row.get(1)?;
            Ok(ReferenceChoice {
                id,
                label: format!("{}:{}", id, label),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

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

    fn list_tables(&self) -> Result<Vec<AppTableSummary>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, table_name, display_name, label_column_id, sort_order FROM app_tables ORDER BY sort_order, id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(AppTableSummary {
                id: row.get(0)?,
                table_name: row.get(1)?,
                display_name: row.get(2)?,
                label_column_id: row.get(3)?,
                sort_order: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    fn get_table_summary(&self, table_id: i64) -> Result<AppTableSummary, DbError> {
        self.conn
            .query_row(
                "SELECT id, table_name, display_name, label_column_id, sort_order FROM app_tables WHERE id = ?",
                [table_id],
                |row| {
                    Ok(AppTableSummary {
                        id: row.get(0)?,
                        table_name: row.get(1)?,
                        display_name: row.get(2)?,
                        label_column_id: row.get(3)?,
                        sort_order: row.get(4)?,
                    })
                },
            )
            .map_err(DbError::from)
    }

    fn list_columns(&self, table_id: i64) -> Result<Vec<AppColumn>, DbError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT id, table_id, column_name, display_name, field_type, sort_order,
                   select_option_group_id, ref_table_id, is_required
            FROM app_table_columns
            WHERE table_id = ?
            ORDER BY sort_order, id
            ",
        )?;
        let rows = stmt.query_map([table_id], |row| {
            Ok(AppColumn {
                id: row.get(0)?,
                table_id: row.get(1)?,
                column_name: row.get(2)?,
                display_name: row.get(3)?,
                field_type: row.get(4)?,
                sort_order: row.get(5)?,
                select_option_group_id: row.get(6)?,
                ref_table_id: row.get(7)?,
                is_required: row.get::<_, i64>(8)? != 0,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    fn list_option_groups(&self) -> Result<Vec<SelectOptionGroup>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, description FROM select_option_groups ORDER BY name")?;
        let groups = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;

        let mut result = Vec::new();
        for group in groups {
            let (id, name, description) = group?;
            result.push(SelectOptionGroup {
                id,
                name,
                description,
                options: self.list_options(id)?,
            });
        }
        Ok(result)
    }

    fn list_options(&self, group_id: i64) -> Result<Vec<SelectOption>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, group_id, option_no, sort_order, label FROM select_options WHERE group_id = ? ORDER BY sort_order, option_no",
        )?;
        let rows = stmt.query_map([group_id], |row| {
            Ok(SelectOption {
                id: row.get(0)?,
                group_id: row.get(1)?,
                option_no: row.get(2)?,
                sort_order: row.get(3)?,
                label: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    fn list_records(
        &self,
        table_name: &str,
        columns: &[AppColumn],
    ) -> Result<Vec<TableRecord>, DbError> {
        self.list_records_with_order(table_name, columns, "ASC")
    }

    fn list_records_with_order(
        &self,
        table_name: &str,
        columns: &[AppColumn],
        id_order: &str,
    ) -> Result<Vec<TableRecord>, DbError> {
        let selected_columns = columns
            .iter()
            .map(|column| format!("\"{}\"", column.column_name))
            .collect::<Vec<_>>();
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {} FROM \"{}\" ORDER BY id {}",
            selected_columns.join(", "),
            table_name,
            id_order
        ))?;
        let mut rows = stmt.query([])?;
        let mut records = Vec::new();

        while let Some(row) = rows.next()? {
            let mut values = Map::new();
            let mut display_values = Map::new();
            let mut record_id = 0;

            for (index, column) in columns.iter().enumerate() {
                let value = sqlite_value_to_json(row.get_ref(index)?)?;
                if column.column_name == "id" {
                    record_id = value.as_i64().unwrap_or_default();
                }
                let display = self.display_value_for_column(column, &value)?;
                values.insert(column.column_name.clone(), value);
                display_values.insert(column.column_name.clone(), Value::String(display));
            }

            records.push(TableRecord {
                id: record_id,
                values: Value::Object(values),
                display_values: Value::Object(display_values),
            });
        }

        Ok(records)
    }

    fn display_value_for_column(
        &self,
        column: &AppColumn,
        value: &Value,
    ) -> Result<String, DbError> {
        if value.is_null() {
            return Ok(String::new());
        }

        match column.field_type.as_str() {
            "boolean" => Ok(if value.as_i64().unwrap_or_default() == 0 {
                "false".into()
            } else {
                "true".into()
            }),
            "single_select" => {
                let group_id = column.select_option_group_id.ok_or_else(|| {
                    DbError::InvalidInput("single_select group is missing".into())
                })?;
                let option_no = value.as_i64().unwrap_or_default();
                let label: Option<String> = self
                    .conn
                    .query_row(
                        "SELECT label FROM select_options WHERE group_id = ? AND option_no = ?",
                        params![group_id, option_no],
                        |row| row.get(0),
                    )
                    .optional()?;
                Ok(label.unwrap_or_else(|| option_no.to_string()))
            }
            "reference" => {
                let ref_table_id = column
                    .ref_table_id
                    .ok_or_else(|| DbError::InvalidInput("reference target is missing".into()))?;
                let table = self.get_table_summary(ref_table_id)?;
                let label_column = self.label_column_name(ref_table_id)?;
                let ref_id = value.as_i64().unwrap_or_default();
                let label: Option<String> = self
                    .conn
                    .query_row(
                        &format!(
                            "SELECT COALESCE(CAST(\"{}\" AS TEXT), '') FROM \"{}\" WHERE id = ?",
                            label_column, table.table_name
                        ),
                        [ref_id],
                        |row| row.get(0),
                    )
                    .optional()?;
                Ok(format!("{}:{}", ref_id, label.unwrap_or_default()))
            }
            _ => Ok(match value {
                Value::String(text) => text.clone(),
                _ => value.to_string(),
            }),
        }
    }

    fn csv_select_option_maps(
        &self,
        columns: &[AppColumn],
    ) -> Result<HashMap<String, HashMap<String, i64>>, DbError> {
        let mut maps = HashMap::new();
        // CSVには選択肢ラベルが入ることがあるため、ラベル/番号の両方からoption_noを引けるようにします。
        for column in columns
            .iter()
            .filter(|column| column.field_type == "single_select")
        {
            let group_id = column.select_option_group_id.ok_or_else(|| {
                DbError::InvalidInput(format!(
                    "{} is single_select but option group is missing",
                    column.display_name
                ))
            })?;
            let mut stmt = self
                .conn
                .prepare("SELECT option_no, label FROM select_options WHERE group_id = ?")?;
            let options = stmt.query_map([group_id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?;

            let mut option_map = HashMap::new();
            for option in options {
                let (option_no, label) = option?;
                option_map.insert(option_no.to_string(), option_no);
                option_map.insert(label, option_no);
            }
            maps.insert(column.column_name.clone(), option_map);
        }
        Ok(maps)
    }

    fn table_name_by_id(&self, table_id: i64) -> Result<String, DbError> {
        self.conn
            .query_row(
                "SELECT table_name FROM app_tables WHERE id = ?",
                [table_id],
                |row| row.get(0),
            )
            .map_err(DbError::from)
    }

    fn table_reference_sources(&self, table_id: i64) -> Result<Vec<String>, DbError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT t.display_name, t.table_name, c.display_name, c.column_name
            FROM app_table_columns c
            JOIN app_tables t ON t.id = c.table_id
            WHERE c.ref_table_id = ?
            ORDER BY t.sort_order, t.id, c.sort_order, c.id
            ",
        )?;
        let rows = stmt.query_map([table_id], |row| {
            let table_display_name: String = row.get(0)?;
            let table_name: String = row.get(1)?;
            let column_display_name: String = row.get(2)?;
            let column_name: String = row.get(3)?;
            Ok(format!(
                "{} ({}) / {} ({})",
                table_display_name, table_name, column_display_name, column_name
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    fn label_column_name(&self, table_id: i64) -> Result<String, DbError> {
        let label_column_id: Option<i64> = self
            .conn
            .query_row(
                "SELECT label_column_id FROM app_tables WHERE id = ?",
                [table_id],
                |row| row.get(0),
            )
            .optional()?
            .flatten();

        if let Some(column_id) = label_column_id {
            let name: Option<String> = self
                .conn
                .query_row(
                    "SELECT column_name FROM app_table_columns WHERE id = ?",
                    [column_id],
                    |row| row.get(0),
                )
                .optional()?;
            let resolved_name = name.unwrap_or_else(|| "id".into());
            if resolved_name != "id" {
                return Ok(resolved_name);
            }
        } else {
            return self
                .first_non_id_column_name(table_id)
                .map(|name| name.unwrap_or_else(|| "id".into()));
        }

        Ok(self
            .first_non_id_column_name(table_id)?
            .unwrap_or_else(|| "id".into()))
    }

    fn next_sort_order(&self, target: &str, table_id: Option<i64>) -> Result<i64, DbError> {
        match target {
            "app_tables" => self
                .conn
                .query_row("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM app_tables", [], |row| row.get(0))
                .map_err(DbError::from),
            "app_table_columns" => self
                .conn
                .query_row(
                    "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM app_table_columns WHERE table_id = ?",
                    [table_id.ok_or_else(|| DbError::InvalidInput("table_id is required".into()))?],
                    |row| row.get(0),
                )
                .map_err(DbError::from),
            _ => Err(DbError::InvalidInput("unsupported sort target".into())),
        }
    }

    fn next_view_nav_sort_order(&self, parent_id: Option<i64>) -> Result<i64, DbError> {
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

    fn next_view_nav_folder_record_sort_order(&self, folder_id: i64) -> Result<i64, DbError> {
        self.conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM view_nav_folder_records WHERE folder_id = ?",
                [folder_id],
                |row| row.get(0),
            )
            .map_err(DbError::from)
    }

    fn view_nav_folder_record_exists(
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

    fn ensure_view_nav_folder(&self, folder_id: i64) -> Result<(), DbError> {
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

    fn ensure_table_record_exists(&self, table_name: &str, record_id: i64) -> Result<(), DbError> {
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

    fn list_view_layout_templates_for_folder_id(
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

    fn get_view_layout_template(&self, template_id: i64) -> Result<ViewLayoutTemplate, DbError> {
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

    fn list_view_layout_template_cards(
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

    fn list_view_layout_template_card_ids(&self, template_id: i64) -> Result<Vec<i64>, DbError> {
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

    fn ensure_view_layout_template(&self, template_id: i64) -> Result<(), DbError> {
        self.get_view_layout_template(template_id).map(|_| ())
    }

    fn assigned_folder_template_id(&self, folder_id: i64) -> Result<Option<i64>, DbError> {
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

    fn assigned_record_template_id(&self, folder_record_id: i64) -> Result<Option<i64>, DbError> {
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

    fn resolve_record_view_layout_template_id(
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

    fn copy_view_layout_cards(
        &self,
        source_template_id: i64,
        target_template_id: i64,
    ) -> Result<(), DbError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT card_id, x, y, width, height, visible,
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
                    },
                    row.get::<_, i64>(19)?,
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
                  (template_id, table_id, card_id, column_id, updated_at)
                SELECT ?, table_id, ?, column_id, CURRENT_TIMESTAMP
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

    fn insert_view_layout_template_card(
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

    fn get_view_layout_template_card(
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

    fn resolve_view_layout_items(
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

    fn get_view_nav_folder_record(
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

    fn list_view_nav_folder_record_ids(&self, folder_id: i64) -> Result<Vec<i64>, DbError> {
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

    fn list_view_nav_folder_records_for_folder(
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

    fn list_record_tag_groups(&self) -> Result<Vec<RecordTagGroup>, DbError> {
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

    fn list_record_tag_items(&self) -> Result<Vec<RecordTag>, DbError> {
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

    fn get_record_tag_group(&self, group_id: i64) -> Result<RecordTagGroup, DbError> {
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

    fn get_record_tag(&self, tag_id: i64) -> Result<RecordTag, DbError> {
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

    fn find_record_tag_id_by_name(&self, name: &str) -> Result<Option<i64>, DbError> {
        self.conn
            .query_row("SELECT id FROM record_tags WHERE name = ?", [name], |row| {
                row.get(0)
            })
            .optional()
            .map_err(DbError::from)
    }

    fn ensure_record_tag_group(&self, group_id: i64) -> Result<(), DbError> {
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

    fn ensure_record_tag(&self, tag_id: i64) -> Result<(), DbError> {
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

    fn next_record_tag_group_sort_order(&self) -> Result<i64, DbError> {
        self.conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM record_tag_groups",
                [],
                |row| row.get(0),
            )
            .map_err(DbError::from)
    }

    fn next_record_tag_sort_order(&self, group_id: Option<i64>) -> Result<i64, DbError> {
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

    fn resequence_column_sort_order(&self, table_id: i64) -> Result<(), DbError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT id
            FROM app_table_columns
            WHERE table_id = ?
            ORDER BY sort_order, id
            ",
        )?;
        let column_ids = stmt
            .query_map([table_id], |row| row.get::<_, i64>(0))?
            .collect::<Result<Vec<_>, _>>()?;

        for (index, column_id) in column_ids.into_iter().enumerate() {
            self.conn.execute(
                "UPDATE app_table_columns SET sort_order = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                params![index as i64 + 1, column_id],
            )?;
        }

        Ok(())
    }

    fn first_non_id_column_name(&self, table_id: i64) -> Result<Option<String>, DbError> {
        self.conn
            .query_row(
                "
                SELECT column_name
                FROM app_table_columns
                WHERE table_id = ? AND column_name != 'id'
                ORDER BY sort_order, id
                LIMIT 1
                ",
                [table_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(DbError::from)
    }

    fn list_view_table_records(
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

fn validate_identifier(input: &str) -> Result<(), DbError> {
    let starts_ok = input
        .chars()
        .next()
        .map(|ch| ch.is_ascii_alphabetic())
        .unwrap_or(false);
    let chars_ok = input
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
    if !input.is_empty() && starts_ok && chars_ok {
        Ok(())
    } else {
        Err(DbError::InvalidInput(format!(
            "identifier `{}` must start with a letter and use only [A-Za-z0-9_]",
            input
        )))
    }
}

fn validate_field_type(field_type: &str) -> Result<(), DbError> {
    let allowed = [
        "text",
        "integer",
        "real",
        "boolean",
        "date",
        "image",
        "single_select",
        "reference",
    ];
    if allowed.contains(&field_type) {
        Ok(())
    } else {
        Err(DbError::InvalidInput(format!(
            "unsupported field type: {}",
            field_type
        )))
    }
}

fn sqlite_type_for(field_type: &str) -> &'static str {
    match field_type {
        "text" | "image" => "TEXT",
        "real" => "REAL",
        "integer" | "boolean" | "date" | "single_select" | "reference" => "INTEGER",
        _ => "TEXT",
    }
}

fn bool_to_i64(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn override_text(current: Option<String>, template: Option<String>) -> Option<String> {
    if current == template {
        None
    } else {
        current
    }
}

fn override_number(current: Option<f64>, template: Option<f64>) -> Option<f64> {
    match (current, template) {
        (Some(current_value), Some(template_value))
            if (current_value - template_value).abs() <= 0.001 =>
        {
            None
        }
        (current_value, template_value) if current_value == template_value => None,
        (current_value, _) => current_value,
    }
}

fn is_required_value_empty(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => true,
        Some(Value::String(text)) => text.trim().is_empty(),
        _ => false,
    }
}

fn to_sql_value(value: Option<&Value>, field_type: &str) -> Box<dyn ToSql> {
    match field_type {
        "integer" | "date" | "single_select" | "reference" => {
            let parsed = value.and_then(|item| {
                item.as_i64()
                    .or_else(|| item.as_str().and_then(|text| text.parse::<i64>().ok()))
            });
            Box::new(parsed as Option<i64>)
        }
        "real" => {
            let parsed = value.and_then(|item| {
                item.as_f64()
                    .or_else(|| item.as_str().and_then(|text| text.parse::<f64>().ok()))
            });
            Box::new(parsed as Option<f64>)
        }
        "boolean" => {
            let parsed = value.and_then(|item| {
                item.as_bool()
                    .or_else(|| item.as_i64().map(|n| n != 0))
                    .or_else(|| {
                        item.as_str().and_then(|text| match text {
                            "true" | "1" => Some(true),
                            "false" | "0" => Some(false),
                            _ => None,
                        })
                    })
            });
            Box::new(parsed.map(bool_to_i64) as Option<i64>)
        }
        _ => Box::new(value.and_then(Value::as_str).map(|item| item.to_string()) as Option<String>),
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
struct ResolvedExcelColumnMapping {
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

fn resolve_excel_column_mapping(
    columns: &[AppColumn],
    excel_column_names: &[String],
    payload: &[ExcelColumnMappingPayload],
) -> Vec<ResolvedExcelColumnMapping> {
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
                return ResolvedExcelColumnMapping {
                    column: column.clone(),
                    source_column_name: Some(source.clone()),
                    matched_by: Some("manual".into()),
                };
            }
            if let Some(source) = excel_column_names
                .iter()
                .find(|name| **name == column.column_name)
            {
                return ResolvedExcelColumnMapping {
                    column: column.clone(),
                    source_column_name: Some(source.clone()),
                    matched_by: Some("物理名".into()),
                };
            }
            if let Some(source) = excel_column_names
                .iter()
                .find(|name| **name == column.display_name)
            {
                return ResolvedExcelColumnMapping {
                    column: column.clone(),
                    source_column_name: Some(source.clone()),
                    matched_by: Some("論理名".into()),
                };
            }
            ResolvedExcelColumnMapping {
                column: column.clone(),
                source_column_name: None,
                matched_by: None,
            }
        })
        .collect()
}

fn import_column_mapping_suggestions(
    mapping: &[ResolvedExcelColumnMapping],
) -> Vec<ExcelColumnMappingSuggestion> {
    mapping
        .iter()
        .map(|mapping| ExcelColumnMappingSuggestion {
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
    mapping: &[ResolvedExcelColumnMapping],
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
    mapping: &[ResolvedExcelColumnMapping],
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
    mapping: &[ResolvedExcelColumnMapping],
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
    mapping: &[ResolvedExcelColumnMapping],
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
    mapping: &[ResolvedExcelColumnMapping],
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
    mapping: &[ResolvedExcelColumnMapping],
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
    mapping: &[ResolvedExcelColumnMapping],
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
    mapping: &[ResolvedExcelColumnMapping],
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

fn sqlite_value_to_json(value: ValueRef<'_>) -> Result<Value, rusqlite::Error> {
    Ok(match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(v) => Value::from(v),
        ValueRef::Real(v) => Value::from(v),
        ValueRef::Text(v) => Value::from(String::from_utf8_lossy(v).to_string()),
        ValueRef::Blob(_) => Value::Null,
    })
}
