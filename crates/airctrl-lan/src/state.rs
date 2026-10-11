use sha2::{Digest, Sha256};
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::time::{Instant, SystemTime};
use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub struct ArtifactCache {
    pub mtime: Option<SystemTime>,
    pub sha256: String,
}

pub struct LanState {
    pub advertised_ip: Ipv4Addr,
    pub advertised_port: u16,
    pub artifact_path: PathBuf,
    pub version: String,
    pub start_time: Instant,
    pub build_timestamp: String,
    pub cached_artifact: RwLock<Option<ArtifactCache>>,
}

impl LanState {
    pub fn new(
        advertised_ip: Ipv4Addr,
        advertised_port: u16,
        artifact_path: PathBuf,
        version: String,
    ) -> Self {
        Self {
            advertised_ip,
            advertised_port,
            artifact_path,
            version,
            start_time: Instant::now(),
            build_timestamp: "2026-10-11T07:20:00Z".to_string(),
            cached_artifact: RwLock::new(None),
        }
    }

    /// Inspects the artifact path on disk.
    /// Returns (is_available, file_size, sha256_hash).
    /// Enforces that the artifact must be a regular file (`metadata.is_file()`).
    /// If the path is a directory or non-regular file, returns (false, None, None).
    pub async fn inspect_artifact(&self) -> (bool, Option<u64>, Option<String>) {
        let metadata = match tokio::fs::metadata(&self.artifact_path).await {
            Ok(m) if m.is_file() => m,
            _ => return (false, None, None),
        };

        let current_mtime = metadata.modified().ok();
        let file_size = metadata.len();

        {
            let guard = self.cached_artifact.read().await;
            if let Some(ref cache) = *guard {
                if cache.mtime == current_mtime && current_mtime.is_some() {
                    return (true, Some(file_size), Some(cache.sha256.clone()));
                }
            }
        }

        // Recompute hash if modified or not yet cached
        match tokio::fs::read(&self.artifact_path).await {
            Ok(bytes) => {
                let mut hasher = Sha256::new();
                hasher.update(&bytes);
                let hash = hex::encode(hasher.finalize());

                let mut guard = self.cached_artifact.write().await;
                *guard = Some(ArtifactCache {
                    mtime: current_mtime,
                    sha256: hash.clone(),
                });

                (true, Some(file_size), Some(hash))
            }
            Err(_) => (false, None, None),
        }
    }
}
