import { invoke } from "@tauri-apps/api/core";

import type {
  AppBootstrap,
  AppSettings,
  CreateDatabasePayload,
  StartupDbStatus,
  UpdateNotificationSettingsPayload
} from "../types";

export const settingsApi = {
  getStartupDatabaseStatus: () =>
    invoke<StartupDbStatus>("get_startup_database_status"),
  bootstrap: () => invoke<AppBootstrap>("bootstrap_app"),
  getAppSettings: () => invoke<AppSettings>("get_app_settings"),
  updateRecordIdVisibility: (show: boolean) =>
    invoke<AppSettings>("update_record_id_visibility", { show }),
  updateNotificationSettings: (payload: UpdateNotificationSettingsPayload) =>
    invoke<AppSettings>("update_notification_settings", { payload }),
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
    invoke<AppSettings>("open_database_file", { dbFile })
};
