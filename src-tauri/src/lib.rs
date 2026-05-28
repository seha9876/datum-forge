//! Tauri バックエンドのアプリケーション境界です。
//!
//! このモジュールは共有データベースハンドルを保持し、フロントエンドから
//! 呼び出す IPC command を公開します。

mod database;

use std::{path::Path, process::Command, sync::Mutex};

use database::{
    AddColumnPayload, AddViewNavFolderRecordsPayload, AppBootstrap, AppSettings,
    AssignViewLayoutFolderTemplatePayload, AttachRecordTagPayload, CreateAndAttachRecordTagPayload,
    CreateDatabasePayload, CreateTablePayload, CreateViewLayoutTemplatePayload,
    CreateViewNavFolderPayload, Db, DeleteColumnPayload, DeleteRecordPayload,
    DeleteRecordTagGroupPayload, DeleteRecordTagPayload, DeleteTablePayload,
    DeleteViewLayoutTemplatePayload, DeleteViewNavFolderPayload, DetachRecordTagPayload,
    DuplicateViewLayoutTemplatePayload, FolderViewLayoutTemplates,
    GetResolvedViewFieldLayoutPayload, GetViewLayoutTemplateCardsPayload,
    ListViewLayoutCardColumnBindingsPayload, ListViewLayoutTemplatesForFolderPayload, RecordTag,
    RecordTagBundle, RecordTagGroup, RecordTagGroupLinkPayload, ReferenceChoice,
    RemoveViewNavFolderRecordPayload, RenameViewLayoutTemplatePayload, ReorderColumnsPayload,
    ReorderViewNavFolderRecordsPayload, ResetViewLayoutCardOverridePayload,
    ResetViewLayoutCardOverridesPayload, ResolvedViewFieldLayout, SaveOptionGroupPayload,
    SaveRecordPayload, SaveRecordTagGroupPayload, SaveRecordTagPayload,
    SaveViewLayoutCardColumnBindingsPayload, SaveViewLayoutCardOverridesPayload,
    SaveViewLayoutTemplateCardsPayload, StartupDbStatus, TableDetail, UpdateColumnPayload,
    UpdateLabelColumnPayload, ViewLayoutCardColumnBinding, ViewLayoutTemplate,
    ViewLayoutTemplateCard, ViewNavFolderRecord, ViewNavNode, ViewTableSection,
};
use tauri::State;

/// Tauri command handler に注入される共有アプリケーション状態です。
/// DB 接続は mutex で保護し、各 command が同じ接続を使います。
struct AppState {
    db: Mutex<Option<Db>>,
}

fn db_not_ready_error() -> String {
    "DBセットアップが完了していません。".to_string()
}

#[tauri::command]
fn bootstrap_app(state: State<'_, AppState>) -> Result<AppBootstrap, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .bootstrap()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_startup_database_status(state: State<'_, AppState>) -> Result<StartupDbStatus, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    Db::startup_status(db.as_ref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_app_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .settings())
}

#[tauri::command]
fn update_record_id_visibility(
    state: State<'_, AppState>,
    show: bool,
) -> Result<AppSettings, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_mut()
        .ok_or_else(db_not_ready_error)?
        .update_record_id_visibility(show)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_database_file(
    state: State<'_, AppState>,
    payload: CreateDatabasePayload,
) -> Result<AppSettings, String> {
    let db = Db::create_database(payload).map_err(|e| e.to_string())?;
    let settings = db.settings();
    *state.db.lock().map_err(|e| e.to_string())? = Some(db);
    Ok(settings)
}

#[tauri::command]
fn setup_open_database_file(
    state: State<'_, AppState>,
    db_file: String,
) -> Result<AppSettings, String> {
    let db = Db::open_existing_database(db_file).map_err(|e| e.to_string())?;
    let settings = db.settings();
    *state.db.lock().map_err(|e| e.to_string())? = Some(db);
    Ok(settings)
}

#[tauri::command]
fn open_path_folder(path: String) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("開くフォルダーのパスが空です。".to_string());
    }

    let target = Path::new(trimmed);
    let folder = if target.is_dir() {
        target
    } else {
        target
            .parent()
            .ok_or_else(|| "親フォルダーを取得できません。".to_string())?
    };

    if !folder.exists() {
        return Err("対象のフォルダーが見つかりません。".to_string());
    }

    #[cfg(target_os = "windows")]
    let result = Command::new("explorer.exe").arg(folder).spawn();

    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(folder).spawn();

    #[cfg(all(unix, not(target_os = "macos")))]
    let result = Command::new("xdg-open").arg(folder).spawn();

    result
        .map(|_| ())
        .map_err(|error| format!("フォルダーを開けませんでした: {error}"))
}

