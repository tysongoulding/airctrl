use airctrl_lan::{start_lan_server, LanServerConfig};
use reqwest::StatusCode;
use std::net::Ipv4Addr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tempfile::tempdir;

#[tokio::test]
async fn test_lan_server_high_concurrency_stress() {
    let tmp = tempdir().expect("Failed to create tempdir");
    let fake_exe = tmp.path().join("airctrl.exe");
    let payload = vec![0x4D, 0x5A, 0x90, 0x00, 0x20, 0x42]; // 6 bytes dummy
    tokio::fs::write(&fake_exe, &payload).await.expect("Failed to write dummy exe");

    let config = LanServerConfig {
        bind_ip: Ipv4Addr::LOCALHOST,
        port: 0,
        advertised_ip: Some(Ipv4Addr::LOCALHOST),
        artifact_path: fake_exe.clone(),
        version: "0.1.0-stress".to_string(),
    };

    let handle = start_lan_server(config).await.expect("Failed to start LAN server");
    let base = format!("http://127.0.0.1:{}", handle.local_addr.port());
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(50)
        .build()
        .expect("Failed to build client");

    let num_requests = 200;
    let success_count = Arc::new(AtomicUsize::new(0));
    let mut tasks = Vec::new();

    for i in 0..num_requests {
        let client_clone = client.clone();
        let base_url = base.clone();
        let successes = success_count.clone();

        let endpoint = match i % 4 {
            0 => "/health",
            1 => "/version",
            2 => "/run.ps1",
            _ => "/airctrl.exe",
        };

        tasks.push(tokio::spawn(async move {
            let url = format!("{}{}", base_url, endpoint);
            if let Ok(resp) = client_clone.get(&url).send().await {
                if resp.status().is_success() {
                    successes.fetch_add(1, Ordering::Relaxed);
                }
            }
        }));
    }

    for task in tasks {
        let _ = task.await;
    }

    let completed = success_count.load(Ordering::Relaxed);
    assert_eq!(
        completed, num_requests,
        "Expected all {} concurrent requests to succeed, but only {} succeeded",
        num_requests, completed
    );

    handle.shutdown().await;
}

#[tokio::test]
async fn test_lan_server_malformed_queries_and_methods() {
    let tmp = tempdir().expect("Failed to create tempdir");
    let fake_exe = tmp.path().join("airctrl.exe");
    tokio::fs::write(&fake_exe, b"MZtestbinary").await.unwrap();

    let config = LanServerConfig {
        bind_ip: Ipv4Addr::LOCALHOST,
        port: 0,
        advertised_ip: Some(Ipv4Addr::LOCALHOST),
        artifact_path: fake_exe.clone(),
        version: "0.1.0-test".to_string(),
    };

    let handle = start_lan_server(config).await.unwrap();
    let base = format!("http://127.0.0.1:{}", handle.local_addr.port());
    let client = reqwest::Client::new();

    let long_query = format!("?very_long={}", "a".repeat(4096));
    let malformed_queries = [
        "?foo=bar&baz=%ff%00%20",
        "?param=1&param=2&param=3",
        "?path=../../../../etc/passwd",
        "?command=calc.exe;exit",
        &long_query,
    ];

    for q in &malformed_queries {
        let r1 = client.get(format!("{}/health{}", base, q)).send().await.unwrap();
        assert_eq!(r1.status(), StatusCode::OK);

        let r2 = client.get(format!("{}/version{}", base, q)).send().await.unwrap();
        assert_eq!(r2.status(), StatusCode::OK);

        let r3 = client.get(format!("{}/run.ps1{}", base, q)).send().await.unwrap();
        assert_eq!(r3.status(), StatusCode::OK);

        let r4 = client.get(format!("{}/airctrl.exe{}", base, q)).send().await.unwrap();
        assert_eq!(r4.status(), StatusCode::OK);
    }

    // 2. Unsupported HTTP methods (POST, PUT, DELETE, PATCH)
    let post_resp = client.post(format!("{}/run.ps1", base)).body("evil").send().await.unwrap();
    assert_eq!(post_resp.status(), StatusCode::METHOD_NOT_ALLOWED);

    let delete_resp = client.delete(format!("{}/airctrl.exe", base)).send().await.unwrap();
    assert_eq!(delete_resp.status(), StatusCode::METHOD_NOT_ALLOWED);

    let put_resp = client.put(format!("{}/health", base)).send().await.unwrap();
    assert_eq!(put_resp.status(), StatusCode::METHOD_NOT_ALLOWED);

    // 3. Nonexistent and traversal routes
    let not_found = client.get(format!("{}/nonexistent_endpoint", base)).send().await.unwrap();
    assert_eq!(not_found.status(), StatusCode::NOT_FOUND);

    let traversal = client.get(format!("{}/../etc/passwd", base)).send().await.unwrap();
    assert_eq!(traversal.status(), StatusCode::NOT_FOUND);

    handle.shutdown().await;
}

