pub mod discovery;
pub mod integrity;

pub use discovery::{get_default_model_dir, get_search_directories, ModelManager};
pub use integrity::{compute_file_sha256, verify_model_file};
