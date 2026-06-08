use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const MAX_MSG_LEN: u32 = 1024 * 1024;

pub async fn write_msg<W, T>(w: &mut W, msg: &T) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let bytes = serde_json::to_vec(msg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let len = bytes.len() as u32;
    w.write_all(&len.to_be_bytes()).await?;
    w.write_all(&bytes).await?;
    w.flush().await?;
    Ok(())
}

pub async fn read_msg<R, T>(r: &mut R) -> std::io::Result<T>
where
    R: AsyncRead + Unpin,
    T: DeserializeOwned,
{
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf);
    if len > MAX_MSG_LEN {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "control message too large",
        ));
    }
    let mut buf = vec![0u8; len as usize];
    r.read_exact(&mut buf).await?;
    serde_json::from_slice(&buf)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{AcceptDecision, Offer, OfferedFile};

    #[tokio::test]
    async fn write_then_read_round_trips_over_duplex() {
        let (mut a, mut b) = tokio::io::duplex(64 * 1024);
        let offer = Offer {
            from_name: "Bob".into(),
            from_peer_id: "id-1".into(),
            files: vec![OfferedFile { name: "x".into(), rel_path: "x".into(), size: 1 }],
            total_size: 1,
        };
        let offer2 = offer.clone();
        let writer = tokio::spawn(async move {
            write_msg(&mut a, &offer2).await.unwrap();
        });
        let got: Offer = read_msg(&mut b).await.unwrap();
        writer.await.unwrap();
        assert_eq!(got, offer);
    }

    #[tokio::test]
    async fn multiple_messages_in_sequence() {
        let (mut a, mut b) = tokio::io::duplex(64 * 1024);
        let writer = tokio::spawn(async move {
            write_msg(&mut a, &AcceptDecision { ok: true }).await.unwrap();
            write_msg(&mut a, &AcceptDecision { ok: false }).await.unwrap();
        });
        let m1: AcceptDecision = read_msg(&mut b).await.unwrap();
        let m2: AcceptDecision = read_msg(&mut b).await.unwrap();
        writer.await.unwrap();
        assert!(m1.ok);
        assert!(!m2.ok);
    }
}
