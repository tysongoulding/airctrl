use crate::engine::{MockWhisperEngine, ParakeetV3CpuEngine, SpeechToTextEngine, WhisperCpuEngine};
use crate::error::Result;
use crate::types::{ModelFormat, ModelInfo, SttConfig, TranscriptionResult};
use parking_lot::RwLock;
use std::path::PathBuf;

pub enum InnerEngine {
    Whisper(WhisperCpuEngine),
    Parakeet(ParakeetV3CpuEngine),
    Mock(MockWhisperEngine),
}

/// Unified STT engine dispatching to Whisper CPU, Parakeet V3 CPU, or Mock engine,
/// with runtime model switching support.
pub struct UnifiedSttEngine {
    inner: RwLock<InnerEngine>,
}

impl UnifiedSttEngine {
    pub fn new(config: &SttConfig) -> Result<Self> {
        if config.use_mock {
            let mock = if let Some(path) = &config.model_path {
                MockWhisperEngine::with_path(path.clone())
            } else {
                MockWhisperEngine::new()
            };
            return Ok(Self {
                inner: RwLock::new(InnerEngine::Mock(mock)),
            });
        }

        let path = config.model_path.clone().unwrap_or_else(|| {
            PathBuf::from(config.model_variant.default_filename())
        });

        if config.model_variant.is_gguf() {
            let engine = ParakeetV3CpuEngine::load(path)?;
            Ok(Self {
                inner: RwLock::new(InnerEngine::Parakeet(engine)),
            })
        } else {
            let engine = WhisperCpuEngine::load(path, config.model_variant.clone())?;
            Ok(Self {
                inner: RwLock::new(InnerEngine::Whisper(engine)),
            })
        }
    }

    pub fn with_mock() -> Self {
        Self {
            inner: RwLock::new(InnerEngine::Mock(MockWhisperEngine::new())),
        }
    }

    /// Dynamically switches the active inference model at runtime.
    pub fn switch_model(&self, info: ModelInfo) -> Result<()> {
        let new_inner = if info.format == ModelFormat::Gguf {
            InnerEngine::Parakeet(ParakeetV3CpuEngine::load(info.file_path)?)
        } else {
            InnerEngine::Whisper(WhisperCpuEngine::load(info.file_path, info.variant)?)
        };

        *self.inner.write() = new_inner;
        Ok(())
    }

    /// Returns the currently active model format.
    pub fn current_format(&self) -> ModelFormat {
        match &*self.inner.read() {
            InnerEngine::Whisper(_) => ModelFormat::GgmlBin,
            InnerEngine::Parakeet(_) => ModelFormat::Gguf,
            InnerEngine::Mock(m) => m.model_info().map(|i| i.format).unwrap_or(ModelFormat::GgmlBin),
        }
    }
}

impl SpeechToTextEngine for UnifiedSttEngine {
    fn transcribe(&self, samples: &[f32]) -> Result<TranscriptionResult> {
        match &*self.inner.read() {
            InnerEngine::Whisper(e) => e.transcribe(samples),
            InnerEngine::Parakeet(e) => e.transcribe(samples),
            InnerEngine::Mock(e) => e.transcribe(samples),
        }
    }

    fn is_loaded(&self) -> bool {
        match &*self.inner.read() {
            InnerEngine::Whisper(e) => e.is_loaded(),
            InnerEngine::Parakeet(e) => e.is_loaded(),
            InnerEngine::Mock(e) => e.is_loaded(),
        }
    }

    fn model_info(&self) -> Option<ModelInfo> {
        match &*self.inner.read() {
            InnerEngine::Whisper(e) => e.model_info(),
            InnerEngine::Parakeet(e) => e.model_info(),
            InnerEngine::Mock(e) => e.model_info(),
        }
    }
}
