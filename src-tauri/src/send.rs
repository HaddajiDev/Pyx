use std::path::PathBuf;

use quinn::Connection;
use tokio::io::AsyncReadExt;

use crate::hash::Hasher;
use crate::protocol::{AcceptDecision, FileAck, FileHeader, Offer, OfferedFile};
use crate::protocol_io::{read_msg, write_msg};

const CHUNK: usize = 64 * 1024;

#[derive(Debug)]
pub struct SendOutcome {
    pub accepted: bool,
    pub files_sent: usize,
}

#[derive(Debug, Clone)]
struct PlannedFile {
    abs: PathBuf,
    rel: String,
    size: u64,
}

fn base_name(path: &std::path::Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("file")
        .to_string()
}

async fn plan_files(
    inputs: Vec<PathBuf>,
) -> Result<Vec<PlannedFile>, Box<dyn std::error::Error + Send + Sync>> {
    let mut out = Vec::new();
    for input in inputs {
        let meta = tokio::fs::metadata(&input).await?;
        if meta.is_file() {
            let name = base_name(&input);
            out.push(PlannedFile { abs: input, rel: name, size: meta.len() });
        } else if meta.is_dir() {
            let root = base_name(&input);
            let mut stack = vec![(input.clone(), root)];
            while let Some((dir, prefix)) = stack.pop() {
                let mut rd = tokio::fs::read_dir(&dir).await?;
                while let Some(entry) = rd.next_entry().await? {
                    let entry_meta = entry.metadata().await?;
                    let name = entry.file_name().to_string_lossy().to_string();
                    let rel = format!("{prefix}/{name}");
                    if entry_meta.is_dir() {
                        stack.push((entry.path(), rel));
                    } else if entry_meta.is_file() {
                        out.push(PlannedFile {
                            abs: entry.path(),
                            rel,
                            size: entry_meta.len(),
                        });
                    }
                }
            }
        }
    }
    Ok(out)
}

pub async fn send_files<O, F>(
    conn: &Connection,
    from_name: String,
    from_peer_id: String,
    paths: Vec<PathBuf>,
    on_offer: O,
    mut on_progress: F,
) -> Result<SendOutcome, Box<dyn std::error::Error + Send + Sync>>
where
    O: FnOnce(&[OfferedFile]),
    F: FnMut(&str, u64, u64),
{
    let plan = plan_files(paths).await?;
    let offered: Vec<OfferedFile> = plan
        .iter()
        .map(|f| OfferedFile {
            name: base_name(&f.abs),
            rel_path: f.rel.clone(),
            size: f.size,
        })
        .collect();
    let total_size = plan.iter().map(|f| f.size).sum();

    on_offer(&offered);

    let (mut ctrl_send, mut ctrl_recv) = conn.open_bi().await?;
    let offer = Offer { from_name, from_peer_id, files: offered.clone(), total_size };
    write_msg(&mut ctrl_send, &offer).await?;

    let decision: AcceptDecision = read_msg(&mut ctrl_recv).await?;
    if !decision.ok {
        ctrl_send.finish().ok();
        return Ok(SendOutcome { accepted: false, files_sent: 0 });
    }

    let mut files_sent = 0usize;
    for planned in &plan {
        let mut file = tokio::fs::File::open(&planned.abs).await?;
        let mut hasher = Hasher::new();
        let mut buf = vec![0u8; CHUNK];
        loop {
            let n = file.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        let blake3_hex = hasher.finalize_hex();

        let mut body = conn.open_uni().await?;
        let header = FileHeader {
            name: base_name(&planned.abs),
            rel_path: planned.rel.clone(),
            size: planned.size,
            blake3_hex,
        };
        write_msg(&mut body, &header).await?;

        let mut file = tokio::fs::File::open(&planned.abs).await?;
        let mut sent: u64 = 0;
        loop {
            let n = file.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            body.write_all(&buf[..n]).await?;
            sent += n as u64;
            on_progress(&planned.rel, sent, planned.size);
        }
        body.finish()?;

        let ack: FileAck = read_msg(&mut ctrl_recv).await?;
        if ack.ok {
            files_sent += 1;
        }
    }

    ctrl_send.finish().ok();
    Ok(SendOutcome { accepted: true, files_sent })
}
