use super::*;
use std::{
    env,
    path::{Path, PathBuf},
};

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

pub(super) fn save_settings_with_app_settings(
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

pub(super) fn show_record_ids_in_navigation_setting() -> bool {
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

pub(super) fn notification_settings_setting() -> NotificationSettings {
    load_settings()
        .ok()
        .and_then(|settings| settings.notification_settings)
        .map(normalize_notification_settings)
        .unwrap_or_else(default_notification_settings)
}

pub(super) fn last_excel_import_tables_setting() -> HashMap<i64, String> {
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
}
