use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use crate::receive::ProgressEvent;
use crate::state::{AppState, Identity, Peer};
use crate::transport::make_client_endpoint;

#[tauri::command]
pub fn get_identity(state: State<'_, Arc<AppState>>) -> Identity {
    state.identity.lock().unwrap().clone()
}

#[tauri::command]
pub fn list_peers(state: State<'_, Arc<AppState>>) -> Vec<Peer> {
    state.peers.lock().unwrap().values().cloned().collect()
}

#[tauri::command]
pub fn respond_offer(transfer_id: String, accept: bool, state: State<'_, Arc<AppState>>) {
    if let Some(tx) = state.pending.lock().unwrap().remove(&transfer_id) {
        let _ = tx.send(accept);
    }
}

#[tauri::command]
pub fn get_download_dir(state: State<'_, Arc<AppState>>) -> String {
    state.download_dir().to_string_lossy().to_string()
}

#[tauri::command]
pub fn set_download_dir(path: String, state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let dir = PathBuf::from(&path);
    if !dir.is_dir() {
        return Err("Not a folder".to_string());
    }
    state.set_download_dir(dir.clone());
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_download_dir(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let dir = state.download_dir();
    app.opener()
        .open_path(dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn send_to_peer(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    peer_id: String,
    paths: Vec<String>,
) -> Result<String, String> {
    let addr = state
        .peer_addr(&peer_id)
        .ok_or_else(|| "peer not found".to_string())?;
    let socket_addr: std::net::SocketAddr =
        addr.parse().map_err(|e| format!("bad addr: {e}"))?;
    let (from_name, from_peer_id) = {
        let id = state.identity.lock().unwrap();
        (id.display_name.clone(), id.peer_id.clone())
    };
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();

    let transfer_id = format!("out-{}", uuid::Uuid::new_v4());
    let app2 = app.clone();
    let tid = transfer_id.clone();

    tauri::async_runtime::spawn(async move {
        let endpoint = match make_client_endpoint() {
            Ok(e) => e,
            Err(e) => {
                let _ = app2.emit(
                    "transfer-error",
                    serde_json::json!({ "id": tid.clone(), "error": format!("{e}") }),
                );
                return;
            }
        };
        let conn = match endpoint.connect(socket_addr, "filedrop.local") {
            Ok(c) => match c.await {
                Ok(c) => c,
                Err(e) => {
                    let _ = app2.emit(
                    "transfer-error",
                    serde_json::json!({ "id": tid.clone(), "error": format!("{e}") }),
                );
                    return;
                }
            },
            Err(e) => {
                let _ = app2.emit(
                    "transfer-error",
                    serde_json::json!({ "id": tid.clone(), "error": format!("{e}") }),
                );
                return;
            }
        };

        let app3 = app2.clone();
        let tid2 = tid.clone();
        let app_offer = app2.clone();
        let tid_offer = tid.clone();
        let result = crate::send::send_files(
            &conn,
            from_name,
            from_peer_id,
            paths,
            move |files: &[crate::protocol::OfferedFile]| {
                let _ = app_offer.emit(
                    "transfer-files",
                    serde_json::json!({ "transfer_id": tid_offer, "files": files }),
                );
            },
            move |rel_path, bytes, total| {
                let _ = app3.emit(
                    "transfer-progress",
                    &ProgressEvent {
                        transfer_id: tid2.clone(),
                        direction: "outgoing".into(),
                        file_name: rel_path.to_string(),
                        bytes,
                        total,
                    },
                );
            },
        )
        .await;

        match result {
            Ok(o) if !o.accepted => {
                let _ = app2.emit("transfer-declined", &tid);
            }
            Ok(_) => {
                let _ = app2.emit("transfer-done", &tid);
            }
            Err(e) => {
                let _ = app2.emit(
                    "transfer-error",
                    serde_json::json!({ "id": tid.clone(), "error": format!("{e}") }),
                );
            }
        }
    });

    Ok(transfer_id)
}
