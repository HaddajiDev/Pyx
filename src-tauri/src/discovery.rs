use std::sync::Arc;

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use tauri::{AppHandle, Emitter};

use crate::state::{AppState, Peer};

const SERVICE_TYPE: &str = "_filedrop._udp.local.";

pub fn start(app: AppHandle, state: Arc<AppState>, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let daemon = ServiceDaemon::new()?;

    let (peer_id, display_name) = {
        let id = state.identity.lock().unwrap();
        (id.peer_id.clone(), id.display_name.clone())
    };

    let host = format!("{peer_id}.local.");
    let properties = [
        ("peer_id", peer_id.as_str()),
        ("name", display_name.as_str()),
    ];
    let service = ServiceInfo::new(
        SERVICE_TYPE,
        &peer_id,
        &host,
        (),
        port,
        &properties[..],
    )?
    .enable_addr_auto();

    daemon.register(service)?;

    let receiver = daemon.browse(SERVICE_TYPE)?;
    let my_id = peer_id.clone();

    tauri::async_runtime::spawn(async move {
        while let Ok(event) = receiver.recv_async().await {
            match event {
                ServiceEvent::ServiceResolved(info) => {
                    let found_id = info
                        .get_property_val_str("peer_id")
                        .unwrap_or("")
                        .to_string();
                    if found_id.is_empty() || found_id == my_id {
                        continue;
                    }
                    let name = info
                        .get_property_val_str("name")
                        .unwrap_or("Unknown")
                        .to_string();
                    let addr = info
                        .get_addresses()
                        .iter()
                        .next()
                        .map(|ip| format!("{ip}:{}", info.get_port()));
                    if let Some(addr) = addr {
                        let peer = Peer { peer_id: found_id.clone(), display_name: name, addr };
                        state.upsert_peer(peer.clone());
                        let _ = app.emit("peer-found", &peer);
                    }
                }
                ServiceEvent::ServiceRemoved(_ty, fullname) => {
                    let removed_id = fullname.split('.').next().unwrap_or("").to_string();
                    if !removed_id.is_empty() {
                        state.remove_peer(&removed_id);
                        let _ = app.emit("peer-lost", &removed_id);
                    }
                }
                _ => {}
            }
        }
    });

    std::mem::forget(daemon);
    Ok(())
}
