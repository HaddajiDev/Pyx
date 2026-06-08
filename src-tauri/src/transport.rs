use std::error::Error;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use quinn::{ClientConfig, Endpoint, ServerConfig};
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};

pub fn ensure_crypto_provider() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

fn generate_self_signed() -> Result<(CertificateDer<'static>, PrivatePkcs8KeyDer<'static>), Box<dyn Error>> {
    let cert = rcgen::generate_simple_self_signed(vec!["filedrop.local".to_string()])?;
    let cert_der = CertificateDer::from(cert.cert);
    let key = PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der());
    Ok((cert_der, key))
}

pub fn make_server_endpoint() -> Result<Endpoint, Box<dyn Error>> {
    ensure_crypto_provider();
    let (cert, key) = generate_self_signed()?;
    let mut server_config = ServerConfig::with_single_cert(vec![cert], key.into())?;
    let transport = Arc::get_mut(&mut server_config.transport).unwrap();
    transport.max_idle_timeout(Some(std::time::Duration::from_secs(30).try_into()?));
    let addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 0);
    let endpoint = Endpoint::server(server_config, addr)?;
    Ok(endpoint)
}

#[derive(Debug)]
struct SkipServerVerification(Arc<rustls::crypto::CryptoProvider>);

impl SkipServerVerification {
    fn new() -> Arc<Self> {
        Arc::new(Self(Arc::new(rustls::crypto::ring::default_provider())))
    }
}

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message, cert, dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message, cert, dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

pub fn make_client_endpoint() -> Result<Endpoint, Box<dyn Error>> {
    ensure_crypto_provider();
    let crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(SkipServerVerification::new())
        .with_no_client_auth();
    let client_config = ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(crypto)?,
    ));
    let addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 0);
    let mut endpoint = Endpoint::client(addr)?;
    endpoint.set_default_client_config(client_config);
    Ok(endpoint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn client_connects_to_server_and_exchanges_bytes() {
        let server = make_server_endpoint().unwrap();
        let server_addr =
            SocketAddr::from((Ipv4Addr::LOCALHOST, server.local_addr().unwrap().port()));

        let server_task = tokio::spawn(async move {
            let incoming = server.accept().await.unwrap();
            let conn = incoming.await.unwrap();
            let (mut send, mut recv) = conn.accept_bi().await.unwrap();
            let mut buf = [0u8; 5];
            recv.read_exact(&mut buf).await.unwrap();
            send.write_all(&buf).await.unwrap();
            send.finish().unwrap();
            conn.closed().await;
            buf
        });

        let client = make_client_endpoint().unwrap();
        let conn = client
            .connect(server_addr, "filedrop.local")
            .unwrap()
            .await
            .unwrap();
        let (mut send, mut recv) = conn.open_bi().await.unwrap();
        send.write_all(b"hello").await.unwrap();
        send.finish().unwrap();
        let mut echoed = [0u8; 5];
        recv.read_exact(&mut echoed).await.unwrap();
        assert_eq!(&echoed, b"hello");

        conn.close(0u32.into(), b"done");
        client.wait_idle().await;

        let server_got = server_task.await.unwrap();
        assert_eq!(&server_got, b"hello");
    }
}
