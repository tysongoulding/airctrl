use airctrl_lan::{generate_run_ps1, script::sanitize_host_ip, start_lan_server, LanServerConfig};
use reqwest::StatusCode;
use std::collections::HashSet;
use std::net::Ipv4Addr;
use tempfile::tempdir;

#[tokio::test]
async fn test_adversarial_dynamic_port_allocation_concurrency() {
    let tmp = tempdir().expect("Failed to create tempdir");
    let fake_exe = tmp.path().join("airctrl.exe");
    tokio::fs::write(&fake_exe, b"MZfakebinary").await.unwrap();

    let server_count = 5;
    let mut handles = Vec::new();
    let mut allocated_ports = HashSet::new();

    for i in 0..server_count {
        let config = LanServerConfig {
            bind_ip: Ipv4Addr::LOCALHOST,
            port: 0, // Dynamic port allocation
            advertised_ip: Some(Ipv4Addr::LOCALHOST),
            artifact_path: fake_exe.clone(),
            version: format!("0.1.0-dyn-{}", i),
        };

        let handle = start_lan_server(config)
            .await
            .expect("Failed to start server on dynamic port");
        let port = handle.local_addr.port();

        assert!(port > 0, "Dynamic port must be non-zero");
        assert!(
            allocated_ports.insert(port),
            "Port {} was allocated more than once across concurrent servers",
            port
        );
        handles.push(handle);
    }

    let client = reqwest::Client::new();

    // Verify all servers are live, serve their own health, and embed their own port in run.ps1
    for handle in &handles {
        let port = handle.local_addr.port();
        let base = format!("http://127.0.0.1:{}", port);

        // 1. Check health
        let health_res = client
            .get(format!("{}/health", base))
            .send()
            .await
            .unwrap();
        assert_eq!(health_res.status(), StatusCode::OK);

        // 2. Check run.ps1 has the specific port embedded
        let ps1_res = client
            .get(format!("{}/run.ps1", base))
            .send()
            .await
            .unwrap();
        assert_eq!(ps1_res.status(), StatusCode::OK);
        let ps1_text = ps1_res.text().await.unwrap();
        assert!(
            ps1_text.contains(&format!("$Port = {}", port)),
            "Script for server on port {} did not contain '$Port = {}'",
            port,
            port
        );
    }

    // Clean shutdown
    for handle in handles {
        handle.shutdown().await;
    }
}

#[tokio::test]
async fn test_adversarial_directory_traversal_and_routing_rejections() {
    let tmp = tempdir().expect("Failed to create tempdir");
    let fake_exe = tmp.path().join("airctrl.exe");
    tokio::fs::write(&fake_exe, b"MZtraversaltest").await.unwrap();

    let config = LanServerConfig {
        bind_ip: Ipv4Addr::LOCALHOST,
        port: 0,
        advertised_ip: Some(Ipv4Addr::LOCALHOST),
        artifact_path: fake_exe.clone(),
        version: "0.1.0-traversal".to_string(),
    };

    let handle = start_lan_server(config).await.unwrap();
    let base = format!("http://127.0.0.1:{}", handle.local_addr.port());
    let client = reqwest::Client::new();

    // 1. Path traversal attempts
    let traversal_paths = [
        "/../Cargo.toml",
        "/../../etc/passwd",
        "/airctrl.exe/..",
        "/airctrl.exe/../Cargo.toml",
        "/run.ps1/../../etc/shadow",
        "//etc/passwd",
        "/unknown/route",
        "/api/v1/download",
    ];

    for path in &traversal_paths {
        let url = format!("{}{}", base, path);
        let resp = client.get(&url).send().await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "Expected 404 Not Found for path '{}', got {}",
            path,
            resp.status()
        );
    }

    // 2. Unsupported HTTP methods (POST, PUT, DELETE, PATCH) on valid GET routes
    let routes = ["/health", "/version", "/run.ps1", "/airctrl.exe"];
    for route in &routes {
        let post_resp = client.post(format!("{}{}", base, route)).send().await.unwrap();
        assert_eq!(
            post_resp.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "Expected 405 Method Not Allowed for POST {}, got {}",
            route,
            post_resp.status()
        );

        let put_resp = client.put(format!("{}{}", base, route)).send().await.unwrap();
        assert_eq!(
            put_resp.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "Expected 405 Method Not Allowed for PUT {}, got {}",
            route,
            put_resp.status()
        );

        let del_resp = client.delete(format!("{}{}", base, route)).send().await.unwrap();
        assert_eq!(
            del_resp.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "Expected 405 Method Not Allowed for DELETE {}, got {}",
            route,
            del_resp.status()
        );
    }

    handle.shutdown().await;
}

