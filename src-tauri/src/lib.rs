#[cfg(target_os = "android")]
pub mod android_jni;
pub mod app;
mod commands;
pub mod domain;
pub mod error;
pub mod infra;
pub mod sync;
pub use crate::commands::sync::{
    sync_create_snapshot_for_pool, sync_full_for_pool, sync_full_with_runtime_for_pool,
    sync_hold_lock_for_test, sync_restore_snapshot_for_pool, SyncRuntime,
};

use fs2::FileExt;
use infra::init_db;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

const DEFAULT_PROFILE: &str = "default";
const PROFILE_ARG: &str = "--profile";
const PROFILE_ENV: &str = "PROJEX_PROFILE";

pub struct AppRuntimeState {
    profile_name: String,
    data_dir: PathBuf,
    #[allow(dead_code)]
    lock_file: File,
}

impl AppRuntimeState {
    pub fn profile_name(&self) -> &str {
        &self.profile_name
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn log_dir(&self) -> PathBuf {
        self.data_dir.join("logs")
    }
}

fn app_data_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("com.nickdu.projex")
}

fn parse_profile_arg(args: &[String]) -> Option<String> {
    for (index, arg) in args.iter().enumerate() {
        if let Some(value) = arg.strip_prefix(&format!("{PROFILE_ARG}=")) {
            return Some(value.to_string());
        }

        if arg == PROFILE_ARG {
            return Some(args.get(index + 1).cloned().unwrap_or_default());
        }
    }

    None
}

fn normalize_profile_name(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 64 || trimmed.starts_with('-') {
        return None;
    }

    if trimmed == "." || trimmed == ".." {
        return None;
    }

    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }

    Some(trimmed.to_ascii_lowercase())
}

fn resolve_profile_name() -> String {
    let args: Vec<String> = std::env::args().collect();
    if let Some(raw) = parse_profile_arg(&args) {
        if let Some(profile) = normalize_profile_name(&raw) {
            return profile;
        }
        eprintln!(
            "Invalid profile from {PROFILE_ARG}: '{}', fallback to '{DEFAULT_PROFILE}'",
            raw
        );
    }

    if let Ok(raw) = std::env::var(PROFILE_ENV) {
        if let Some(profile) = normalize_profile_name(&raw) {
            return profile;
        }
        eprintln!(
            "Invalid profile from {PROFILE_ENV}: '{}', fallback to '{DEFAULT_PROFILE}'",
            raw
        );
    }

    DEFAULT_PROFILE.to_string()
}

fn resolve_profile_data_dir(base_data_dir: &Path, profile_name: &str) -> PathBuf {
    base_data_dir.join("profiles").join(profile_name)
}

fn resolve_data_dir(app: &tauri::AppHandle, profile_name: &str) -> PathBuf {
    let base_data_dir = app.path().app_data_dir().unwrap_or_else(|_| app_data_dir());
    resolve_profile_data_dir(&base_data_dir, profile_name)
}

fn resolve_log_target_names(profile_name: &str) -> (String, String) {
    (
        format!("webview-{profile_name}"),
        format!("rust-{profile_name}"),
    )
}

fn acquire_profile_lock(data_dir: &Path, profile_name: &str) -> Result<File, String> {
    std::fs::create_dir_all(data_dir).map_err(|e| {
        format!(
            "Failed to create profile data dir for '{}': {}",
            profile_name, e
        )
    })?;

    let lock_path = data_dir.join("app.lock");
    let lock_file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .map_err(|e| format!("Failed to open profile lock file {:?}: {}", lock_path, e))?;

    lock_file.try_lock_exclusive().map_err(|e| {
        format!(
            "Profile '{}' is already in use (lock {:?}): {}",
            profile_name, lock_path, e
        )
    })?;

    Ok(lock_file)
}

