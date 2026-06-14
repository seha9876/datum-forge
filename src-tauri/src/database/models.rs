use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, path::PathBuf};
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
pub(super) struct AppSettingsFile {
    pub(super) db_path: PathBuf,
    #[serde(default)]
    pub(super) show_record_ids_in_navigation: Option<bool>,
    #[serde(default)]
    pub(super) notification_settings: Option<NotificationSettings>,
    #[serde(default)]
    pub(super) last_excel_import_tables: Option<HashMap<i64, String>>,
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
    pub columns: Vec<ViewLayoutCardColumnBinding>,
    pub slots: Vec<ViewLayoutTemplateCardSlot>,
    pub preset_id: Option<String>,
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
    pub auto_height_enabled: bool,
    pub push_down_siblings: bool,
    pub max_auto_height: Option<f64>,
    pub max_auto_height_behavior: String,
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
    pub slots: Vec<ViewLayoutTemplateCardSlot>,
    pub preset_id: Option<String>,
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
    pub auto_height_enabled: bool,
    pub push_down_siblings: bool,
    pub max_auto_height: Option<f64>,
    pub max_auto_height_behavior: String,
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
    pub column_mapping: Vec<ImportColumnMappingPayload>,
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
pub struct ImportColumnMappingPayload {
    pub target_column_name: String,
    pub source_column_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectCsvImportResult {
    pub headers: Vec<String>,
    pub row_count: usize,
    pub column_mappings: Vec<ImportColumnMappingSuggestion>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewCsvImportPayload {
    pub table_id: i64,
    pub input_path: String,
    pub mode: ImportTableCsvMode,
    pub column_mapping: Vec<ImportColumnMappingPayload>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewExcelTableImportPayload {
    pub table_id: i64,
    pub input_path: String,
    pub excel_table_name: String,
    pub mode: ImportTableCsvMode,
    pub column_mapping: Vec<ImportColumnMappingPayload>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportExcelTablePayload {
    pub table_id: i64,
    pub input_path: String,
    pub excel_table_name: String,
    pub mode: ImportTableCsvMode,
    pub column_mapping: Vec<ImportColumnMappingPayload>,
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
pub struct ImportColumnMappingSuggestion {
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
    pub column_mappings: Vec<ImportColumnMappingSuggestion>,
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
    pub column_mappings: Vec<ImportColumnMappingSuggestion>,
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
    pub preset_id: Option<String>,
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
    pub auto_height_enabled: bool,
    pub push_down_siblings: bool,
    pub max_auto_height: Option<f64>,
    pub max_auto_height_behavior: String,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewLayoutTemplateCardSlot {
    pub slot_id: i64,
    pub sort_order: i64,
    pub display_format: Option<String>,
    pub font_size: Option<f64>,
    pub text_color: Option<String>,
    pub font_weight: Option<String>,
    pub text_align: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewLayoutCardColumnBinding {
    pub card_id: i64,
    pub column_id: i64,
    pub sort_order: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewLayoutCardColumnBindingPayload {
    pub card_id: i64,
    pub column_id: i64,
    pub sort_order: i64,
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
    pub(super) conn: Connection,
    pub(super) db_path: PathBuf,
}

fn default_notification_duration_seconds() -> i64 {
    4
}
