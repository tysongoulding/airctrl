use crate::error::{Result, SttError};
use crate::model::integrity::{compute_file_sha256, verify_model_file};
use crate::types::{ModelInfo, WhisperModelVariant};
use std::path::{Path, PathBuf};

/// Resolves the default directory where local Whisper models are stored.
///
/// Priority:
/// 1. `AIRCTRL_MODEL_DIR` environment variable
/// 2. Windows: `%LOCALAPPDATA%/airctrl/models`
/// 3. Linux/Unix: `$XDG_CACHE_HOME/airctrl/models` or `~/.cache/airctrl/models`
/// 4. Fallback: `./models`
pub fn get_default_model_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("AIRCTRL_MODEL_DIR") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir);
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(local_app_data).join("airctrl").join("models");
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(xdg_cache) = std::env::var("XDG_CACHE_HOME") {
            return PathBuf::from(xdg_cache).join("airctrl").join("models");
        }
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home)
                .join(".cache")
                .join("airctrl")
                .join("models");
        }
    }

    PathBuf::from("./models")
}

/// Discovers, validates, and manages offline Whisper model weight files.
pub struct ModelManager {
    model_dir: PathBuf,
}

impl Default for ModelManager {
    fn default() -> Self {
        Self::default_dir()
    }
}

impl ModelManager {
    pub fn new(model_dir: PathBuf) -> Self {
        Self { model_dir }
    }

    pub fn default_dir() -> Self {
        Self::new(get_default_model_dir())
    }

    pub fn model_dir(&self) -> &Path {
        &self.model_dir
    }

    /// Discovers all available Whisper models in the model directory.
    pub fn discover_models(&self) -> Result<Vec<ModelInfo>> {
        let mut found = Vec::new();
        if !self.model_dir.exists() {
            return Ok(found);
        }

        let variants = [
            WhisperModelVariant::TinyEn,
            WhisperModelVariant::BaseEn,
            WhisperModelVariant::SmallEn,
        ];

        for variant in variants {
            let path = self.model_dir.join(variant.default_filename());
            if path.exists() && path.is_file() {
                let metadata = std::fs::metadata(&path)?;
                if metadata.len() > 0 {
                    let sha = compute_file_sha256(&path).unwrap_or_default();
                    let is_verified = variant
                        .official_sha256()
                        .map(|exp| exp.eq_ignore_ascii_case(&sha))
                        .unwrap_or(false);

                    found.push(ModelInfo {
                        variant,
                        file_path: path,
                        size_bytes: metadata.len(),
                        sha256: sha,
                        is_verified,
                    });
                }
            }
        }

        Ok(found)
    }

    /// Resolves the filesystem path for a specific Whisper model variant.
    pub fn resolve_model(&self, variant: WhisperModelVariant) -> Result<PathBuf> {
        let path = self.model_dir.join(variant.default_filename());
        if path.exists() {
            Ok(path)
        } else {
            Err(SttError::ModelNotFound(path))
        }
    }

    /// Verifies the integrity of a discovered model against its official checksum.
    pub fn verify_model(&self, variant: WhisperModelVariant) -> Result<bool> {
        let path = self.resolve_model(variant)?;
        if let Some(expected_sha) = variant.official_sha256() {
            verify_model_file(&path, expected_sha)
        } else {
            Ok(true)
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

        let resolved = mgr.resolve_model(WhisperModelVariant::TinyEn).unwrap();
        assert_eq!(resolved, tiny_path);

        assert!(mgr.resolve_model(WhisperModelVariant::BaseEn).is_err());
    }
}
