use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use crate::error::{MeetingError, Result};
use rodio::Source;

pub const SUPPORTED_AUDIO_EXTENSIONS: &[&str] = &["wav", "mp3", "m4a", "aac", "flac", "ogg"];

#[derive(Debug, Clone)]
pub struct DecodedAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_secs: u64,
    pub format: String,
}

pub struct AudioDecoder;

impl AudioDecoder {
    pub fn is_supported_extension(ext: &str) -> bool {
        let clean = ext.trim_start_matches('.').to_ascii_lowercase();
        SUPPORTED_AUDIO_EXTENSIONS.contains(&clean.as_str())
    }

    pub fn decode_file(file_path: &Path) -> Result<DecodedAudio> {
        if !file_path.exists() {
            return Err(MeetingError::AudioFileNotFound(file_path.display().to_string()));
        }

        let metadata = std::fs::metadata(file_path)
            .map_err(|e| MeetingError::AudioFileReadError(e.to_string()))?;
        if metadata.len() == 0 {
            return Err(MeetingError::EmptyAudioFile(file_path.display().to_string()));
        }

        let ext = file_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        if ext.is_empty() || !Self::is_supported_extension(&ext) {
            return Err(MeetingError::UnsupportedAudioFormat(format!(
                "Unsupported audio format: .{}",
                ext
            )));
        }

        match ext.as_str() {
            "wav" => Self::decode_wav(file_path),
            _ => Self::decode_rodio(file_path, &ext),
        }
    }

    fn decode_wav(file_path: &Path) -> Result<DecodedAudio> {
        let reader = hound::WavReader::open(file_path)
            .map_err(|e| MeetingError::AudioDecodeFailed(format!("WAV decode error: {e}")))?;
        let spec = reader.spec();
        let channels = spec.channels;
        let sample_rate = spec.sample_rate;

        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Int => match spec.bits_per_sample {
                16 => reader
                    .into_samples::<i16>()
                    .map(|s| s.map(|v| (v as f32 / 32768.0).clamp(-1.0, 1.0)))
                    .collect::<std::result::Result<Vec<f32>, _>>()
                    .map_err(|e| MeetingError::AudioDecodeFailed(e.to_string()))?,
                24 => reader
                    .into_samples::<i32>()
                    .map(|s| s.map(|v| (v as f32 / 8_388_608.0).clamp(-1.0, 1.0)))
                    .collect::<std::result::Result<Vec<f32>, _>>()
                    .map_err(|e| MeetingError::AudioDecodeFailed(e.to_string()))?,
                32 => reader
                    .into_samples::<i32>()
                    .map(|s| s.map(|v| (v as f32 / 2_147_483_648.0).clamp(-1.0, 1.0)))
                    .collect::<std::result::Result<Vec<f32>, _>>()
                    .map_err(|e| MeetingError::AudioDecodeFailed(e.to_string()))?,
                bits => {
                    return Err(MeetingError::UnsupportedAudioFormat(format!(
                        "Unsupported bit depth: {bits} bits"
                    )));
                }
            },
            hound::SampleFormat::Float => reader
                .into_samples::<f32>()
                .map(|s| s.map(|v| v.clamp(-1.0, 1.0)))
                .collect::<std::result::Result<Vec<f32>, _>>()
                .map_err(|e| MeetingError::AudioDecodeFailed(e.to_string()))?,
        };

        let total_frames = samples.len() / channels.max(1) as usize;
        let duration_secs = (total_frames as f64 / sample_rate as f64).round() as u64;

        Ok(DecodedAudio {
            samples,
            sample_rate,
            channels,
            duration_secs,
            format: "wav".to_string(),
        })
    }

    fn decode_rodio(file_path: &Path, ext: &str) -> Result<DecodedAudio> {
        let file = File::open(file_path)
            .map_err(|e| MeetingError::AudioFileReadError(e.to_string()))?;
        let reader = BufReader::new(file);

        let source = rodio::Decoder::new(reader)
            .map_err(|e| MeetingError::AudioDecodeFailed(format!("Failed to decode {ext}: {e}")))?;

        let channels = source.channels();
        let sample_rate = source.sample_rate();
        let samples: Vec<f32> = source.convert_samples::<f32>().collect();

        let total_frames = samples.len() / channels.max(1) as usize;
        let duration_secs = (total_frames as f64 / sample_rate as f64).round() as u64;

        Ok(DecodedAudio {
            samples,
            sample_rate,
            channels,
            duration_secs,
            format: ext.to_string(),
        })
    }
}