#[tokio::test]
async fn test_adversarial_artifact_directory_and_missing_guards() {
    let tmp = tempdir().expect("Failed to create tempdir");
    let sub_dir = tmp.path().join("a_directory_not_a_file");
    tokio::fs::create_dir(&sub_dir).await.unwrap();

    // 1. Test when artifact_path is a directory
    let config_dir = LanServerConfig {
        bind_ip: Ipv4Addr::LOCALHOST,
        port: 0,
        advertised_ip: Some(Ipv4Addr::LOCALHOST),
        artifact_path: sub_dir,
        version: "0.1.0-dir".to_string(),
    };

    let handle_dir = start_lan_server(config_dir).await.unwrap();
    let base_dir = format!("http://127.0.0.1:{}", handle_dir.local_addr.port());
    let client = reqwest::Client::new();

    let resp_dir = client
        .get(format!("{}/airctrl.exe", base_dir))
        .send()
        .await
        .unwrap();
    assert_eq!(resp_dir.status(), StatusCode::NOT_FOUND);
    let body_dir = resp_dir.text().await.unwrap();
    assert!(body_dir.contains("is a directory, expected an executable binary"));

    handle_dir.shutdown().await;

    // 2. Test when artifact_path is non-existent
    let non_existent = tmp.path().join("does_not_exist.exe");
    let config_missing = LanServerConfig {
        bind_ip: Ipv4Addr::LOCALHOST,
        port: 0,
        advertised_ip: Some(Ipv4Addr::LOCALHOST),
        artifact_path: non_existent,
        version: "0.1.0-missing".to_string(),
    };

    let handle_missing = start_lan_server(config_missing).await.unwrap();
    let base_missing = format!("http://127.0.0.1:{}", handle_missing.local_addr.port());

    let resp_missing = client
        .get(format!("{}/airctrl.exe", base_missing))
        .send()
        .await
        .unwrap();
    assert_eq!(resp_missing.status(), StatusCode::NOT_FOUND);
    let body_missing = resp_missing.text().await.unwrap();
    assert!(body_missing.contains("not found. Please compile airctrl.exe first."));

    handle_missing.shutdown().await;
}

#[test]
fn test_adversarial_powershell_subexpression_and_injection_matrix() {
    let attack_vectors = [
        "$(calc.exe)",
        "$(whoami)",
        "$(Invoke-Expression 'Get-Process')",
        "$env:COMPUTERNAME",
        "$PID",
        "127.0.0.1; Remove-Item -Path C:\\ -Recurse",
        "127.0.0.1 & calc.exe",
        "127.0.0.1 | iex",
        "127.0.0.1' # comment breakout",
        "127.0.0.1\" # double quote breakout",
        "127.0.0.1`ncalc.exe",
        "127.0.0.1\r\nStop-Process -Name svchost\r\n",
        "127.0.0.1`",
        "127.0.0.1\0",
        "\u{FF04}(calc.exe)", // fullwidth dollar sign
        "\u{FF08}calc.exe\u{FF09}", // fullwidth parens
        "../../attacker.internal.domain",
        "192.168.1.1/run.ps1",
        "::1; calc.exe",
    ];

    for attack in &attack_vectors {
        // Invariant 1: sanitize_host_ip MUST reject the malicious host
        let sanitized = sanitize_host_ip(attack);
        assert!(
            sanitized.is_err(),
            "sanitize_host_ip unexpectedly allowed malicious attack string: '{}'",
            attack
        );

        // Invariant 2: generate_run_ps1 MUST NOT interpolate malicious payload into script
        let script = generate_run_ps1(attack, 8080);
        assert!(
            !script.contains(attack),
            "generate_run_ps1 interpolated raw attack string: '{}'",
            attack
        );
        assert!(
            script.contains("$HostIp = '127.0.0.1'"),
            "generate_run_ps1 did not fall back safely to 127.0.0.1 on injection attack: '{}'",
            attack
        );
    }
}

#[test]
fn test_adversarial_powershell_valid_ipv4_ipv6_hostnames() {
    let valid_cases = [
        ("192.168.1.1", "192.168.1.1"),
        ("10.0.0.5", "10.0.0.5"),
        ("172.16.254.1", "172.16.254.1"),
        ("127.0.0.1", "127.0.0.1"),
        ("::1", "[::1]"),
        ("[::1]", "[::1]"),
        ("2001:db8::1", "[2001:db8::1]"),
        ("[2001:db8::1]", "[2001:db8::1]"),
        ("localhost", "localhost"),
        ("build-server.local", "build-server.local"),
        ("windows-pc-1", "windows-pc-1"),
    ];

    for (input, expected) in &valid_cases {
        let res = sanitize_host_ip(input)
            .unwrap_or_else(|_| panic!("Failed for valid host: {}", input));
        assert_eq!(res, *expected);

        let script = generate_run_ps1(input, 9090);
        assert!(script.contains(&format!("$HostIp = '{}'", expected)));
        assert!(script.contains("$Port = 9090"));
    }
}
