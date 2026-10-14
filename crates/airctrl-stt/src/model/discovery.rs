use crate::error::{Result, SttError};
use crate::model::integrity::{compute_file_sha256, verify_model_file};
use crate::types::{ModelFormat, ModelInfo, WhisperModelVariant};
use std::borrow::Borrow;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Resolves standard model search directories in precedence order:
/// 1. `AIRCTRL_MODEL_DIR` environment variable
/// 2. OS Application Data Directory (`{app_data_dir}/models/`)
/// 3. HuggingFace Hub Cache (`~/.cache/huggingface/hub/`)
/// 4. Local Workspace Fallback (`./models`)
pub fn get_search_directories() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // Priority 1: Environment override
    if let Ok(dir) = std::env::var("AIRCTRL_MODEL_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            dirs.push(PathBuf::from(trimmed));
        }
    }

    // Priority 2: OS Application Data Directory
    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            dirs.push(PathBuf::from(local_app_data).join("airctrl").join("models"));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
            dirs.push(PathBuf::from(xdg_data).join("airctrl").join("models"));
        }
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(&home).join(".local").join("share").join("airctrl").join("models"));
            if let Ok(xdg_cache) = std::env::var("XDG_CACHE_HOME") {
                dirs.push(PathBuf::from(xdg_cache).join("airctrl").join("models"));
            } else {
                dirs.push(PathBuf::from(&home).join(".cache").join("airctrl").join("models"));
            }
        }
    }

    // Priority 3: HuggingFace Hub Cache
    if let Ok(hf_home) = std::env::var("HF_HOME") {
        dirs.push(PathBuf::from(hf_home).join("hub"));
    } else if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(home).join(".cache").join("huggingface").join("hub"));
    }

    // Priority 4: Local Workspace Fallback
    dirs.push(PathBuf::from("./models"));

    // Deduplicate preserving priority order
    let mut unique = Vec::new();
    for d in dirs {
        if !unique.contains(&d) {
            unique.push(d);
        }
    }
    unique
}

/// Backwards-compatible resolver for default model directory.
pub fn get_default_model_dir() -> PathBuf {
    get_search_directories()
        .into_iter()
        .next()
        .unwrap_or_else(|| PathBuf::from("./models"))
}

/// Discovers, validates, and manages offline speech-to-text models across directories.
pub struct ModelManager {
    search_dirs: Vec<PathBuf>,
}

impl Default for ModelManager {
    fn default() -> Self {
        Self::default_dirs()
    }
}

impl ModelManager {
    pub fn new(model_dir: PathBuf) -> Self {
        Self {
            search_dirs: vec![model_dir],
        }
    }

    pub fn with_search_dirs(search_dirs: Vec<PathBuf>) -> Self {
        Self { search_dirs }
    }

    pub fn default_dirs() -> Self {
        Self::with_search_dirs(get_search_directories())
    }

    pub fn default_dir() -> Self {
        Self::default_dirs()
    }

    pub fn search_dirs(&self) -> &[PathBuf] {
        &self.search_dirs
    }

    pub fn model_dir(&self) -> &Path {
        self.search_dirs
            .first()
            .map(|p| p.as_path())
            .unwrap_or_else(|| Path::new("./models"))
    }

