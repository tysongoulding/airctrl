use airctrl_lan::generate_run_ps1;
use std::process::Command;

#[test]
fn test_run_ps1_lexical_balance_and_required_tokens() {
    let script = generate_run_ps1("192.168.144.110", 8080);

    // 1. Verify delimiter balance (braces, brackets, parentheses)
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
                    let top = stack.pop().expect("Unmatched closing brace }");
                    assert_eq!(top.0, '{', "Mismatched brace pair at char {}", i);
                }
                ')' => {
                    let top = stack.pop().expect("Unmatched closing paren )");
                    assert_eq!(top.0, '(', "Mismatched paren pair at char {}", i);
                }
                ']' => {
                    let top = stack.pop().expect("Unmatched closing bracket ]");
                    assert_eq!(top.0, '[', "Mismatched bracket pair at char {}", i);
                }
                _ => {}
            }
        }
        i += 1;
    }

    assert!(stack.is_empty(), "Unclosed delimiters remaining: {:?}", stack);

    // 2. Verify all placeholders replaced
    assert!(!script.contains("{{HOST_IP}}"), "Unresolved {{HOST_IP}}");
    assert!(!script.contains("{{PORT}}"), "Unresolved {{PORT}}");

    // 3. Verify essential PowerShell control flow and cmdlets
    let mandatory_tokens = [
        "$ErrorActionPreference = 'Stop'",
        "Invoke-RestMethod",
        "Invoke-WebRequest",
        "Stop-Process",
        "Start-Process",
        "Get-FileHash",
        "Test-Path",
        "New-Item",
        "try {",
        "} catch {",
        "$env:LOCALAPPDATA",
    ];

    for token in mandatory_tokens {
        assert!(
            script.contains(token),
            "Generated run.ps1 missing mandatory token: '{}'",
            token
        );
    }
}

#[test]
fn test_run_ps1_pwsh_ast_validation_if_available() {
    // If PowerShell Core (pwsh) is present on system, run AST parser
    if let Ok(output) = Command::new("pwsh").arg("-v").output() {
        if output.status.success() {
            let script = generate_run_ps1("192.168.144.110", 8080);
            let check_cmd = r#"
                $content = [Console]::In.ReadToEnd()
                $errors = $null
                $tokens = $null
                [System.Management.Automation.Language.Parser]::ParseInput($content, [ref]$tokens, [ref]$errors) | Out-Null
                if ($errors.Count -gt 0) {
                    $errors | ForEach-Object { Write-Error $_ }
                    exit 1
                }
            "#;
            let mut child = Command::new("pwsh")
                .args(["-NoProfile", "-Command", check_cmd])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .expect("Failed to spawn pwsh");

            use std::io::Write;
            child.stdin.as_mut().unwrap().write_all(script.as_bytes()).unwrap();
            let exit_status = child.wait_with_output().unwrap();
            assert!(
                exit_status.status.success(),
                "PowerShell AST Parser failed: {}",
                String::from_utf8_lossy(&exit_status.stderr)
            );
        }
    }
}
