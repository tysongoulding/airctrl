use super::entry::LiveTranscriptChunk;
use super::store::MeetingArchiveStore;

impl MeetingArchiveStore {
    /// Returns the exact start timestamp in milliseconds for a chunk within a meeting.
    /// Returns None if either the meeting or chunk does not exist.
    pub fn seek_audio_offset(&self, meeting_id: &str, chunk_id: &str) -> Option<u64> {
        let entry = self.get_meeting(meeting_id)?;
        for chunk in &entry.transcript_chunks {
            if chunk.id == chunk_id {
                return Some(chunk.start_time_ms);
            }
        }
        None
    }

    /// Reverse lookup: Resolves active transcript chunk at a given playback offset.
    pub fn get_chunk_at_offset(&self, meeting_id: &str, offset_ms: u64) -> Option<LiveTranscriptChunk> {
        let entry = self.get_meeting(meeting_id)?;
        for chunk in &entry.transcript_chunks {
            if chunk.start_time_ms <= offset_ms && offset_ms < chunk.end_time_ms {
                return Some(chunk.clone());
            }
        }
        None
    }

    /// Computes physical byte offset into a standard PCM WAV file (44-byte RIFF header).
    pub fn compute_wav_byte_offset(
        offset_ms: u64,
        sample_rate: u32,
        channels: u16,
        bits_per_sample: u16,
    ) -> u64 {
        let bytes_per_sample = (bits_per_sample as u64) / 8;
        let bytes_per_second = (sample_rate as u64) * (channels as u64) * bytes_per_sample;
        let pcm_offset = (offset_ms * bytes_per_second) / 1000;
        44 + pcm_offset
    }
}