#[tauri::command]
fn update_database_directory(
    state: State<'_, AppState>,
    db_directory: String,
) -> Result<AppSettings, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_mut()
        .ok_or_else(db_not_ready_error)?
        .update_db_directory(db_directory)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn rename_database_file(
    state: State<'_, AppState>,
    db_file_name: String,
) -> Result<AppSettings, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_mut()
        .ok_or_else(db_not_ready_error)?
        .rename_db_file(db_file_name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn open_database_file(state: State<'_, AppState>, db_file: String) -> Result<AppSettings, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_mut()
        .ok_or_else(db_not_ready_error)?
        .open_db_file(db_file)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_table(state: State<'_, AppState>, payload: CreateTablePayload) -> Result<i64, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .create_table(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_table(state: State<'_, AppState>, payload: DeleteTablePayload) -> Result<(), String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .delete_table(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn add_column(state: State<'_, AppState>, payload: AddColumnPayload) -> Result<(), String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .add_column(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_column(state: State<'_, AppState>, payload: DeleteColumnPayload) -> Result<(), String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .delete_column(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn update_column(state: State<'_, AppState>, payload: UpdateColumnPayload) -> Result<(), String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .update_column(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn update_label_column(
    state: State<'_, AppState>,
    payload: UpdateLabelColumnPayload,
) -> Result<(), String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .update_label_column(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn reorder_columns(
    state: State<'_, AppState>,
    payload: ReorderColumnsPayload,
) -> Result<(), String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .reorder_columns(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn save_option_group(
    state: State<'_, AppState>,
    payload: SaveOptionGroupPayload,
) -> Result<i64, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .save_option_group(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_table_detail(state: State<'_, AppState>, table_id: i64) -> Result<TableDetail, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .get_table_detail(table_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn save_record(state: State<'_, AppState>, payload: SaveRecordPayload) -> Result<(), String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .save_record(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_record(state: State<'_, AppState>, payload: DeleteRecordPayload) -> Result<(), String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .delete_record(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_reference_choices(
    state: State<'_, AppState>,
    table_id: i64,
) -> Result<Vec<ReferenceChoice>, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .get_reference_choices(table_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_view_nav_nodes(state: State<'_, AppState>) -> Result<Vec<ViewNavNode>, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .list_view_nav_nodes()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_view_nav_folder(
    state: State<'_, AppState>,
    payload: CreateViewNavFolderPayload,
) -> Result<ViewNavNode, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .create_view_nav_folder(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_view_nav_folder(
    state: State<'_, AppState>,
    payload: DeleteViewNavFolderPayload,
) -> Result<(), String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .delete_view_nav_folder(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_view_nav_folder_records(
    state: State<'_, AppState>,
) -> Result<Vec<ViewNavFolderRecord>, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .list_view_nav_folder_records()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn add_view_nav_folder_records(
    state: State<'_, AppState>,
    payload: AddViewNavFolderRecordsPayload,
) -> Result<Vec<ViewNavFolderRecord>, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .add_view_nav_folder_records(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_view_nav_folder_record(
    state: State<'_, AppState>,
    payload: RemoveViewNavFolderRecordPayload,
) -> Result<(), String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .remove_view_nav_folder_record(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn reorder_view_nav_folder_records(
    state: State<'_, AppState>,
    payload: ReorderViewNavFolderRecordsPayload,
) -> Result<Vec<ViewNavFolderRecord>, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .reorder_view_nav_folder_records(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_view_table_sections(state: State<'_, AppState>) -> Result<Vec<ViewTableSection>, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .get_view_table_sections()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_all_folder_layout_templates(
    state: State<'_, AppState>,
) -> Result<Vec<ViewLayoutTemplate>, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .list_all_folder_layout_templates()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_view_layout_templates_for_folder(
    state: State<'_, AppState>,
    payload: ListViewLayoutTemplatesForFolderPayload,
) -> Result<FolderViewLayoutTemplates, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .list_view_layout_templates_for_folder(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_view_layout_template(
    state: State<'_, AppState>,
    payload: CreateViewLayoutTemplatePayload,
) -> Result<ViewLayoutTemplate, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .create_view_layout_template(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn rename_view_layout_template(
    state: State<'_, AppState>,
    payload: RenameViewLayoutTemplatePayload,
) -> Result<ViewLayoutTemplate, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .rename_view_layout_template(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn duplicate_view_layout_template(
    state: State<'_, AppState>,
    payload: DuplicateViewLayoutTemplatePayload,
) -> Result<ViewLayoutTemplate, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .duplicate_view_layout_template(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_view_layout_template(
    state: State<'_, AppState>,
    payload: DeleteViewLayoutTemplatePayload,
) -> Result<(), String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .delete_view_layout_template(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn assign_view_layout_folder_template(
    state: State<'_, AppState>,
    payload: AssignViewLayoutFolderTemplatePayload,
) -> Result<ViewLayoutTemplate, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .assign_view_layout_folder_template(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_resolved_view_field_layout(
    state: State<'_, AppState>,
    payload: GetResolvedViewFieldLayoutPayload,
) -> Result<ResolvedViewFieldLayout, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .get_resolved_view_field_layout(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_view_layout_template_cards(
    state: State<'_, AppState>,
    payload: GetViewLayoutTemplateCardsPayload,
) -> Result<Vec<ViewLayoutTemplateCard>, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .get_view_layout_template_cards(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_view_layout_card_column_bindings(
    state: State<'_, AppState>,
    payload: ListViewLayoutCardColumnBindingsPayload,
) -> Result<Vec<ViewLayoutCardColumnBinding>, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .list_view_layout_card_column_bindings(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn save_view_layout_template_cards(
    state: State<'_, AppState>,
    payload: SaveViewLayoutTemplateCardsPayload,
) -> Result<(), String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .save_view_layout_template_cards(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn save_view_layout_card_column_bindings(
    state: State<'_, AppState>,
    payload: SaveViewLayoutCardColumnBindingsPayload,
) -> Result<(), String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .save_view_layout_card_column_bindings(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn save_view_layout_card_overrides(
    state: State<'_, AppState>,
    payload: SaveViewLayoutCardOverridesPayload,
) -> Result<(), String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .save_view_layout_card_overrides(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn reset_view_layout_card_override(
    state: State<'_, AppState>,
    payload: ResetViewLayoutCardOverridePayload,
) -> Result<(), String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .reset_view_layout_card_override(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn reset_view_layout_card_overrides(
    state: State<'_, AppState>,
    payload: ResetViewLayoutCardOverridesPayload,
) -> Result<(), String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .reset_view_layout_card_overrides(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_record_tags(state: State<'_, AppState>) -> Result<RecordTagBundle, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .list_record_tags()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_record_tags_for_record(
    state: State<'_, AppState>,
    table_id: i64,
    record_id: i64,
) -> Result<Vec<RecordTag>, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .list_record_tags_for_record(table_id, record_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn save_record_tag_group(
    state: State<'_, AppState>,
    payload: SaveRecordTagGroupPayload,
) -> Result<RecordTagGroup, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .save_record_tag_group(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_record_tag_group(
    state: State<'_, AppState>,
    payload: DeleteRecordTagGroupPayload,
) -> Result<(), String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .delete_record_tag_group(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn save_record_tag(
    state: State<'_, AppState>,
    payload: SaveRecordTagPayload,
) -> Result<RecordTag, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .save_record_tag(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_record_tag(
    state: State<'_, AppState>,
    payload: DeleteRecordTagPayload,
) -> Result<(), String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .delete_record_tag(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn attach_record_tag_group(
    state: State<'_, AppState>,
    payload: RecordTagGroupLinkPayload,
) -> Result<RecordTag, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .attach_record_tag_group(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn detach_record_tag_group(
    state: State<'_, AppState>,
    payload: RecordTagGroupLinkPayload,
) -> Result<RecordTag, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .detach_record_tag_group(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn attach_record_tag(
    state: State<'_, AppState>,
    payload: AttachRecordTagPayload,
) -> Result<RecordTag, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .attach_record_tag(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_and_attach_record_tag(
    state: State<'_, AppState>,
    payload: CreateAndAttachRecordTagPayload,
) -> Result<RecordTag, String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .create_and_attach_record_tag(payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn detach_record_tag(
    state: State<'_, AppState>,
    payload: DetachRecordTagPayload,
) -> Result<(), String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(db_not_ready_error)?
        .detach_record_tag(payload)
        .map_err(|e| e.to_string())
}

pub fn run() {
    let db = match Db::open_configured() {
        Ok(db) => db,
        Err(error) => {
            eprintln!("failed to initialize configured sqlite: {error}");
            None
        }
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState { db: Mutex::new(db) })
        .invoke_handler(tauri::generate_handler![
            get_startup_database_status,
            bootstrap_app,
            get_app_settings,
            update_record_id_visibility,
            create_database_file,
            setup_open_database_file,
            open_path_folder,
            update_database_directory,
            rename_database_file,
            open_database_file,
            create_table,
            delete_table,
            add_column,
            delete_column,
            update_column,
            update_label_column,
            reorder_columns,
            save_option_group,
            get_table_detail,
            save_record,
            delete_record,
            get_reference_choices,
            list_view_nav_nodes,
            create_view_nav_folder,
            delete_view_nav_folder,
            list_view_nav_folder_records,
            add_view_nav_folder_records,
            remove_view_nav_folder_record,
            reorder_view_nav_folder_records,
            get_view_table_sections,
            list_all_folder_layout_templates,
            list_view_layout_templates_for_folder,
            create_view_layout_template,
            rename_view_layout_template,
            duplicate_view_layout_template,
            delete_view_layout_template,
            assign_view_layout_folder_template,
            get_resolved_view_field_layout,
            get_view_layout_template_cards,
            list_view_layout_card_column_bindings,
            save_view_layout_template_cards,
            save_view_layout_card_column_bindings,
            save_view_layout_card_overrides,
            reset_view_layout_card_override,
            reset_view_layout_card_overrides,
            list_record_tags,
            list_record_tags_for_record,
            save_record_tag_group,
            delete_record_tag_group,
            save_record_tag,
            delete_record_tag,
            attach_record_tag_group,
            detach_record_tag_group,
            attach_record_tag,
            create_and_attach_record_tag,
            detach_record_tag
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
