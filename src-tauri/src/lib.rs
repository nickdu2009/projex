pub mod app;
mod commands;
pub mod domain;
pub mod error;
pub mod infra;
pub mod sync;

use infra::init_db;
use std::path::PathBuf;
use tauri::Manager;

fn app_data_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("com.nickdu.projex")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let data_dir = app
                .handle()
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| app_data_dir());
            let db_path = data_dir.join("app.db");
            log::info!("DB path: {:?}", db_path);

            let pool = init_db(&db_path).map_err(|e| {
                log::error!("DB init failed: {}", e);
                e
            })?;
            app.manage(pool);

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
            commands::sync::cmd_sync_get_status,
            commands::sync::cmd_sync_full,
            commands::sync::cmd_sync_create_snapshot,
            commands::sync::cmd_sync_restore_snapshot,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
