pub mod linux;
pub mod mock;
pub mod traits;
pub mod windows;

pub use mock::{
    create_mock_platform, MockAudioOps, MockCameraOps, MockInputOps, MockPhysicalCameraStream,
    MockPlatformControllers, MockVirtualCameraSink, MockWindowOps,
};
pub use traits::{
    PhysicalCameraStream, PlatformAudioOps, PlatformCameraOps, PlatformInputOps, PlatformProvider,
    PlatformWindowOps, VirtualCameraDeviceSink,
};

use std::sync::Arc;

/// Instantiates a `PlatformProvider`. If `use_mock` is true or if compiling on an unsupported
/// platform, the mock provider is returned.
pub fn create_platform_provider(use_mock: bool) -> PlatformProvider {
    if use_mock {
        let (provider, _) = create_mock_platform();
        return provider;
    }

    #[cfg(target_os = "windows")]
    {
        PlatformProvider {
            window: Arc::new(windows::WindowsWindowOps),
            input: Arc::new(windows::WindowsInputOps),
            audio: Arc::new(windows::WindowsAudioOps),
            camera: Arc::new(mock::MockCameraOps::new()),
        }
    }

    #[cfg(target_os = "linux")]
    {
        PlatformProvider {
            window: Arc::new(linux::LinuxWindowOps),
            input: Arc::new(linux::LinuxInputOps),
            audio: Arc::new(linux::LinuxAudioOps),
            camera: Arc::new(mock::MockCameraOps::new()),
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let (provider, _) = create_mock_platform();
        provider
    }
}

