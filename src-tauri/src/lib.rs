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

use infra::init_db;
use std::path::PathBuf;
use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

fn app_data_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("com.nickdu.projex")
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
            // Get data directory early to read log level config
            let data_dir = app
                .handle()
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| app_data_dir());
            let db_path = data_dir.join("app.db");

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

            // Configure log targets:
            // - Webview: for displaying logs in dev console
            // - LogDir (webview.log): for frontend logs
            // - LogDir (rust.log): for backend logs
            // 文件轮转策略：单个文件最大 10MB，保留最近 5 个文件。
            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log_level)
                    .max_file_size(10 * 1024 * 1024) // 10 MB per file
                    .targets([
                        Target::new(TargetKind::Webview),
                        Target::new(TargetKind::LogDir {
                            file_name: Some("webview".into()),
                        })
                        .filter(|metadata| {
                            metadata
                                .target()
                                .starts_with(tauri_plugin_log::WEBVIEW_TARGET)
                        }),
                        Target::new(TargetKind::LogDir {
                            file_name: Some("rust".into()),
                        })
                        .filter(|metadata| {
                            !metadata
                                .target()
                                .starts_with(tauri_plugin_log::WEBVIEW_TARGET)
                        }),
                    ])
                    .build(),
            )?;

            log::info!("DB path: {:?}", db_path);

            let pool = init_db(&db_path).map_err(|e| {
                log::error!("DB init failed: {}", e);
                e
            })?;
            app.manage(pool.clone());

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
