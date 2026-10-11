use crate::error::{Result, SttError};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// Computes the SHA-256 checksum of a file on disk using streaming 64KB blocks.
pub fn compute_file_sha256(file_path: &Path) -> Result<String> {
    if !file_path.exists() {
        return Err(SttError::ModelNotFound(file_path.to_path_buf()));
    }

    let file = File::open(file_path)?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

/// Verifies that a file on disk matches the expected SHA-256 checksum.
pub fn verify_model_file(file_path: &Path, expected_sha256: &str) -> Result<bool> {
    let actual_sha256 = compute_file_sha256(file_path)?;
    if actual_sha256.eq_ignore_ascii_case(expected_sha256) {
        Ok(true)
    } else {
        Err(SttError::ModelCorrupted {
            path: file_path.to_path_buf(),
            expected: expected_sha256.to_string(),
            actual: actual_sha256,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_compute_and_verify_sha256() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test_model.bin");

        let content = b"whisper-model-dummy-content-12345";
        let mut file = File::create(&file_path).unwrap();
        file.write_all(content).unwrap();

        let sha = compute_file_sha256(&file_path).unwrap();
        assert!(!sha.is_empty());

        assert!(verify_model_file(&file_path, &sha).unwrap());
        assert!(verify_model_file(&file_path, "0000000000000000").is_err());
    }

    #[test]
    fn test_compute_missing_file_error() {
        let missing = Path::new("/non/existent/model.bin");
        assert!(matches!(
            compute_file_sha256(missing),
            Err(SttError::ModelNotFound(_))
        ));
    }
}
