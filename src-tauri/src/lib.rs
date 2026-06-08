pub mod fileutil;
pub mod hash;
pub mod protocol;
pub mod protocol_io;
pub mod receive;
pub mod send;
pub mod transport;

mod commands;
mod discovery;
mod state;

use std::sync::Arc;

use state::{AppState, Identity};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    transport::ensure_crypto_provider();

    let identity = Identity::generate();
    let app_state = Arc::new(AppState::new(identity));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(app_state.clone())
        .invoke_handler(tauri::generate_handler![
            commands::get_identity,
            commands::list_peers,
            commands::respond_offer,
            commands::send_to_peer,
            commands::get_download_dir,
            commands::set_download_dir,
            commands::open_download_dir,
        ])
        .setup(move |app| {
            use tauri::Manager;
            let handle = app.handle().clone();

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_icon(tauri::include_image!("icons/icon.png"));
            }

            if let Ok(cfg_dir) = app.path().app_config_dir() {
                let _ = std::fs::create_dir_all(&cfg_dir);
                app_state.load_config(cfg_dir.join("config.json"));
            }

            let endpoint = tauri::async_runtime::block_on(async {
                transport::make_server_endpoint()
            })
            .expect("failed to start QUIC server");
            let port = endpoint.local_addr().expect("no local addr").port();
            *app_state.server_port.lock().unwrap() = port;

            let srv_handle = handle.clone();
            let srv_state = app_state.clone();
            tauri::async_runtime::spawn(async move {
                receive::run_server(endpoint, srv_handle, srv_state).await;
            });

            discovery::start(handle, app_state.clone(), port)
                .expect("failed to start discovery");

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
