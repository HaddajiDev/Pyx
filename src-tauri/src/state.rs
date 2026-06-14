use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use mdns_sd::ServiceDaemon;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

#[derive(Debug, Clone, Serialize)]
pub struct Identity {
    pub peer_id: String,
    pub display_name: String,
}

impl Identity {
    pub fn generate() -> Self {
        let peer_id = uuid::Uuid::new_v4().to_string();
        let display_name = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Unknown".to_string());
        Self { peer_id, display_name }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Peer {
    pub peer_id: String,
    pub display_name: String,
    pub addr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Config {
    download_dir: Option<String>,
    #[serde(default)]
    display_name: Option<String>,
}

fn default_download_dir() -> PathBuf {
    dirs::download_dir().unwrap_or_else(std::env::temp_dir)
}

pub struct AppState {
    pub identity: Mutex<Identity>,
    pub server_port: Mutex<u16>,
    pub peers: Mutex<HashMap<String, Peer>>,
    pub pending: Mutex<HashMap<String, oneshot::Sender<bool>>>,
    download_dir: Mutex<PathBuf>,
    config_path: Mutex<Option<PathBuf>>,
    mdns: Mutex<Option<ServiceDaemon>>,
}

impl AppState {
    pub fn new(identity: Identity) -> Self {
        Self {
            identity: Mutex::new(identity),
            server_port: Mutex::new(0),
            peers: Mutex::new(HashMap::new()),
            pending: Mutex::new(HashMap::new()),
            download_dir: Mutex::new(default_download_dir()),
            config_path: Mutex::new(None),
            mdns: Mutex::new(None),
        }
    }

    pub fn download_dir(&self) -> PathBuf {
        self.download_dir.lock().unwrap().clone()
    }

    pub fn set_download_dir(&self, dir: PathBuf) {
        *self.download_dir.lock().unwrap() = dir;
        self.persist();
    }

    pub fn display_name(&self) -> String {
        self.identity.lock().unwrap().display_name.clone()
    }

    pub fn set_display_name(&self, name: String) {
        self.identity.lock().unwrap().display_name = name;
        self.persist();
    }

    pub fn set_mdns(&self, daemon: ServiceDaemon) {
        *self.mdns.lock().unwrap() = Some(daemon);
    }

    pub fn mdns(&self) -> Option<ServiceDaemon> {
        self.mdns.lock().unwrap().clone()
    }

    pub fn load_config(&self, config_path: PathBuf) {
        if let Ok(bytes) = std::fs::read(&config_path) {
            if let Ok(cfg) = serde_json::from_slice::<Config>(&bytes) {
                if let Some(d) = cfg.download_dir {
                    let p = PathBuf::from(d);
                    if p.is_dir() {
                        *self.download_dir.lock().unwrap() = p;
                    }
                }
                if let Some(n) = cfg.display_name {
                    if !n.trim().is_empty() {
                        self.identity.lock().unwrap().display_name = n;
                    }
                }
            }
        }
        *self.config_path.lock().unwrap() = Some(config_path);
    }

    fn persist(&self) {
        let Some(path) = self.config_path.lock().unwrap().clone() else {
            return;
        };
        let cfg = Config {
            download_dir: Some(self.download_dir().to_string_lossy().to_string()),
            display_name: Some(self.display_name()),
        };
        if let Ok(json) = serde_json::to_vec_pretty(&cfg) {
            let _ = std::fs::write(path, json);
        }
    }

    pub fn upsert_peer(&self, peer: Peer) {
        self.peers.lock().unwrap().insert(peer.peer_id.clone(), peer);
    }

    pub fn remove_peer(&self, peer_id: &str) {
        self.peers.lock().unwrap().remove(peer_id);
    }

    pub fn peer_addr(&self, peer_id: &str) -> Option<String> {
        self.peers.lock().unwrap().get(peer_id).map(|p| p.addr.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_nonempty() {
        let id = Identity::generate();
        assert!(!id.peer_id.is_empty());
        assert!(!id.display_name.is_empty());
    }

    #[test]
    fn download_dir_persists_and_reloads() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = tmp.path().join("config.json");
        let target = tmp.path().join("target");
        std::fs::create_dir(&target).unwrap();

        let s1 = AppState::new(Identity::generate());
        s1.load_config(cfg.clone());
        s1.set_download_dir(target.clone());
        assert_eq!(s1.download_dir(), target);

        let s2 = AppState::new(Identity::generate());
        s2.load_config(cfg);
        assert_eq!(s2.download_dir(), target);
    }

    #[test]
    fn peer_upsert_and_remove() {
        let state = AppState::new(Identity::generate());
        state.upsert_peer(Peer {
            peer_id: "p1".into(),
            display_name: "Mac".into(),
            addr: "10.0.0.2:5000".into(),
        });
        assert_eq!(state.peer_addr("p1"), Some("10.0.0.2:5000".into()));
        state.remove_peer("p1");
        assert_eq!(state.peer_addr("p1"), None);
    }
}
