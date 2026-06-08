use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OfferedFile {
    pub name: String,
    pub rel_path: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Offer {
    pub from_name: String,
    pub from_peer_id: String,
    pub files: Vec<OfferedFile>,
    pub total_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AcceptDecision {
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileHeader {
    pub name: String,
    pub rel_path: String,
    pub size: u64,
    pub blake3_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileAck {
    pub name: String,
    pub ok: bool,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offer_round_trips_through_json() {
        let offer = Offer {
            from_name: "Alice".into(),
            from_peer_id: "abc-123".into(),
            files: vec![
                OfferedFile { name: "build.zip".into(), rel_path: "build.zip".into(), size: 1024 },
                OfferedFile {
                    name: "logo.png".into(),
                    rel_path: "assets/logo.png".into(),
                    size: 2048,
                },
            ],
            total_size: 3072,
        };
        let json = serde_json::to_vec(&offer).unwrap();
        let back: Offer = serde_json::from_slice(&json).unwrap();
        assert_eq!(offer, back);
    }

    #[test]
    fn file_header_round_trips() {
        let h = FileHeader {
            name: "a.bin".into(),
            rel_path: "sub/a.bin".into(),
            size: 9,
            blake3_hex: "deadbeef".into(),
        };
        let json = serde_json::to_vec(&h).unwrap();
        let back: FileHeader = serde_json::from_slice(&json).unwrap();
        assert_eq!(h, back);
    }
}
