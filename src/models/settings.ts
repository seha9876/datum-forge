export interface AppSettings {
  dbPath: string;
  showRecordIdsInNavigation: boolean;
  notificationSettings: NotificationSettings;
  lastExcelImportTables: Record<number, string>;
}

export interface NotificationSettings {
  usePerKindDurations: boolean;
  commonDurationSeconds: number;
  successDurationSeconds: number;
  warningDurationSeconds: number;
  errorDurationSeconds: number;
}

export interface UpdateNotificationSettingsPayload {
  notificationSettings: NotificationSettings;
}

export type StartupDbState = "ready" | "firstLaunch" | "missingDb" | "error";

export interface StartupDbStatus {
  state: StartupDbState;
  dbPath: string | null;
  defaultDbDirectory: string;
  defaultDbFileName: string;
  missingDbPath: string | null;
  message: string | null;
}

export interface CreateDatabasePayload {
  dbDirectory: string;
  dbFileName: string;
}
