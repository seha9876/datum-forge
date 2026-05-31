import { invoke } from "@tauri-apps/api/core";

import type {
  AddColumnPayload,
  AddViewNavFolderRecordsPayload,
  AppBootstrap,
  AppSettings,
  AssignViewLayoutFolderTemplatePayload,
  AssignViewLayoutRecordTemplatePayload,
  ClearViewLayoutRecordTemplatePayload,
  CreateDatabasePayload,
  CreateTablePayload,
  AttachRecordTagPayload,
  CreateAndAttachRecordTagPayload,
  CreateViewLayoutTemplatePayload,
  DeleteRecordTagPayload,
  DeleteColumnPayload,
  DeleteRecordPayload,
  DeleteTablePayload,
  DeleteViewLayoutTemplatePayload,
  DeleteViewNavFolderPayload,
  DetachRecordTagPayload,
  DuplicateViewLayoutTemplatePayload,
  ExportTableCsvPayload,
  FolderViewLayoutTemplates,
  GetViewLayoutTemplateCardsPayload,
  GetResolvedViewFieldLayoutPayload,
  ImportTableCsvPayload,
  ImportTableCsvResult,
  ListViewLayoutCardColumnBindingsPayload,
  ListViewLayoutTemplatesForFolderPayload,
  ReferenceChoice,
  RecordTag,
  RecordTagBundle,
  RecordTagGroup,
  RecordTagGroupLinkPayload,
  ReorderViewNavFolderRecordsPayload,
  RemoveViewNavFolderRecordPayload,
  RenameViewLayoutTemplatePayload,
  ReorderColumnsPayload,
  ResetViewLayoutCardOverridePayload,
  ResetViewLayoutCardOverridesPayload,
  ResolvedViewFieldLayout,
  SaveOptionGroupPayload,
  SaveRecordTagGroupPayload,
  SaveRecordTagPayload,
  SaveRecordPayload,
  SaveViewLayoutCardColumnBindingsPayload,
  SaveViewLayoutCardOverridesPayload,
  SaveViewLayoutTemplateCardsPayload,
  StartupDbStatus,
  TableDetail,
  CreateViewNavFolderPayload,
  UpdateLabelColumnPayload,
  UpdateColumnPayload,
  ViewLayoutTemplate,
  ViewLayoutCardColumnBinding,
  ViewLayoutTemplateCard,
  ViewNavFolderRecord,
  ViewNavNode,
  ViewTableSection
} from "./types";

