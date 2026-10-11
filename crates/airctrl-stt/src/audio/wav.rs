use crate::error::{Result, SttError};
use std::io::Cursor;

/// Parses RIFF 16-bit PCM WAV bytes into normalized [-1.0, 1.0] mono float samples.
///
/// Enforces:
/// 1. Minimum 44-byte length check (raises `InvalidWavHeader`).
/// 2. Valid `RIFF` and `WAVE` magic markers.
/// 3. Normalization of 16-bit PCM integers by `32768.0`.
/// 4. Multi-channel downmixing by arithmetic channel averaging.
pub fn parse_wav_samples(wav_bytes: &[u8]) -> Result<Vec<f32>> {
    if wav_bytes.len() < 44 {
        return Err(SttError::InvalidWavHeader(
            "WAV header truncated (less than 44 bytes)".to_string(),
        ));
    }

    if &wav_bytes[0..4] != b"RIFF" || &wav_bytes[8..12] != b"WAVE" {
        return Err(SttError::InvalidWavHeader(
            "Invalid RIFF/WAVE header".to_string(),
        ));
    }

    let cursor = Cursor::new(wav_bytes);
    let mut reader = hound::WavReader::new(cursor)
        .map_err(|e| SttError::WavDecodeError(e.to_string()))?;

    let spec = reader.spec();
    let channels = spec.channels as usize;

    if channels == 0 {
        return Err(SttError::InvalidWavHeader(
            "WAV specifies 0 channels".to_string(),
        ));
    }

    let raw_samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => match spec.bits_per_sample {
            16 => reader
                .samples::<i16>()
                .map(|s| s.map(|v| (v as f32 / 32768.0).clamp(-1.0, 1.0)))
                .collect::<std::result::Result<Vec<f32>, hound::Error>>()
                .map_err(|e| SttError::WavDecodeError(e.to_string()))?,
            24 => reader
                .samples::<i32>()
                .map(|s| s.map(|v| (v as f32 / 8_388_608.0).clamp(-1.0, 1.0)))
                .collect::<std::result::Result<Vec<f32>, hound::Error>>()
                .map_err(|e| SttError::WavDecodeError(e.to_string()))?,
            32 => reader
                .samples::<i32>()
                .map(|s| s.map(|v| (v as f32 / 2_147_483_648.0).clamp(-1.0, 1.0)))
                .collect::<std::result::Result<Vec<f32>, hound::Error>>()
                .map_err(|e| SttError::WavDecodeError(e.to_string()))?,
            bits => {
                return Err(SttError::UnsupportedAudioFormat(format!(
                    "Unsupported integer bit depth: {bits} bits"
                )));
            }
        },
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|s| s.map(|v| v.clamp(-1.0, 1.0)))
            .collect::<std::result::Result<Vec<f32>, hound::Error>>()
            .map_err(|e| SttError::WavDecodeError(e.to_string()))?,
    };

    if channels == 1 {
        Ok(raw_samples)
    } else {
        let mut mono = Vec::with_capacity(raw_samples.len() / channels);
        for chunk in raw_samples.chunks(channels) {
            let sum: f32 = chunk.iter().sum();
            let avg = (sum / channels as f32).clamp(-1.0, 1.0);
            mono.push(avg);
        }
        Ok(mono)
    }
}

/// Generates synthetic PCM 16-bit RIFF WAV audio bytes matching E2E test harness requirements.
pub fn create_synthetic_wav(
    duration_secs: f32,
    sample_rate: u32,
    channels: u16,
    freq: f32,
    amplitude: f32,
    corrupt_header: bool,
) -> Vec<u8> {
    let total_samples = (duration_secs * sample_rate as f32).round() as usize;
    let ch = channels.max(1) as usize;
    let mut raw_pcm = Vec::with_capacity(total_samples * ch * 2);

    for i in 0..total_samples {
        let t = i as f32 / sample_rate as f32;
        let val_f = amplitude * 32767.0 * (2.0 * std::f32::consts::PI * freq * t).sin();
        let val_i = val_f.clamp(-32768.0, 32767.0).round() as i16;
        for _ in 0..channels {
            raw_pcm.extend_from_slice(&val_i.to_le_bytes());
        }
    }

    let data_size = raw_pcm.len() as u32;
    let block_align = channels * 2;
    let byte_rate = sample_rate * block_align as u32;
    let riff_size = 36 + data_size;

    let mut header = Vec::with_capacity(44);
    if corrupt_header {
        header.extend_from_slice(b"BADF");
    } else {
        header.extend_from_slice(b"RIFF");
    }
    header.extend_from_slice(&riff_size.to_le_bytes());
    header.extend_from_slice(b"WAVEfmt ");
    header.extend_from_slice(&16u32.to_le_bytes()); // PCM Subchunk1Size
    header.extend_from_slice(&1u16.to_le_bytes());  // AudioFormat (1 = PCM)
    header.extend_from_slice(&channels.to_le_bytes());
    header.extend_from_slice(&sample_rate.to_le_bytes());
    header.extend_from_slice(&byte_rate.to_le_bytes());
    header.extend_from_slice(&block_align.to_le_bytes());
    header.extend_from_slice(&16u16.to_le_bytes()); // BitsPerSample
    header.extend_from_slice(b"data");
    header.extend_from_slice(&data_size.to_le_bytes());

    let mut wav_bytes = header;
    wav_bytes.extend_from_slice(&raw_pcm);
    wav_bytes
}

/// Encodes normalized [-1.0, 1.0] float samples into 16-bit PCM WAV bytes using hound.
pub fn encode_wav(samples: &[f32], sample_rate: u32, channels: u16) -> Result<Vec<u8>> {
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec)
            .map_err(|e| SttError::WavDecodeError(e.to_string()))?;

        for &s in samples {
            let clamped = s.clamp(-1.0, 1.0);
            let val = (clamped * 32767.0).round() as i16;
            writer
                .write_sample(val)
                .map_err(|e| SttError::WavDecodeError(e.to_string()))?;
        }

        writer
            .finalize()
            .map_err(|e| SttError::WavDecodeError(e.to_string()))?;
    }

    Ok(cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_parse_synthetic_wav() {
        let wav = create_synthetic_wav(0.1, 16000, 1, 440.0, 0.5, false);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");

        let samples = parse_wav_samples(&wav).unwrap();
        assert_eq!(samples.len(), 1600);
        for &s in &samples {
            assert!((-1.0..=1.0).contains(&s));
        }
    }

    #[test]
    fn test_corrupt_wav_header_rejected() {
        let corrupt = create_synthetic_wav(0.1, 16000, 1, 440.0, 0.5, true);
        assert!(parse_wav_samples(&corrupt).is_err());
    }

    #[test]
    fn test_truncated_wav_header_rejected() {
        let truncated = b"RIFF1234WAVEfmt ";
        assert!(parse_wav_samples(truncated).is_err());
    }

    #[test]
    fn test_stereo_downmix_to_mono() {
        let stereo = create_synthetic_wav(0.05, 16000, 2, 440.0, 0.4, false);
        let samples = parse_wav_samples(&stereo).unwrap();
        assert_eq!(samples.len(), 800); // 0.05s * 16000 = 800 mono samples
    }
}