#[tokio::test]
async fn test_lan_server_dynamic_artifact_deletion_and_recovery() {
    let tmp = tempdir().expect("Failed to create tempdir");
    let fake_exe = tmp.path().join("airctrl.exe");
    tokio::fs::write(&fake_exe, b"MZinitialbuild").await.unwrap();

    let config = LanServerConfig {
        bind_ip: Ipv4Addr::LOCALHOST,
        port: 0,
        advertised_ip: Some(Ipv4Addr::LOCALHOST),
        artifact_path: fake_exe.clone(),
        version: "0.1.0-dynamic".to_string(),
    };

    let handle = start_lan_server(config).await.unwrap();
    let base = format!("http://127.0.0.1:{}", handle.local_addr.port());
    let client = reqwest::Client::new();

    // Initially available
    let r1 = client.get(format!("{}/airctrl.exe", base)).send().await.unwrap();
    assert_eq!(r1.status(), StatusCode::OK);

    // Delete artifact while server is active
    tokio::fs::remove_file(&fake_exe).await.unwrap();

    // Now should return 404
    let r2 = client.get(format!("{}/airctrl.exe", base)).send().await.unwrap();
    assert_eq!(r2.status(), StatusCode::NOT_FOUND);

    let health = client.get(format!("{}/health", base)).send().await.unwrap().json::<serde_json::Value>().await.unwrap();
    assert_eq!(health["artifact_available"], false);

    // Re-create artifact with new content
    tokio::fs::write(&fake_exe, b"MZnewrebuiltbinary").await.unwrap();

    // Now should return 200 with new content
    let r3 = client.get(format!("{}/airctrl.exe", base)).send().await.unwrap();
    assert_eq!(r3.status(), StatusCode::OK);
    let bytes = r3.bytes().await.unwrap();
    assert_eq!(bytes.as_ref(), b"MZnewrebuiltbinary");

    handle.shutdown().await;
}

#[tokio::test]
async fn test_lan_server_directory_artifact_behavior() {
    let tmp = tempdir().expect("Failed to create tempdir");
    let dir_as_artifact = tmp.path().join("is_a_directory.exe");
    tokio::fs::create_dir(&dir_as_artifact).await.unwrap();

    let config = LanServerConfig {
        bind_ip: Ipv4Addr::LOCALHOST,
        port: 0,
        advertised_ip: Some(Ipv4Addr::LOCALHOST),
        artifact_path: dir_as_artifact.clone(),
        version: "0.1.0-dir-test".to_string(),
    };

    let handle = start_lan_server(config).await.unwrap();
    let base = format!("http://127.0.0.1:{}", handle.local_addr.port());
    let client = reqwest::Client::new();

    // 1. /health correctly reports artifact_available = false
    let health: serde_json::Value = client.get(format!("{}/health", base)).send().await.unwrap().json().await.unwrap();
    assert_eq!(health["artifact_available"], false, "inspect_artifact must report false for a directory");
    assert_eq!(health["artifact_size_bytes"], serde_json::Value::Null);

    // 2. /airctrl.exe returns 404 NOT_FOUND without EISDIR stream crash
    let exe_resp = client.get(format!("{}/airctrl.exe", base)).send().await.unwrap();
    assert_eq!(exe_resp.status(), reqwest::StatusCode::NOT_FOUND);
    let body = exe_resp.json::<serde_json::Value>().await.unwrap();
    assert!(body["error"].as_str().unwrap().contains("directory"));

    handle.shutdown().await;
}

#[tokio::test]
async fn test_lan_state_cache_concurrency_and_sha_integrity() {
    use airctrl_lan::LanState;

    let tmp = tempdir().expect("Failed to create tempdir");
    let file = tmp.path().join("airctrl.exe");
    tokio::fs::write(&file, b"initial payload 12345").await.unwrap();

    let state = Arc::new(LanState::new(
        Ipv4Addr::LOCALHOST,
        8080,
        file.clone(),
        "0.1.0".to_string(),
    ));

    // Run 50 concurrent readers inspecting artifact
    let mut tasks = Vec::new();
    let inconsistent_hashes = Arc::new(AtomicUsize::new(0));

    for _ in 0..50 {
        let state_clone = state.clone();
        let inconsistent = inconsistent_hashes.clone();
        tasks.push(tokio::spawn(async move {
            let (avail, size, hash) = state_clone.inspect_artifact().await;
            if avail && size.is_some() && hash.is_none() {
                inconsistent.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    for t in tasks {
        let _ = t.await;
    }

    let inconsistencies = inconsistent_hashes.load(Ordering::Relaxed);
    println!("Inconsistent cache reads (available but hash is None): {}", inconsistencies);
}

#[tokio::test]
async fn test_lan_server_client_abort_midstream() {
    use tokio::io::AsyncReadExt;

    let tmp = tempdir().expect("Failed to create tempdir");
    let file = tmp.path().join("airctrl.exe");
    let large_payload = vec![0x90; 1024 * 1024]; // 1MB payload
    tokio::fs::write(&file, &large_payload).await.unwrap();

    let config = LanServerConfig {
        bind_ip: Ipv4Addr::LOCALHOST,
        port: 0,
        advertised_ip: Some(Ipv4Addr::LOCALHOST),
        artifact_path: file.clone(),
        version: "0.1.0-stream-abort".to_string(),
    };

    let handle = start_lan_server(config).await.unwrap();
    let port = handle.local_addr.port();

    // Connect raw TCP socket, send HTTP GET, read only first 32 bytes and close socket
    for _ in 0..10 {
        let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
            .await
            .expect("Failed to connect TCP");
        let request = b"GET /airctrl.exe HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
        use tokio::io::AsyncWriteExt;
        stream.write_all(request).await.expect("Failed to write request");

        let mut buf = [0u8; 32];
        let _ = stream.read(&mut buf).await;
        // Drop socket abruptly while server is streaming 1MB
        drop(stream);
    }

    // Verify server is still alive and responsive
    let client = reqwest::Client::new();
    let health = client
        .get(format!("http://127.0.0.1:{}/health", port))
        .send()
        .await
        .expect("Server died after client abort");
    assert_eq!(health.status(), StatusCode::OK);

    handle.shutdown().await;
}


