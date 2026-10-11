pub mod handle;
#[allow(clippy::module_inception)]
pub mod watcher;

pub use handle::WatcherHandle;
pub use watcher::MeetingWatcher;
