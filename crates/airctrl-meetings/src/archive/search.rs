use super::entry::MeetingArchiveEntry;
use super::store::MeetingArchiveStore;

impl MeetingArchiveStore {
    /// Full-text search across meeting title, participants, AI summary markdown,
    /// and nested transcript chunks.
    /// Case-insensitive. Empty or whitespace-only query returns all meetings.
    pub fn search(&self, query: &str) -> Vec<MeetingArchiveEntry> {
        let q = query.trim().to_lowercase();
        let entries = self.list_meetings();

        if q.is_empty() {
            return entries;
        }

        let mut matched = Vec::new();

        for entry in entries {
            let is_match = entry.title.to_lowercase().contains(&q)
                || entry.summary_markdown.to_lowercase().contains(&q)
                || entry.participants.iter().any(|p| p.to_lowercase().contains(&q))
                || entry.transcript_chunks.iter().any(|chunk| chunk.text.to_lowercase().contains(&q));

            if is_match {
                matched.push(entry);
            }
        }

        matched
    }

    /// Extracts an excerpt around the first match occurrence within a 80-character window.
    /// Safely handles multi-byte UTF-8 code points (emojis, CJK, accented Latin) and
    /// casing length disparities without panicking or allocating intermediate vectors.
    pub fn extract_match_snippet(text: &str, query: &str) -> Option<String> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return None;
        }

        let lower = text.to_lowercase();
        let lower_idx = lower.find(&q)?;
        let text_len = text.len();

        // Determine starting byte offset in `text`.
        // Fast path: if byte lengths match, lower_idx corresponds identically in text.
        let match_byte = if text_len == lower.len() {
            lower_idx
        } else {
            // Map lower_idx back to character byte boundary in text
            let mut lower_byte_offset = 0;
            let mut found = text_len;
            for (byte_offset, ch) in text.char_indices() {
                if lower_byte_offset >= lower_idx {
                    found = byte_offset;
                    break;
                }
                for lc in ch.to_lowercase() {
                    lower_byte_offset += lc.len_utf8();
                }
            }
            found
        };

        // Guarantee match_byte is on a valid character boundary
        let safe_match_byte = text.floor_char_boundary(match_byte.min(text_len));

        // Look back up to 40 characters before match start
        let start = text[..safe_match_byte]
            .char_indices()
            .rev()
            .nth(39)
            .map(|(offset, _)| offset)
            .unwrap_or(0);

        // Approximate match end by advancing query character count from safe_match_byte
        let q_char_count = q.chars().count();
        let match_end = text[safe_match_byte..]
            .char_indices()
            .nth(q_char_count)
            .map(|(offset, _)| safe_match_byte + offset)
            .unwrap_or(text_len);

        // Look ahead up to 40 characters after match end
        let end = text[match_end..]
            .char_indices()
            .nth(40)
            .map(|(offset, _)| match_end + offset)
            .unwrap_or(text_len);

        let prefix = if start > 0 { "..." } else { "" };
        let suffix = if end < text_len { "..." } else { "" };

        Some(format!("{}{}{}", prefix, text[start..end].trim(), suffix))
    }
}
