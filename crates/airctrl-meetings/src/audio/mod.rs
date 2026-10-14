pub mod correlator;
pub mod energy;
pub mod hangover;
pub mod mixer;
pub mod monitor;

pub use correlator::{AudioProcessCorrelator, CorrelatedState};
pub use energy::{compute_dbfs, compute_peak, compute_rms};
pub use hangover::{HangoverStateMachine, SpeechState};
pub use mixer::{DualAudioMixer, MixerConfig};
pub use monitor::{AudioStreamMetrics, AudioStreamMonitor};

