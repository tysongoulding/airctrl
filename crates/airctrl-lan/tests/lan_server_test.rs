use airctrl_lan::{start_lan_server, LanServerConfig};
use std::net::Ipv4Addr;
use std::path::PathBuf;
use tempfile::tempdir;

#[tokio::test]
async fn test_lan_server_endpoints_and_artifact_serving() {
    let tmp = tempdir().expect("Failed to create tempdir");
    let fake_exe = tmp.path().join("airctrl.exe");

    // Write synthetic PE dummy executable (MZ header + 2KB data)
    let mut dummy_payload = vec![0x4D, 0x5A, 0x90, 0x00];
    dummy_payload.resize(2048, 0xAA);
    tokio::fs::write(&fake_exe, &dummy_payload).await.expect("Failed to write dummy exe");

    let config = LanServerConfig {
        bind_ip: Ipv4Addr::LOCALHOST,
        port: 0, // OS assigns ephemeral port
        advertised_ip: Some(Ipv4Addr::LOCALHOST),
        artifact_path: fake_exe.clone(),
        version: "0.1.0-test".to_string(),
    };

    let handle = start_lan_server(config).await.expect("Server failed to start");
    let base = format!("http://127.0.0.1:{}", handle.local_addr.port());
    let client = reqwest::Client::new();

    // 1. Test /health
    let health_resp = client.get(format!("{}/health", base)).send().await.expect("Health req failed");
    assert_eq!(health_resp.status(), 200);
    let health_json: serde_json::Value = health_resp.json().await.expect("Health json invalid");
    assert_eq!(health_json["status"], "healthy");
    assert_eq!(health_json["artifact_available"], true);
    assert_eq!(health_json["artifact_size_bytes"], 2048);

    // 2. Test /version
    let version_resp = client.get(format!("{}/version", base)).send().await.expect("Version req failed");
    assert_eq!(version_resp.status(), 200);
    let version_json: serde_json::Value = version_resp.json().await.expect("Version json invalid");
    assert_eq!(version_json["version"], "0.1.0-test");
    assert_eq!(version_json["target"], "x86_64-pc-windows-msvc");
    assert_eq!(version_json["artifact_available"], true);
    assert!(version_json["artifact_sha256"].as_str().is_some());

    // 3. Test /run.ps1
    let ps1_resp = client.get(format!("{}/run.ps1", base)).send().await.expect("Run.ps1 req failed");
    assert_eq!(ps1_resp.status(), 200);
    let ps1_text = ps1_resp.text().await.expect("Failed to read script body");
    assert!(ps1_text.contains(&format!("$Port = {}", handle.local_addr.port())));
    assert!(ps1_text.contains("airctrl.exe"));
    assert!(ps1_text.contains("Invoke-RestMethod"));
    assert!(ps1_text.contains("Start-Process"));

    // 4. Test /airctrl.exe binary download
    let exe_resp = client.get(format!("{}/airctrl.exe", base)).send().await.expect("Exe req failed");
    assert_eq!(exe_resp.status(), 200);
    assert_eq!(
        exe_resp.headers().get("content-type").unwrap(),
        "application/vnd.microsoft.portable-executable"
    );
    let downloaded_bytes = exe_resp.bytes().await.expect("Failed to read downloaded bytes");
    assert_eq!(downloaded_bytes.as_ref(), dummy_payload.as_slice());

    // 5. Test graceful shutdown
    handle.shutdown().await;
}

#[tokio::test]
async fn test_lan_server_missing_artifact_returns_404() {
    let config = LanServerConfig {
        bind_ip: Ipv4Addr::LOCALHOST,
        port: 0,
        advertised_ip: Some(Ipv4Addr::LOCALHOST),
        artifact_path: PathBuf::from("/non/existent/airctrl.exe"),
        version: "0.1.0-test".to_string(),
    };

    let handle = start_lan_server(config).await.expect("Server failed to start");
    let base = format!("http://127.0.0.1:{}", handle.local_addr.port());
    let client = reqwest::Client::new();

    let exe_resp = client.get(format!("{}/airctrl.exe", base)).send().await.expect("Exe req failed");
    assert_eq!(exe_resp.status(), 404);

    let health_resp = client.get(format!("{}/health", base)).send().await.expect("Health req failed");
    let health_json: serde_json::Value = health_resp.json().await.expect("Health json invalid");
    assert_eq!(health_json["artifact_available"], false);

    handle.shutdown().await;
}
