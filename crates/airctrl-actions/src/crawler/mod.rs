pub mod catalog;
pub mod context_map;
#[allow(clippy::module_inception)]
pub mod crawler;
pub mod matching;

pub use catalog::{catalog_installed_applications, launch_application};
pub use context_map::{CrawlerConfig, WindowContextMap};
pub use crawler::{crawl_running_applications, WindowProcessCrawler};
pub use matching::WindowMatcher;
