use airctrl_lan::generate_run_ps1;

/// Helper function to check delimiter balance in generated PowerShell script
fn check_delimiter_balance(script: &str) -> Result<(), String> {
    let mut stack = Vec::new();
    let chars: Vec<char> = script.chars().collect();
    let mut i = 0;
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while i < chars.len() {
        let c = chars[i];
        if in_single_quote {
            if c == '\'' {
                in_single_quote = false;
            }
        } else if in_double_quote {
            if c == '`' {
                i += 2; // skip escaped character
                continue;
            }
            if c == '"' {
                in_double_quote = false;
            }
        } else {
            match c {
                '\'' => in_single_quote = true,
                '"' => in_double_quote = true,
                '{' | '(' | '[' => stack.push((c, i)),
                '}' => {
                    let top = stack.pop().ok_or_else(|| format!("Unmatched closing brace }} at {}", i))?;
                    if top.0 != '{' {
                        return Err(format!("Mismatched brace pair at char {}", i));
                    }
                }
                ')' => {
                    let top = stack.pop().ok_or_else(|| format!("Unmatched closing paren ) at {}", i))?;
                    if top.0 != '(' {
                        return Err(format!("Mismatched paren pair at char {}", i));
                    }
                }
                ']' => {
                    let top = stack.pop().ok_or_else(|| format!("Unmatched closing bracket ] at {}", i))?;
                    if top.0 != '[' {
                        return Err(format!("Mismatched bracket pair at char {}", i));
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }

    if in_single_quote {
        return Err("Unclosed single quote".to_string());
    }
    if in_double_quote {
        return Err("Unclosed double quote".to_string());
    }
    if !stack.is_empty() {
        return Err(format!("Unclosed delimiters remaining: {:?}", stack));
    }
    Ok(())
}

#[test]
fn test_powershell_fuzz_standard_and_edge_inputs() {
    let edge_cases = [
        ("127.0.0.1", 80),
        ("0.0.0.0", 8080),
        ("192.168.1.254", 65535),
        ("10.0.0.1", 1),
        ("localhost", 3000),
        ("windows-build-vm.internal.corp", 8080),
    ];

    for (host, port) in edge_cases {
        let script = generate_run_ps1(host, port);
        assert!(!script.contains("{{HOST_IP}}"));
        assert!(!script.contains("{{PORT}}"));
        assert!(script.contains(&format!("$Port = {}", port)));
        assert!(check_delimiter_balance(&script).is_ok());
    }
}

#[test]
fn test_powershell_fuzz_injection_attack_vectors() {
    // Attack payloads attempting to break out of string context or inject PowerShell commands
    let malicious_hosts = [
        // 1. Double quote breakout
        r#"192.168.1.1" ; Write-Host "PWNED" ; $dummy = ""#,
        // 2. PowerShell subexpression evaluation in double-quoted string
        r#"$(Stop-Process -Name svchost -Force)"#,
        // 3. PowerShell variable interpolation
        r#"$env:COMPUTERNAME"#,
        // 4. Backtick escaping
        r#"192.168.1.1`" ; Invoke-Expression 'calc.exe' ; `"#,
        // 5. Newline command injection
        "192.168.1.1\r\nInvoke-Expression 'calc.exe'\r\n#",
        // 6. Unbalanced single and double quotes
        r#"192.168.1.1""#,
        r#"192.168.1.1'"#,
        // 7. Unbalanced brackets and braces
        "192.168.1.1{{{(((",
        // 8. Null byte and control characters
        "192.168.1.1\0evil",
        // 9. IPv6 address without brackets
        "::1",
        "fe80::1ff:fe00:1",
    ];

    let mut vulnerabilities = Vec::new();

    for evil_host in malicious_hosts {
        let script = generate_run_ps1(evil_host, 8080);

        // Check if delimiters became unbalanced
        let balance_result = check_delimiter_balance(&script);

        // Check if malicious payload passed raw into double-quoted $HostIp
        let host_line = script
            .lines()
            .find(|line| line.starts_with("$HostIp = "))
            .unwrap_or("");

        let has_subexpression = evil_host.contains("$(") && host_line.contains("$(");
        let has_quote_breakout = evil_host.contains('"') && (balance_result.is_err() || host_line.matches('"').count() > 2);
        let has_raw_ipv6 = evil_host.contains(':') && !evil_host.starts_with('[') && script.contains(&format!("http://{}:8080", evil_host));

        if has_subexpression || has_quote_breakout || has_raw_ipv6 || balance_result.is_err() {
            vulnerabilities.push((
                evil_host,
                balance_result.err(),
                host_line.to_string(),
                has_subexpression,
                has_quote_breakout,
                has_raw_ipv6,
            ));
        }
    }

    println!("=== FUZZING VULNERABILITY REPORT ===");
    for (host, err, line, subexpr, breakout, ipv6) in &vulnerabilities {
        println!(
            "Payload: {:?}\n  Delimiter Error: {:?}\n  Line: {}\n  Subexpr Executable: {}\n  Quote Breakout: {}\n  Raw IPv6 URL: {}\n",
            host, err, line, subexpr, breakout, ipv6
        );
    }

    // Verify that all injection vectors are successfully neutralized
    assert!(
        vulnerabilities.is_empty(),
        "Expected 0 injection vulnerabilities after sanitization, but found: {:?}",
        vulnerabilities
    );
}
