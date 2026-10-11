use airctrl_stt::{
    compute_file_sha256, get_default_model_dir, verify_model_file, ModelManager, SttError,
    WhisperModelVariant,
};
use std::fs::File;
use std::io::Write;

#[test]
fn test_model_manager_discovery_and_resolution() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mgr = ModelManager::new(temp_dir.path().to_path_buf());

    // Initially empty
    let discovered = mgr.discover_models().unwrap();
    assert!(discovered.is_empty());

    // Create tiny.en model
    let tiny_filename = WhisperModelVariant::TinyEn.default_filename();
    let tiny_path = temp_dir.path().join(tiny_filename);
    let mut f = File::create(&tiny_path).unwrap();
    f.write_all(b"sample model weights content").unwrap();

    let discovered = mgr.discover_models().unwrap();
    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].variant, WhisperModelVariant::TinyEn);
    assert_eq!(discovered[0].file_path, tiny_path);
    assert!(!discovered[0].sha256.is_empty());

    // Resolve existing
    let resolved = mgr.resolve_model(WhisperModelVariant::TinyEn).unwrap();
    assert_eq!(resolved, tiny_path);

    // Resolve missing variant
    let missing = mgr.resolve_model(WhisperModelVariant::BaseEn);
    assert!(matches!(missing, Err(SttError::ModelNotFound(_))));
}

#[test]
fn test_streaming_sha256_large_file_and_corruption() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("large_model.bin");

    // Write > 128KB to exercise multi-block streaming
    let data = vec![0x42u8; 150 * 1024];
    let mut f = File::create(&file_path).unwrap();
    f.write_all(&data).unwrap();

    let hash = compute_file_sha256(&file_path).unwrap();
    assert_eq!(hash.len(), 64); // 64 hex characters

    // Correct hash matches
    assert!(verify_model_file(&file_path, &hash).unwrap());

    // Corrupted hash fails
    let corrupt_err = verify_model_file(&file_path, "deadbeefdeadbeef").unwrap_err();
    assert!(matches!(corrupt_err, SttError::ModelCorrupted { .. }));
}

#[test]
fn test_get_default_model_dir() {
    // Test env override
    std::env::set_var("AIRCTRL_MODEL_DIR", "/custom/model/path");
    let dir = get_default_model_dir();
    assert_eq!(dir.to_str().unwrap(), "/custom/model/path");
    std::env::remove_var("AIRCTRL_MODEL_DIR");
}
