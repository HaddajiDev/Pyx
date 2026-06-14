use std::sync::{Arc, Mutex};

use pyx_lib::receive::receive_transfer;
use pyx_lib::send::send_files;
use pyx_lib::transport::{make_client_endpoint, make_server_endpoint};

#[tokio::test]
async fn sends_two_files_end_to_end_and_verifies_contents() {
    let src = tempfile::tempdir().unwrap();
    let f1 = src.path().join("build.bin");
    let f2 = src.path().join("dump.sql");
    let data1 = vec![7u8; 200_000];
    let data2 = b"SELECT 1;".to_vec();
    std::fs::write(&f1, &data1).unwrap();
    std::fs::write(&f2, &data2).unwrap();

    let dst = tempfile::tempdir().unwrap();
    let dst_path = dst.path().to_path_buf();

    let server = make_server_endpoint().unwrap();
    let server_addr = std::net::SocketAddr::from((
        std::net::Ipv4Addr::LOCALHOST,
        server.local_addr().unwrap().port(),
    ));
    let progress = Arc::new(Mutex::new(Vec::<String>::new()));
    let progress2 = progress.clone();
    let recv_task = tokio::spawn(async move {
        let incoming = server.accept().await.unwrap();
        let conn = incoming.await.unwrap();
        receive_transfer(
            &conn,
            &dst_path,
            |_offer| async { true },
            |name, _b, _t| progress2.lock().unwrap().push(name.to_string()),
        )
        .await
        .unwrap()
    });

    let client = make_client_endpoint().unwrap();
    let conn = client
        .connect(server_addr, "filedrop.local")
        .unwrap()
        .await
        .unwrap();
    let outcome = send_files(
        &conn,
        "Tester".into(),
        "peer-1".into(),
        vec![f1.clone(), f2.clone()],
        |_files| {},
        |_name, _b, _t| {},
    )
    .await
    .unwrap();

    let recv_outcome = recv_task.await.unwrap();

    assert!(outcome.accepted);
    assert_eq!(outcome.files_sent, 2);
    assert!(recv_outcome.accepted);
    assert_eq!(recv_outcome.saved.len(), 2);

    let got1 = std::fs::read(dst.path().join("build.bin")).unwrap();
    let got2 = std::fs::read(dst.path().join("dump.sql")).unwrap();
    assert_eq!(got1, data1);
    assert_eq!(got2, data2);
    assert!(!progress.lock().unwrap().is_empty());
}

#[tokio::test]
async fn rejected_offer_writes_nothing() {
    let src = tempfile::tempdir().unwrap();
    let f1 = src.path().join("secret.bin");
    std::fs::write(&f1, b"nope").unwrap();
    let dst = tempfile::tempdir().unwrap();
    let dst_path = dst.path().to_path_buf();

    let server = make_server_endpoint().unwrap();
    let server_addr = std::net::SocketAddr::from((
        std::net::Ipv4Addr::LOCALHOST,
        server.local_addr().unwrap().port(),
    ));
    let recv_task = tokio::spawn(async move {
        let incoming = server.accept().await.unwrap();
        let conn = incoming.await.unwrap();
        receive_transfer(&conn, &dst_path, |_o| async { false }, |_n, _b, _t| {})
            .await
            .unwrap()
    });

    let client = make_client_endpoint().unwrap();
    let conn = client.connect(server_addr, "filedrop.local").unwrap().await.unwrap();
    let outcome = send_files(&conn, "T".into(), "p".into(), vec![f1], |_| {}, |_, _, _| {})
        .await
        .unwrap();
    let recv_outcome = recv_task.await.unwrap();

    assert!(!outcome.accepted);
    assert!(!recv_outcome.accepted);
    assert!(std::fs::read_dir(dst.path()).unwrap().next().is_none());
}

#[tokio::test]
async fn sends_a_folder_preserving_structure() {
    let src = tempfile::tempdir().unwrap();
    let proj = src.path().join("project");
    let sub = proj.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(proj.join("main.rs"), b"fn main() {}").unwrap();
    std::fs::write(sub.join("util.rs"), vec![9u8; 100_000]).unwrap();

    let dst = tempfile::tempdir().unwrap();
    let dst_path = dst.path().to_path_buf();

    let server = make_server_endpoint().unwrap();
    let server_addr = std::net::SocketAddr::from((
        std::net::Ipv4Addr::LOCALHOST,
        server.local_addr().unwrap().port(),
    ));
    let recv_task = tokio::spawn(async move {
        let incoming = server.accept().await.unwrap();
        let conn = incoming.await.unwrap();
        receive_transfer(&conn, &dst_path, |_o| async { true }, |_n, _b, _t| {})
            .await
            .unwrap()
    });

    let client = make_client_endpoint().unwrap();
    let conn = client
        .connect(server_addr, "filedrop.local")
        .unwrap()
        .await
        .unwrap();
    let outcome = send_files(&conn, "T".into(), "p".into(), vec![proj.clone()], |_| {}, |_, _, _| {})
        .await
        .unwrap();
    let recv_outcome = recv_task.await.unwrap();

    assert!(outcome.accepted);
    assert_eq!(outcome.files_sent, 2);
    assert_eq!(recv_outcome.saved.len(), 2);

    let got_main = std::fs::read(dst.path().join("project").join("main.rs")).unwrap();
    let got_util = std::fs::read(dst.path().join("project").join("sub").join("util.rs")).unwrap();
    assert_eq!(got_main, b"fn main() {}");
    assert_eq!(got_util, vec![9u8; 100_000]);
}

#[cfg(windows)]
#[tokio::test]
async fn folder_transfer_survives_a_locked_source_file() {
    use std::os::windows::fs::OpenOptionsExt;

    let src = tempfile::tempdir().unwrap();
    let proj = src.path().join("proj");
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::write(proj.join("a.txt"), b"first file").unwrap();
    std::fs::write(proj.join("b.txt"), vec![5u8; 50_000]).unwrap();
    std::fs::write(proj.join("c.txt"), b"third file").unwrap();

    // Lock b.txt exclusively (FILE_SHARE_NONE) so the sender can't read it.
    let locked = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(proj.join("b.txt"))
        .unwrap();

    let dst = tempfile::tempdir().unwrap();
    let dst_path = dst.path().to_path_buf();
    let server = make_server_endpoint().unwrap();
    let server_addr = std::net::SocketAddr::from((
        std::net::Ipv4Addr::LOCALHOST,
        server.local_addr().unwrap().port(),
    ));
    let recv_task = tokio::spawn(async move {
        let incoming = server.accept().await.unwrap();
        let conn = incoming.await.unwrap();
        receive_transfer(&conn, &dst_path, |_o| async { true }, |_n, _b, _t| {})
            .await
            .unwrap()
    });

    let client = make_client_endpoint().unwrap();
    let conn = client
        .connect(server_addr, "filedrop.local")
        .unwrap()
        .await
        .unwrap();
    let outcome = send_files(&conn, "T".into(), "p".into(), vec![proj.clone()], |_| {}, |_, _, _| {})
        .await
        .unwrap();
    let recv_outcome = recv_task.await.unwrap();

    drop(locked);

    // The transfer should COMPLETE; the two readable files arrive, locked one doesn't.
    assert!(outcome.accepted);
    assert_eq!(outcome.files_sent, 2, "readable files should still send");
    assert_eq!(recv_outcome.saved.len(), 2);
    assert!(dst.path().join("proj").join("a.txt").exists());
    assert!(dst.path().join("proj").join("c.txt").exists());
    assert!(!dst.path().join("proj").join("b.txt").exists());
}
