use std::future::Future;
use std::path::{Path, PathBuf};

use quinn::Connection;
use tokio::io::AsyncWriteExt;

use crate::fileutil::safe_dest_path;
use crate::hash::Hasher;
use crate::protocol::{AcceptDecision, FileAck, FileHeader, Offer};
use crate::protocol_io::{read_msg, write_msg};

const CHUNK: usize = 64 * 1024;
const MAX_CTRL_TAIL: usize = 1024;

#[derive(Debug)]
pub struct ReceiveOutcome {
    pub accepted: bool,
    pub saved: Vec<PathBuf>,
}

pub async fn receive_transfer<D, Fut, P>(
    conn: &Connection,
    dest_dir: &Path,
    decide: D,
    mut on_progress: P,
) -> Result<ReceiveOutcome, Box<dyn std::error::Error + Send + Sync>>
where
    D: FnOnce(Offer) -> Fut,
    Fut: Future<Output = bool>,
    P: FnMut(&str, u64, u64),
{
    let (mut ctrl_send, mut ctrl_recv) = conn.accept_bi().await?;
    let offer: Offer = read_msg(&mut ctrl_recv).await?;
    let expected = offer.files.len();

    let accepted = decide(offer).await;
    write_msg(&mut ctrl_send, &AcceptDecision { ok: accepted }).await?;
    if !accepted {
        let _ = ctrl_recv.read_to_end(MAX_CTRL_TAIL).await;
        return Ok(ReceiveOutcome { accepted: false, saved: Vec::new() });
    }

    tokio::fs::create_dir_all(dest_dir).await.ok();
    let mut saved = Vec::new();

    for _ in 0..expected {
        let mut body = conn.accept_uni().await?;
        let header: FileHeader = read_msg(&mut body).await?;
        let display_name = header.rel_path.clone();
        let dest = safe_dest_path(dest_dir, &header.rel_path);
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }

        // If we can't create the destination (e.g. path too long), we must
        // still read the incoming stream to keep the protocol in sync, then
        // report this one file as failed and move on.
        let mut file = tokio::fs::File::create(&dest).await.ok();
        let mut hasher = Hasher::new();
        let mut buf = vec![0u8; CHUNK];
        let mut received: u64 = 0;
        let mut read_ok = true;
        let mut err: Option<String> = if file.is_none() {
            Some("couldn't create file on this device".into())
        } else {
            None
        };
        loop {
            let n = match body.read(&mut buf).await {
                Ok(Some(n)) => n,
                Ok(None) => break,
                Err(_) => {
                    read_ok = false;
                    err = Some("transfer interrupted".into());
                    break;
                }
            };
            hasher.update(&buf[..n]);
            received += n as u64;
            if let Some(f) = file.as_mut() {
                if f.write_all(&buf[..n]).await.is_err() {
                    err = Some("couldn't write to disk".into());
                    file = None; // stop writing but keep draining the stream
                }
            }
            on_progress(&display_name, received, header.size);
        }
        if let Some(f) = file.as_mut() {
            let _ = f.flush().await;
        }

        let digest = hasher.finalize_hex();
        let integrity = digest == header.blake3_hex && received == header.size;
        let ok = read_ok && err.is_none() && integrity;
        if ok {
            saved.push(dest.clone());
        } else {
            tokio::fs::remove_file(&dest).await.ok();
            if err.is_none() {
                err = Some("integrity check failed".into());
            }
        }
        let ack = FileAck {
            name: display_name,
            ok,
            error: if ok { None } else { err },
        };
        write_msg(&mut ctrl_send, &ack).await?;
    }

    let _ = ctrl_recv.read_to_end(MAX_CTRL_TAIL).await;
    ctrl_send.finish().ok();
    Ok(ReceiveOutcome { accepted: true, saved })
}

use std::sync::Arc;

use quinn::Endpoint;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct IncomingOffer {
    pub transfer_id: String,
    pub from_name: String,
    pub from_peer_id: String,
    pub files: Vec<crate::protocol::OfferedFile>,
    pub total_size: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProgressEvent {
    pub transfer_id: String,
    pub direction: String,
    pub file_name: String,
    pub bytes: u64,
    pub total: u64,
}

pub async fn run_server(endpoint: Endpoint, app: AppHandle, state: Arc<AppState>) {
    while let Some(incoming) = endpoint.accept().await {
        let app = app.clone();
        let state = state.clone();
        tauri::async_runtime::spawn(async move {
            let conn = match incoming.await {
                Ok(c) => c,
                Err(_) => return,
            };
            let transfer_id = format!("in-{}", conn.stable_id());
            let dest_dir = state.download_dir();

            let app_for_decide = app.clone();
            let state_for_decide = state.clone();
            let tid = transfer_id.clone();
            let app_for_progress = app.clone();
            let tid_progress = transfer_id.clone();

            let outcome = receive_transfer(
                &conn,
                &dest_dir,
                move |offer: Offer| {
                    let app = app_for_decide;
                    let state = state_for_decide;
                    let tid = tid.clone();
                    async move {
                        let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
                        state.pending.lock().unwrap().insert(tid.clone(), tx);
                        let _ = app.emit(
                            "incoming-offer",
                            &IncomingOffer {
                                transfer_id: tid.clone(),
                                from_name: offer.from_name,
                                from_peer_id: offer.from_peer_id,
                                files: offer.files,
                                total_size: offer.total_size,
                            },
                        );
                        rx.await.unwrap_or(false)
                    }
                },
                move |name, bytes, total| {
                    let _ = app_for_progress.emit(
                        "transfer-progress",
                        &ProgressEvent {
                            transfer_id: tid_progress.clone(),
                            direction: "incoming".into(),
                            file_name: name.to_string(),
                            bytes,
                            total,
                        },
                    );
                },
            )
            .await;

            if matches!(outcome, Ok(ref o) if o.accepted) {
                let _ = app.emit("transfer-done", &transfer_id);
            }
        });
    }
}
