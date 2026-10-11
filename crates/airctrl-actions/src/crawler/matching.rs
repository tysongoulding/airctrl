use airctrl_core::types::{MatchType, WindowDetails, WindowMatch};

pub struct WindowMatcher {
    fuzzy_threshold: f32,
}

impl WindowMatcher {
    pub fn new(fuzzy_threshold: f32) -> Self {
        Self { fuzzy_threshold }
    }

    pub fn match_windows(&self, query: &str, candidates: &[WindowDetails]) -> Vec<WindowMatch> {
        let raw_query = query.trim();
        if raw_query.is_empty() {
            return Vec::new();
        }

        let norm_query = raw_query
            .to_lowercase()
            .trim_end_matches(".exe")
            .to_string();

        let query_tokens: Vec<&str> = norm_query.split_whitespace().collect();
        let mut matches = Vec::new();

        for win in candidates {
            let norm_proc = win
                .process_name
                .to_lowercase()
                .trim_end_matches(".exe")
                .to_string();
            let norm_title = win.title.to_lowercase();

            let mut best_match: Option<(MatchType, f32)> = None;

            // 1. Exact Process Name Match
            if norm_proc == norm_query {
                best_match = Some((MatchType::ExactProcess, 1.0));
            }
            // 2. Exact Title Match
            else if norm_title == norm_query {
                best_match = Some((MatchType::ExactTitle, 1.0));
            }
            // 3. Substring in Process Name
            else if norm_proc.contains(&norm_query) {
                best_match = Some((MatchType::SubstringProcess, 0.92));
            }
            // 4. Substring in Title
            else if norm_title.contains(&norm_query) {
                let ratio = (norm_query.len() as f32 / norm_title.len().max(1) as f32).min(1.0);
                let score = 0.85 + (0.10 * ratio);
                best_match = Some((MatchType::SubstringTitle, score));
            }
            // 5. Token Overlap in Title
            else if !query_tokens.is_empty() && query_tokens.iter().all(|t| norm_title.contains(t)) {
                best_match = Some((MatchType::TokenOverlap, 0.88));
            }
            // 6. Fuzzy Metric via strsim (Jaro-Winkler & Normalized Levenshtein)
            else {
                let jaro_title = strsim::jaro_winkler(&norm_query, &norm_title) as f32;
                let jaro_proc = strsim::jaro_winkler(&norm_query, &norm_proc) as f32;
                let lev_title = strsim::normalized_levenshtein(&norm_query, &norm_title) as f32;
                let lev_proc = strsim::normalized_levenshtein(&norm_query, &norm_proc) as f32;

                let max_title = jaro_title.max(lev_title);
                let max_proc = jaro_proc.max(lev_proc);

                if max_proc >= max_title && max_proc >= self.fuzzy_threshold {
                    best_match = Some((MatchType::FuzzyProcess, max_proc * 0.95));
                } else if max_title >= self.fuzzy_threshold {
                    best_match = Some((MatchType::FuzzyTitle, max_title));
                }
            }

            if let Some((m_type, base_score)) = best_match {
                // Apply contextual priority boosts
                let mut final_score = base_score;
                if win.is_focused {
                    final_score += 0.05;
                }
                if !win.is_minimized {
                    final_score += 0.04;
                }
                if !win.title.is_empty() {
                    final_score += 0.01;
                }

                if final_score >= self.fuzzy_threshold {
                    matches.push(WindowMatch {
                        window: win.clone(),
                        match_type: m_type,
                        score: final_score.min(1.0),
                    });
                }
            }
        }

        // Sort descending by score
        matches.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        matches
    }
}
