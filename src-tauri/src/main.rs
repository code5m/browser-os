#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod domain;
mod bridge;
mod workspace;
mod keyring_store;
mod sync;

use bridge::AppState;

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            bridge::open_browser,
            bridge::collect_selection,
            bridge::list_artifacts,
            bridge::configure_repo,
            bridge::list_repos,
            bridge::request_sync,
            bridge::confirm_sync,
            bridge::audit_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
