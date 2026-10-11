use airctrl_actions::*;

#[test]
fn test_rea_inspect_valid_pids() {
    // Valid standard PID
    let res = ReaInspector::inspect(1234, None).expect("REA inspection failed");
    assert_eq!(res.target_pid, 1234);
    assert_eq!(res.binary_path, "/usr/bin/mock-app");
    assert_eq!(res.status, "SchemaExtracted");
    assert!(res.action_schema.buttons.contains(&"Save".to_string()));
    assert!(res.action_schema.inputs.contains(&"Username".to_string()));
    assert!(res.action_schema.menus.contains(&"File".to_string()));

    // Lowest valid boundary: PID 1
    let res_pid1 = ReaInspector::inspect(1, Some("/bin/init")).expect("PID 1 failed");
    assert_eq!(res_pid1.target_pid, 1);
    assert_eq!(res_pid1.binary_path, "/bin/init");

    // 32-bit signed max: 2,147,483,647
    let res_max = ReaInspector::inspect(2_147_483_647, None).expect("Max PID failed");
    assert_eq!(res_max.target_pid, 2_147_483_647);
}

#[test]
fn test_rea_inspect_invalid_pids_rejected() {
    // PID 0 rejected
    let err_zero = ReaInspector::inspect(0, None);
    assert!(err_zero.is_err());
    match err_zero.unwrap_err() {
        ActionError::InvalidPid(pid) => assert_eq!(pid, 0),
        other => panic!("Expected InvalidPid error, got {:?}", other),
    }

    // Negative PID rejected
    let err_neg = ReaInspector::inspect(-10, None);
    assert!(err_neg.is_err());
    match err_neg.unwrap_err() {
        ActionError::InvalidPid(pid) => assert_eq!(pid, -10),
        other => panic!("Expected InvalidPid error, got {:?}", other),
    }
}

#[test]
fn test_rea_inspect_binary_paths() {
    // Empty binary path falls back to default
    let res_empty = ReaInspector::inspect(500, Some("")).expect("Empty path fallback failed");
    assert_eq!(res_empty.binary_path, "/usr/bin/mock-app");

    // Whitespace binary path falls back to default
    let res_ws = ReaInspector::inspect(500, Some("   ")).expect("Whitespace path fallback failed");
    assert_eq!(res_ws.binary_path, "/usr/bin/mock-app");

    // Binary path with spaces and quotes preserved
    let win_path = r#"C:\Program Files\My App\app.exe"#;
    let res_win = ReaInspector::inspect(500, Some(win_path)).expect("Windows path failed");
    assert_eq!(res_win.binary_path, win_path);
}

#[test]
fn test_rea_to_recursive_task_spec() {
    let schema = ReaInspector::inspect(42, Some("/usr/bin/calc")).expect("Inspect failed");
    let spec = ReaInspector::to_recursive_spec(&schema, 2);

    assert_eq!(spec.rea_version, "1.0");
    assert_eq!(spec.target_pid, 42);
    assert_eq!(spec.binary_path, "/usr/bin/calc");
    assert_eq!(spec.recursive_depth, 2);
    assert_eq!(spec.status, "SchemaExtracted");
    assert!(spec.task_id.starts_with("rea-spec-"));
}