export const api = {
  getStartupDatabaseStatus: () =>
    invoke<StartupDbStatus>("get_startup_database_status"),
  /**
   * アプリ起動時に必要なテーブル一覧と選択肢グループを取得します。
   *
   * @returns 初期表示に必要なブートストラップ情報
   */
  bootstrap: () => invoke<AppBootstrap>("bootstrap_app"),
  getAppSettings: () => invoke<AppSettings>("get_app_settings"),
  updateRecordIdVisibility: (show: boolean) =>
    invoke<AppSettings>("update_record_id_visibility", { show }),
  createDatabaseFile: (payload: CreateDatabasePayload) =>
    invoke<AppSettings>("create_database_file", { payload }),
  setupOpenDatabaseFile: (dbFile: string) =>
    invoke<AppSettings>("setup_open_database_file", { dbFile }),
  openPathFolder: (path: string) => invoke<void>("open_path_folder", { path }),
  updateDatabaseDirectory: (dbDirectory: string) =>
    invoke<AppSettings>("update_database_directory", { dbDirectory }),
  renameDatabaseFile: (dbFileName: string) =>
    invoke<AppSettings>("rename_database_file", { dbFileName }),
  openDatabaseFile: (dbFile: string) =>
    invoke<AppSettings>("open_database_file", { dbFile }),
  /**
   * 指定したテーブルの定義とレコードを取得します。
   *
   * @param tableId 詳細を読み込むテーブルID
   * @returns テーブル詳細
   */
  getTableDetail: (tableId: number) =>
    invoke<TableDetail>("get_table_detail", { tableId }),
  /**
   * 新しいテーブルを作成します。
   *
   * @param payload 作成するテーブル情報
   * @returns 作成されたテーブルID
   */
  createTable: (payload: CreateTablePayload) =>
    invoke<number>("create_table", { payload }),
  deleteTable: (payload: DeleteTablePayload) =>
    invoke<void>("delete_table", { payload }),
  /** 指定テーブルの表示値をCSVとして保存します。 */
  exportTableCsv: (payload: ExportTableCsvPayload) =>
    invoke<void>("export_table_csv", { payload }),
  /** 選択されたCSVを指定方式で対象テーブルへ取り込みます。 */
  importTableCsv: (payload: ImportTableCsvPayload) =>
    invoke<ImportTableCsvResult>("import_table_csv", { payload }),
  /**
   * テーブルに新しいカラムを追加します。
   *
   * @param payload 追加するカラム情報
   * @returns 完了を表す Promise
   */
  addColumn: (payload: AddColumnPayload) =>
    invoke<void>("add_column", { payload }),
  /**
   * テーブルから既存カラムを削除します。
   *
   * @param payload 削除対象のテーブルIDとカラムID
   * @returns 完了を表す Promise
   */
  deleteColumn: (payload: DeleteColumnPayload) =>
    invoke<void>("delete_column", { payload }),
  /**
   * 既存カラムの表示名と物理名を更新します。
   *
   * @param payload 更新内容
   * @returns 完了を表す Promise
   */
  updateColumn: (payload: UpdateColumnPayload) =>
    invoke<void>("update_column", { payload }),
  /**
   * テーブルの主表示カラムを更新します。
   *
   * @param payload 主表示カラム設定
   * @returns 完了を表す Promise
   */
  updateLabelColumn: (payload: UpdateLabelColumnPayload) =>
    invoke<void>("update_label_column", { payload }),
  /**
   * カラムの表示順を保存します。
   *
   * @param payload 並び替え後のカラムID一覧
   * @returns 完了を表す Promise
   */
  reorderColumns: (payload: ReorderColumnsPayload) =>
    invoke<void>("reorder_columns", { payload }),
  /**
   * 単一選択グループを新規作成または更新します。
   *
   * @param payload 保存対象のグループ情報
   * @returns 保存されたグループID
   */
  saveOptionGroup: (payload: SaveOptionGroupPayload) =>
    invoke<number>("save_option_group", { payload }),
  /**
   * レコードを新規保存または更新します。
   *
   * @param payload 保存するレコード内容
   * @returns 完了を表す Promise
   */
  saveRecord: (payload: SaveRecordPayload) =>
    invoke<void>("save_record", { payload }),
  /**
   * 指定テーブルから既存レコードを削除します。
   *
   * @param payload 削除対象のテーブルIDとレコードID
   * @returns 完了を表す Promise
   */
  deleteRecord: (payload: DeleteRecordPayload) =>
    invoke<void>("delete_record", { payload }),
  /**
   * 参照型カラム向けの候補一覧を取得します。
   *
   * @param tableId 参照先テーブルID
   * @returns 参照候補一覧
   */
  getReferenceChoices: (tableId: number) =>
    invoke<ReferenceChoice[]>("get_reference_choices", { tableId }),
  listViewNavNodes: () => invoke<ViewNavNode[]>("list_view_nav_nodes"),
  createViewNavFolder: (payload: CreateViewNavFolderPayload) =>
    invoke<ViewNavNode>("create_view_nav_folder", { payload }),
  deleteViewNavFolder: (payload: DeleteViewNavFolderPayload) =>
    invoke<void>("delete_view_nav_folder", { payload }),
  listViewNavFolderRecords: () =>
    invoke<ViewNavFolderRecord[]>("list_view_nav_folder_records"),
  addViewNavFolderRecords: (payload: AddViewNavFolderRecordsPayload) =>
    invoke<ViewNavFolderRecord[]>("add_view_nav_folder_records", { payload }),
  removeViewNavFolderRecord: (payload: RemoveViewNavFolderRecordPayload) =>
    invoke<void>("remove_view_nav_folder_record", { payload }),
  /**
   * 閲覧目次で、指定フォルダー内に登録されたレコードの表示順を保存します。
   *
   * @param payload フォルダーIDと並び替え後のフォルダー内レコードID一覧
   * @returns 保存後のフォルダー内レコード一覧
   */
  reorderViewNavFolderRecords: (payload: ReorderViewNavFolderRecordsPayload) =>
    invoke<ViewNavFolderRecord[]>("reorder_view_nav_folder_records", {
      payload
    }),
  getViewTableSections: () =>
    invoke<ViewTableSection[]>("get_view_table_sections"),
  listAllFolderLayoutTemplates: () =>
    invoke<ViewLayoutTemplate[]>("list_all_folder_layout_templates"),
  listViewLayoutTemplatesForFolder: (
    payload: ListViewLayoutTemplatesForFolderPayload
  ) =>
    invoke<FolderViewLayoutTemplates>("list_view_layout_templates_for_folder", {
      payload
    }),
  createViewLayoutTemplate: (payload: CreateViewLayoutTemplatePayload) =>
    invoke<ViewLayoutTemplate>("create_view_layout_template", { payload }),
  renameViewLayoutTemplate: (payload: RenameViewLayoutTemplatePayload) =>
    invoke<ViewLayoutTemplate>("rename_view_layout_template", { payload }),
  duplicateViewLayoutTemplate: (payload: DuplicateViewLayoutTemplatePayload) =>
    invoke<ViewLayoutTemplate>("duplicate_view_layout_template", { payload }),
  deleteViewLayoutTemplate: (payload: DeleteViewLayoutTemplatePayload) =>
    invoke<void>("delete_view_layout_template", { payload }),
  assignViewLayoutFolderTemplate: (
    payload: AssignViewLayoutFolderTemplatePayload
  ) =>
    invoke<ViewLayoutTemplate>("assign_view_layout_folder_template", {
      payload
    }),
  assignViewLayoutRecordTemplate: (
    payload: AssignViewLayoutRecordTemplatePayload
  ) =>
    invoke<ViewNavFolderRecord>("assign_view_layout_record_template", {
      payload
    }),
  clearViewLayoutRecordTemplate: (
    payload: ClearViewLayoutRecordTemplatePayload
  ) =>
    invoke<ViewNavFolderRecord>("clear_view_layout_record_template", {
      payload
    }),
  getResolvedViewFieldLayout: (payload: GetResolvedViewFieldLayoutPayload) =>
    invoke<ResolvedViewFieldLayout>("get_resolved_view_field_layout", {
      payload
    }),
  getViewLayoutTemplateCards: (payload: GetViewLayoutTemplateCardsPayload) =>
    invoke<ViewLayoutTemplateCard[]>("get_view_layout_template_cards", {
      payload
    }),
  listViewLayoutCardColumnBindings: (
    payload: ListViewLayoutCardColumnBindingsPayload
  ) =>
    invoke<ViewLayoutCardColumnBinding[]>(
      "list_view_layout_card_column_bindings",
      {
        payload
      }
    ),
  saveViewLayoutTemplateCards: (payload: SaveViewLayoutTemplateCardsPayload) =>
    invoke<void>("save_view_layout_template_cards", { payload }),
  saveViewLayoutCardColumnBindings: (
    payload: SaveViewLayoutCardColumnBindingsPayload
  ) => invoke<void>("save_view_layout_card_column_bindings", { payload }),
  saveViewLayoutCardOverrides: (payload: SaveViewLayoutCardOverridesPayload) =>
    invoke<void>("save_view_layout_card_overrides", { payload }),
  resetViewLayoutCardOverride: (payload: ResetViewLayoutCardOverridePayload) =>
    invoke<void>("reset_view_layout_card_override", { payload }),
  resetViewLayoutCardOverrides: (
    payload: ResetViewLayoutCardOverridesPayload
  ) => invoke<void>("reset_view_layout_card_overrides", { payload }),
  listRecordTags: () => invoke<RecordTagBundle>("list_record_tags"),
  listRecordTagsForRecord: (tableId: number, recordId: number) =>
    invoke<RecordTag[]>("list_record_tags_for_record", { tableId, recordId }),
  saveRecordTagGroup: (payload: SaveRecordTagGroupPayload) =>
    invoke<RecordTagGroup>("save_record_tag_group", { payload }),
  saveRecordTag: (payload: SaveRecordTagPayload) =>
    invoke<RecordTag>("save_record_tag", { payload }),
  deleteRecordTag: (payload: DeleteRecordTagPayload) =>
    invoke<void>("delete_record_tag", { payload }),
  attachRecordTagGroup: (payload: RecordTagGroupLinkPayload) =>
    invoke<RecordTag>("attach_record_tag_group", { payload }),
  detachRecordTagGroup: (payload: RecordTagGroupLinkPayload) =>
    invoke<RecordTag>("detach_record_tag_group", { payload }),
  attachRecordTag: (payload: AttachRecordTagPayload) =>
    invoke<RecordTag>("attach_record_tag", { payload }),
  createAndAttachRecordTag: (payload: CreateAndAttachRecordTagPayload) =>
    invoke<RecordTag>("create_and_attach_record_tag", { payload }),
  detachRecordTag: (payload: DetachRecordTagPayload) =>
    invoke<void>("detach_record_tag", { payload })
};
