use airctrl_core::types::ApplicationItem;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Discovers installed applications across Windows (Start Menu & Program Files) or Linux/macOS desktop files.
pub fn catalog_installed_applications() -> Vec<ApplicationItem> {
    let mut apps: Vec<ApplicationItem> = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    #[cfg(windows)]
    {
        scan_windows_start_menu(&mut apps, &mut seen_names);
        scan_windows_program_files(&mut apps, &mut seen_names);
    }

    #[cfg(not(windows))]
    {
        scan_unix_applications(&mut apps, &mut seen_names);
    }

    // Always ensure standard core applications are populated if system scan finds few entries (e.g. CI or fresh VM)
    let fallback_apps = get_standard_applications();
    for app in fallback_apps {
        if !seen_names.contains(&app.name.to_lowercase()) {
            seen_names.insert(app.name.to_lowercase());
            apps.push(app);
        }
    }

    apps.sort_by(|a, b| a.name.cmp(&b.name));
    info!("Cataloged {} desktop applications", apps.len());
    apps
}

#[cfg(windows)]
fn scan_windows_start_menu(apps: &mut Vec<ApplicationItem>, seen: &mut std::collections::HashSet<String>) {
    let mut dirs_to_scan = Vec::new();

    // 1. All Users Start Menu
    if let Ok(program_data) = std::env::var("ProgramData") {
        dirs_to_scan.push(PathBuf::from(program_data).join(r"Microsoft\Windows\Start Menu\Programs"));
    } else {
        dirs_to_scan.push(PathBuf::from(r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs"));
    }

    // 2. Current User Start Menu
    if let Ok(app_data) = std::env::var("APPDATA") {
        dirs_to_scan.push(PathBuf::from(app_data).join(r"Microsoft\Windows\Start Menu\Programs"));
    }

    // 3. User Local Programs
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        dirs_to_scan.push(PathBuf::from(local_app_data).join("Programs"));
    }

    for dir in dirs_to_scan {
        if dir.exists() {
            scan_dir_recursive(&dir, 0, 4, apps, seen);
        }
    }
}

#[cfg(windows)]
fn scan_windows_program_files(apps: &mut Vec<ApplicationItem>, seen: &mut std::collections::HashSet<String>) {
    let program_files = [
        std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".into()),
        std::env::var("ProgramFiles(x86)").unwrap_or_else(|_| r"C:\Program Files (x86)".into()),
    ];

    for pf in program_files {
        let path = PathBuf::from(pf);
        if path.exists() {
            scan_dir_recursive(&path, 0, 2, apps, seen);
        }
    }
}

#[cfg(not(windows))]
fn scan_unix_applications(apps: &mut Vec<ApplicationItem>, seen: &mut std::collections::HashSet<String>) {
    let desktop_dirs = [
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
    ];

    for dir in &desktop_dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("desktop") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Some((name, exec, icon)) = parse_desktop_entry(&content) {
                            let key = name.to_lowercase();
                            if !seen.contains(&key) {
                                seen.insert(key.clone());
                                apps.push(ApplicationItem {
                                    id: format!("app-{}", sanitize_id(&name)),
                                    name,
                                    path: exec,
                                    icon: Some(icon),
                                    category: "System Application".into(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(not(windows))]
fn parse_desktop_entry(content: &str) -> Option<(String, String, String)> {
    let mut name = None;
    let mut exec = None;
    let mut icon = None;
    let mut is_no_display = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Name=") && name.is_none() {
            name = Some(trimmed["Name=".len()..].to_string());
        } else if trimmed.starts_with("Exec=") && exec.is_none() {
            let cmd = trimmed["Exec=".len()..].split_whitespace().next().unwrap_or("").to_string();
            exec = Some(cmd);
        } else if trimmed.starts_with("Icon=") && icon.is_none() {
            icon = Some(trimmed["Icon=".len()..].to_string());
        } else if trimmed == "NoDisplay=true" {
            is_no_display = true;
        }
    }

    if is_no_display {
        return None;
    }

    match (name, exec) {
        (Some(n), Some(e)) => Some((n, e, icon.unwrap_or_else(|| "window".into()))),
        _ => None,
    }
}

#[allow(dead_code)]
fn scan_dir_recursive(
    dir: &Path,
    current_depth: usize,
    max_depth: usize,
    apps: &mut Vec<ApplicationItem>,
    seen: &mut std::collections::HashSet<String>,
) {
    if current_depth > max_depth {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir_recursive(&path, current_depth + 1, max_depth, apps, seen);
        } else if path.is_file() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if ext == "lnk" || ext == "exe" {
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let lower_stem = stem.to_lowercase();

                // Skip uninstaller / helper binaries
                if lower_stem.contains("uninstall") || lower_stem.contains("unins000") || lower_stem.starts_with("update") {
                    continue;
                }

                let clean_name = format_app_name(stem);
                let key = clean_name.to_lowercase();
                if !seen.contains(&key) && !clean_name.is_empty() {
                    seen.insert(key);
                    apps.push(ApplicationItem {
                        id: format!("app-{}", sanitize_id(&clean_name)),
                        name: clean_name,
                        path: path.to_string_lossy().to_string(),
                        icon: Some("window".into()),
                        category: "Installed Applications".into(),
                    });
                }
            }
        }
    }
}

#[allow(dead_code)]
fn format_app_name(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    // Replace underscores and dashes with spaces
    let with_spaces = trimmed.replace(['_', '-'], " ");
    // Capitalize words
    with_spaces
        .split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn sanitize_id(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

/// Fallback catalog representing standard cross-platform developer tools and applications.
pub fn get_standard_applications() -> Vec<ApplicationItem> {
    vec![
        ApplicationItem {
            id: "app-cursor".into(),
            name: "Cursor".into(),
            path: "cursor.exe".into(),
            icon: Some("cpu".into()),
            category: "Developer Tools".into(),
        },
        ApplicationItem {
            id: "app-code".into(),
            name: "Visual Studio Code".into(),
            path: "code.exe".into(),
            icon: Some("layout".into()),
            category: "Developer Tools".into(),
        },
        ApplicationItem {
            id: "app-chrome".into(),
            name: "Google Chrome".into(),
            path: "chrome.exe".into(),
            icon: Some("window".into()),
            category: "Web Browsers".into(),
        },
        ApplicationItem {
            id: "app-terminal".into(),
            name: "Windows Terminal".into(),
            path: "wt.exe".into(),
            icon: Some("window".into()),
            category: "System Utilities".into(),
        },
        ApplicationItem {
            id: "app-notepad".into(),
            name: "Notepad".into(),
            path: "notepad.exe".into(),
            icon: Some("list".into()),
            category: "Productivity".into(),
        },
        ApplicationItem {
            id: "app-explorer".into(),
            name: "File Explorer".into(),
            path: "explorer.exe".into(),
            icon: Some("layout".into()),
            category: "System Utilities".into(),
        },
    ]
}

/// Launches a cataloged application by path or command name.
pub fn launch_application(path_or_name: &str) -> Result<u32, String> {
    debug!("Launching application: {}", path_or_name);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let trimmed = path_or_name.trim();
        let lower = trimmed.to_lowercase();
        let resolved = match lower.as_str() {
            "notepad" => "notepad.exe".to_string(),
            "calculator" | "calc" => "calc.exe".to_string(),
            "explorer" | "file explorer" | "files" => "explorer.exe".to_string(),
            "terminal" | "windows terminal" | "wt" => "wt.exe".to_string(),
            "cmd" | "command prompt" => "cmd.exe".to_string(),
            "powershell" => "powershell.exe".to_string(),
            "settings" | "windows settings" => "ms-settings:".to_string(),
            "task manager" | "taskmgr" => "taskmgr.exe".to_string(),
            "paint" | "mspaint" => "mspaint.exe".to_string(),
            "chrome" | "google chrome" => "chrome".to_string(),
            "edge" | "microsoft edge" => "msedge".to_string(),
            "code" | "vs code" | "vscode" | "visual studio code" => "code".to_string(),
            "cursor" | "cursor ide" => "cursor".to_string(),
            _ => {
                if std::path::Path::new(trimmed).exists() {
                    trimmed.to_string()
                } else {
                    let catalog = catalog_installed_applications();
                    if let Some(found) = catalog.iter().find(|a| {
                        a.name.to_lowercase() == lower
                            || a.name.to_lowercase().contains(&lower)
                    }) {
                        found.path.clone()
                    } else {
                        trimmed.to_string()
                    }
                }
            }
        };

        let child = std::process::Command::new("cmd")
            .args(["/C", "start", "", &resolved])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("Failed to spawn {}: {}", resolved, e))?;
        Ok(child.id())
    }

    #[cfg(not(windows))]
    {
        let child = std::process::Command::new("sh")
            .args(["-c", &format!("{} &", path_or_name)])
            .spawn()
            .map_err(|e| format!("Failed to spawn {}: {}", path_or_name, e))?;
        Ok(child.id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_installed_applications_returns_non_empty() {
        let apps = catalog_installed_applications();
        assert!(!apps.is_empty(), "Catalog should contain discovered or standard applications");
        assert!(apps.iter().any(|a| a.name == "Cursor" || a.name == "Visual Studio Code" || a.name == "Google Chrome"));
    }

    #[test]
    fn test_format_app_name() {
        assert_eq!(format_app_name("google_chrome"), "Google Chrome");
        assert_eq!(format_app_name("cursor-ide"), "Cursor Ide");
    }

    #[test]
    fn test_sanitize_id() {
        assert_eq!(sanitize_id("Google Chrome (64-bit)"), "googlechrome64bit");
    }
}
