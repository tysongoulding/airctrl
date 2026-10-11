pub mod context_map;
#[allow(clippy::module_inception)]
pub mod crawler;
pub mod matching;

pub use context_map::{CrawlerConfig, WindowContextMap};
pub use crawler::{crawl_running_applications, WindowProcessCrawler};
pub use matching::WindowMatcher;