    /// Recursively scans search directories for `.bin` (GGML) and `.gguf` (GGUF) models.
    pub fn discover_models(&self) -> Result<Vec<ModelInfo>> {
        let mut discovered = Vec::new();
        let mut seen_canonical_paths = HashSet::new();

        for base_dir in &self.search_dirs {
            if !base_dir.exists() {
                continue;
            }

            let is_hf_hub = base_dir.to_string_lossy().contains("huggingface");
            let max_depth = if is_hf_hub { 4 } else { 2 };

            let mut candidate_paths = Vec::new();
            scan_dir_recursive(base_dir, 0, max_depth, &mut candidate_paths);

            for path in candidate_paths {
                let canonical = match path.canonicalize() {
                    Ok(c) => c,
                    Err(_) => path.clone(),
                };

                if seen_canonical_paths.contains(&canonical) {
                    continue;
                }

                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if let Some(format) = ModelFormat::from_extension(ext) {
                        let metadata = match std::fs::metadata(&path) {
                            Ok(m) if m.is_file() && m.len() > 0 => m,
                            _ => continue,
                        };

                        let filename = path.file_name().unwrap_or_default().to_string_lossy();
                        let variant = WhisperModelVariant::from_filename(&filename);

                        let sha = compute_file_sha256(&path).unwrap_or_default();
                        let is_verified = if let Some(exp) = variant.official_sha256() {
                            exp.eq_ignore_ascii_case(&sha)
                        } else {
                            true
                        };

                        seen_canonical_paths.insert(canonical);
                        discovered.push(ModelInfo {
                            variant,
                            file_path: path,
                            size_bytes: metadata.len(),
                            format,
                            sha256: sha,
                            is_verified,
                        });
                    }
                }
            }
        }

        Ok(discovered)
    }

    /// Resolves the filesystem path for a designated model variant.
    pub fn resolve_model<V: Borrow<WhisperModelVariant>>(&self, variant: V) -> Result<PathBuf> {
        let target = variant.borrow();
        let models = self.discover_models()?;
        for model in models {
            if &model.variant == target {
                return Ok(model.file_path);
            }
        }

        // Direct path check in search dirs
        for dir in &self.search_dirs {
            let direct = dir.join(target.default_filename());
            if direct.exists() && direct.is_file() {
                return Ok(direct);
            }
        }

        Err(SttError::ModelNotFound(PathBuf::from(target.default_filename())))
    }

    /// Verifies model checksum integrity against official known hash.
    pub fn verify_model<V: Borrow<WhisperModelVariant>>(&self, variant: V) -> Result<bool> {
        let target = variant.borrow();
        let path = self.resolve_model(target)?;
        if let Some(expected_sha) = target.official_sha256() {
            verify_model_file(&path, expected_sha)
        } else {
            Ok(true)
        }
    }
}

/// Helper function to traverse directory hierarchies up to max_depth.
fn scan_dir_recursive(dir: &Path, depth: usize, max_depth: usize, out: &mut Vec<PathBuf>) {
    if depth > max_depth {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext_lower = ext.to_ascii_lowercase();
                if ext_lower == "bin" || ext_lower == "gguf" {
                    out.push(path);
                }
            }
        } else if path.is_dir() {
            // Avoid descending into hidden or git directories
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if !name.starts_with('.') {
                    scan_dir_recursive(&path, depth + 1, max_depth, out);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_model_manager_discovery_and_resolve() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mgr = ModelManager::new(temp_dir.path().to_path_buf());

        // Initially no models
        let models = mgr.discover_models().unwrap();
        assert!(models.is_empty());

        // Create a model file
        let tiny_path = temp_dir.path().join("whisper-tiny.en.bin");
        let mut file = File::create(&tiny_path).unwrap();
        file.write_all(b"model data bytes").unwrap();

        let models = mgr.discover_models().unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].variant, WhisperModelVariant::TinyEn);
        assert_eq!(models[0].file_path, tiny_path);
        assert_eq!(models[0].format, ModelFormat::GgmlBin);

        let resolved = mgr.resolve_model(WhisperModelVariant::TinyEn).unwrap();
        assert_eq!(resolved, tiny_path);

        assert!(mgr.resolve_model(WhisperModelVariant::BaseEn).is_err());
    }

    #[test]
    fn test_model_manager_gguf_discovery() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mgr = ModelManager::new(temp_dir.path().to_path_buf());

        let parakeet_path = temp_dir.path().join("parakeet-unified-en-0.6b-Q8_0.gguf");
        let mut file = File::create(&parakeet_path).unwrap();
        file.write_all(b"GGUFdummydata").unwrap();

        let models = mgr.discover_models().unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].variant, WhisperModelVariant::ParakeetV3);
        assert_eq!(models[0].format, ModelFormat::Gguf);
    }
}