fn parse_log_level(level: &str) -> Option<log::LevelFilter> {
    match level.to_uppercase().as_str() {
        "OFF" => Some(log::LevelFilter::Off),
        "ERROR" => Some(log::LevelFilter::Error),
        "WARN" => Some(log::LevelFilter::Warn),
        "INFO" => Some(log::LevelFilter::Info),
        "DEBUG" => Some(log::LevelFilter::Debug),
        "TRACE" => Some(log::LevelFilter::Trace),
        _ => None,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let profile_name = resolve_profile_name();

            // Get data directory early to read log level config
            let data_dir = resolve_data_dir(app.handle(), &profile_name);
            let db_path = data_dir.join("app.db");
            let log_dir = data_dir.join("logs");
            let lock_file =
                acquire_profile_lock(&data_dir, &profile_name).map_err(std::io::Error::other)?;
            std::fs::create_dir_all(&log_dir)?;

            // Determine log level: read from config or use defaults
            let log_level = {
                let default_level = if cfg!(debug_assertions) {
                    log::LevelFilter::Info
                } else {
                    log::LevelFilter::Warn
                };

                // Try to read saved log level from database
                match rusqlite::Connection::open(&db_path) {
                    Ok(conn) => {
                        let saved_level: Result<String, _> = conn.query_row(
                            "SELECT value FROM sync_config WHERE key = 'log_level'",
                            [],
                            |row| row.get(0),
                        );
                        match saved_level {
                            Ok(level_str) => parse_log_level(&level_str).unwrap_or(default_level),
                            Err(_) => default_level,
                        }
                    }
                    Err(_) => default_level,
                }
            };
            let (webview_log_target, rust_log_target) = resolve_log_target_names(&profile_name);

            // Configure log targets:
            // - Webview: for displaying logs in dev console
            // - Folder (webview-<profile>.log): for frontend logs
            // - Folder (rust-<profile>.log): for backend logs
            // 文件轮转策略：单个文件最大 10MB，保留最近 5 个文件。
            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log_level)
                    .max_file_size(10 * 1024 * 1024) // 10 MB per file
                    .targets([
                        Target::new(TargetKind::Webview),
                        Target::new(TargetKind::Folder {
                            path: log_dir.clone(),
                            file_name: Some(webview_log_target),
                        })
                        .filter(|metadata| {
                            metadata
                                .target()
                                .starts_with(tauri_plugin_log::WEBVIEW_TARGET)
                        }),
                        Target::new(TargetKind::Folder {
                            path: log_dir.clone(),
                            file_name: Some(rust_log_target),
                        })
                        .filter(|metadata| {
                            !metadata
                                .target()
                                .starts_with(tauri_plugin_log::WEBVIEW_TARGET)
                        }),
                    ])
                    .build(),
            )?;

            app.manage(AppRuntimeState {
                profile_name: profile_name.clone(),
                data_dir: data_dir.clone(),
                lock_file,
            });

            log::info!("Profile: {}", profile_name);
            log::info!("DB path: {:?}", db_path);
            log::info!("Log dir: {:?}", log_dir);

            let pool = init_db(&db_path).map_err(|e| {
                log::error!("DB init failed: {}", e);
                e
            })?;
            app.manage(pool.clone());

            // Register pool for Android background Worker (JNI path).
            #[cfg(target_os = "android")]
            crate::android_jni::register_pool(pool.clone());

            // Backend auto-sync scheduler (timer lives in Rust).
            let runtime = SyncRuntime::new();
            app.manage(runtime.clone());
            tauri::async_runtime::spawn(async move {
                runtime.refresh_scheduler(pool).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::assignment::cmd_assignment_add_member,
            commands::assignment::cmd_assignment_end_member,
            commands::assignment::cmd_assignment_list_by_project,
            commands::comment::cmd_comment_create,
            commands::comment::cmd_comment_update,
            commands::comment::cmd_comment_delete,
            commands::comment::cmd_comment_list,
            commands::data_transfer::cmd_export_json,
            commands::data_transfer::cmd_import_json,
            commands::logs::cmd_log_list_files,
            commands::logs::cmd_log_tail,
            commands::logs::cmd_log_clear,
            commands::logs::cmd_log_get_level,
            commands::logs::cmd_log_set_level,
            commands::partner::cmd_partner_create,
            commands::partner::cmd_partner_get,
            commands::partner::cmd_partner_list,
            commands::partner::cmd_partner_update,
            commands::partner::cmd_partner_deactivate,
            commands::partner::cmd_partner_projects,
            commands::person::cmd_person_create,
            commands::person::cmd_person_get,
            commands::person::cmd_person_list,
            commands::person::cmd_person_update,
            commands::person::cmd_person_deactivate,
            commands::person::cmd_person_current_projects,
            commands::person::cmd_person_all_projects,
            commands::project::cmd_project_create,
            commands::project::cmd_project_get,
            commands::project::cmd_project_update,
            commands::project::cmd_project_list,
            commands::project::cmd_project_change_status,
            commands::sync::cmd_sync_get_config,
            commands::sync::cmd_sync_update_config,
            commands::sync::cmd_sync_set_enabled,
            commands::sync::cmd_sync_reveal_secret_key,
            commands::sync::cmd_sync_test_connection,
            commands::sync::cmd_sync_get_status,
            commands::sync::cmd_sync_full,
            commands::sync::cmd_sync_create_snapshot,
            commands::sync::cmd_sync_restore_snapshot,
            commands::sync::cmd_sync_export_config,
            commands::sync::cmd_sync_import_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{
        normalize_profile_name, parse_profile_arg, resolve_log_target_names,
        resolve_profile_data_dir,
    };
    use std::path::Path;

    #[test]
    fn parse_profile_from_equals_syntax() {
        let args = vec!["projex".to_string(), "--profile=work".to_string()];
        assert_eq!(parse_profile_arg(&args), Some("work".to_string()));
    }

    #[test]
    fn parse_profile_from_space_syntax() {
        let args = vec![
            "projex".to_string(),
            "--profile".to_string(),
            "work".to_string(),
        ];
        assert_eq!(parse_profile_arg(&args), Some("work".to_string()));
    }

    #[test]
    fn normalize_profile_name_rejects_invalid_characters() {
        assert_eq!(normalize_profile_name("../prod"), None);
        assert_eq!(normalize_profile_name(""), None);
        assert_eq!(normalize_profile_name("-prod"), None);
        assert_eq!(normalize_profile_name("prod*"), None);
    }

    #[test]
    fn normalize_profile_name_normalizes_case() {
        assert_eq!(
            normalize_profile_name("Work_Profile-1"),
            Some("work_profile-1".to_string())
        );
    }

    #[test]
    fn resolve_profile_data_dir_always_nests_profiles() {
        let base = Path::new("/tmp/projex");
        assert_eq!(
            resolve_profile_data_dir(base, "default"),
            base.join("profiles").join("default")
        );
    }

    #[test]
    fn resolve_profile_data_dir_nests_non_default_profiles() {
        let base = Path::new("/tmp/projex");
        assert_eq!(
            resolve_profile_data_dir(base, "work"),
            base.join("profiles").join("work")
        );
    }

    #[test]
    fn resolve_log_target_names_for_profiles() {
        assert_eq!(
            resolve_log_target_names("default"),
            ("webview-default".to_string(), "rust-default".to_string())
        );
        assert_eq!(
            resolve_log_target_names("work"),
            ("webview-work".to_string(), "rust-work".to_string())
        );
    }
}
