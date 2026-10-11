use std::collections::HashMap;
use strsim::{jaro_winkler, normalized_levenshtein};

/// Candidate window, application, or process for fuzzy target resolution.
#[derive(Debug, Clone)]
pub struct FuzzyCandidate<'a> {
    pub id: Option<u64>,
    pub display_name: &'a str,
    pub process_name: Option<&'a str>,
    pub window_title: Option<&'a str>,
}

/// Scoring result for a matched candidate.
#[derive(Debug, Clone, PartialEq)]
pub struct FuzzyMatchResult<'a> {
    pub candidate_id: Option<u64>,
    pub matched_name: &'a str,
    pub score: f64,
}

/// Configuration tuning parameters for fuzzy distance algorithms.
#[derive(Debug, Clone)]
pub struct FuzzyMatcherConfig {
    pub min_similarity_threshold: f64,
    pub exact_match_score: f64,
    pub acronym_boost: f64,
    pub substring_boost: f64,
    pub ambiguity_delta: f64,
}

impl Default for FuzzyMatcherConfig {
    fn default() -> Self {
        Self {
            min_similarity_threshold: 0.65,
            exact_match_score: 1.0,
            acronym_boost: 0.95,
            substring_boost: 0.90,
            ambiguity_delta: 0.05,
        }
    }
}

/// Alias dictionary mapping colloquial and shorthand queries to canonical target names.
#[derive(Debug, Clone)]
pub struct AliasRegistry {
    aliases: HashMap<&'static str, &'static [&'static str]>,
}

impl Default for AliasRegistry {
    fn default() -> Self {
        let mut aliases = HashMap::new();
        aliases.insert("chrom", &["Google Chrome", "chrome", "chrome.exe"][..]);
        aliases.insert("chrome", &["Google Chrome", "chrome", "chrome.exe"][..]);
        aliases.insert("vsc", &["Visual Studio Code", "Code.exe", "code", "vscode"][..]);
        aliases.insert("vscode", &["Visual Studio Code", "Code.exe", "code"][..]);
        aliases.insert("vs code", &["Visual Studio Code", "Code.exe", "code"][..]);
        aliases.insert("code", &["Visual Studio Code", "Code.exe"][..]);
        aliases.insert("term", &["Windows Terminal", "Terminal", "wt.exe", "wt", "alacritty", "kitty"][..]);
        aliases.insert("terminal", &["Windows Terminal", "Terminal", "wt.exe", "wt"][..]);
        aliases.insert("edge", &["Microsoft Edge", "msedge.exe", "msedge"][..]);
        aliases.insert("fox", &["Firefox", "firefox.exe", "Mozilla Firefox"][..]);
        aliases.insert("firefox", &["Firefox", "firefox.exe", "Mozilla Firefox"][..]);
        aliases.insert("slack", &["Slack", "slack.exe"][..]);
        aliases.insert("discord", &["Discord", "discord.exe"][..]);
        aliases.insert("spotify", &["Spotify", "spotify.exe"][..]);
        aliases.insert("spotfy", &["Spotify", "spotify.exe"][..]);
        aliases.insert("note", &["Notepad", "notepad.exe"][..]);
        aliases.insert("notepad", &["Notepad", "notepad.exe"][..]);
        Self { aliases }
    }
}

impl AliasRegistry {
    pub fn matches(&self, query: &str, target: &str) -> bool {
        let q = query.trim().to_lowercase();
        let t = target.trim().to_lowercase();
        if let Some(list) = self.aliases.get(q.as_str()) {
            list.iter().any(|item| item.to_lowercase() == t)
        } else {
            false
        }
    }
}

/// Levenshtein, Jaro-Winkler, and Acronym fuzzy matching engine.
#[derive(Default)]
pub struct FuzzyMatcher {
    config: FuzzyMatcherConfig,
    aliases: AliasRegistry,
}

impl FuzzyMatcher {
    pub fn new(config: FuzzyMatcherConfig) -> Self {
        Self {
            config,
            aliases: AliasRegistry::default(),
        }
    }

    /// Finds the best matching candidate for a given target query.
    pub fn find_best_match<'a>(
        &self,
        query: &str,
        candidates: &'a [FuzzyCandidate<'a>],
    ) -> Option<FuzzyMatchResult<'a>> {
        let q = query.trim().to_lowercase();
        if q.is_empty() || candidates.is_empty() {
            return None;
        }

        let mut scored_matches: Vec<FuzzyMatchResult<'a>> = candidates
            .iter()
            .map(|c| FuzzyMatchResult {
                candidate_id: c.id,
                matched_name: c.display_name,
                score: self.score_candidate(&q, c),
            })
            .filter(|r| r.score >= self.config.min_similarity_threshold)
            .collect();

        // Sort descending by score
        scored_matches.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        scored_matches.into_iter().next()
    }

    /// Evaluates multi-metric similarity score (0.0 to 1.0) between query and candidate.
    pub fn score_candidate(&self, query: &str, candidate: &FuzzyCandidate) -> f64 {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return 0.0;
        }

        let mut max_score: f64 = 0.0;

        let targets = [
            Some(candidate.display_name),
            candidate.process_name,
            candidate.window_title,
        ];

        for target_opt in targets.iter().flatten() {
            let t = target_opt.trim().to_lowercase();
            if t.is_empty() {
                continue;
            }

            // 1. Exact equality
            if q == t {
                return self.config.exact_match_score;
            }

            // 2. Base process name (strip .exe/.bin)
            let base_name = t
                .strip_suffix(".exe")
                .or_else(|| t.strip_suffix(".bin"))
                .unwrap_or(&t);
            if q == base_name {
                max_score = max_score.max(self.config.exact_match_score);
            }

            // 3. Known alias dictionary lookup
            if self.aliases.matches(&q, base_name) || self.aliases.matches(&q, &t) {
                max_score = max_score.max(0.98);
            }

            // 4. Acronym match (e.g. "vsc" matches "Visual Studio Code")
            if self.check_acronym(&q, &t) {
                max_score = max_score.max(self.config.acronym_boost);
            }

            // 5. Individual whole-word match in window title
            let words: Vec<&str> = t
                .split(|c: char| !c.is_alphanumeric())
                .filter(|s| !s.is_empty())
                .collect();
            for word in &words {
                if q == *word {
                    max_score = max_score.max(self.config.substring_boost);
                }
            }

            // 6. Substring containment
            if t.contains(&q) {
                let ratio = (q.len() as f64) / (t.len() as f64);
                max_score = max_score.max(0.75 + 0.15 * ratio);
            }

            // 7. Metric similarity: Jaro-Winkler + Normalized Levenshtein
            let lev = normalized_levenshtein(&q, base_name);
            let jw = jaro_winkler(&q, base_name);
            let metric_score = 0.6 * jw + 0.4 * lev;
            max_score = max_score.max(metric_score);

            // Also test against individual words
            for word in &words {
                let w_lev = normalized_levenshtein(&q, word);
                let w_jw = jaro_winkler(&q, word);
                let w_score = 0.6 * w_jw + 0.4 * w_lev;
                max_score = max_score.max(w_score);
            }
        }

        max_score
    }

    /// Checks if query is the acronym of candidate words (e.g. "vsc" -> "Visual Studio Code").
    fn check_acronym(&self, query: &str, target: &str) -> bool {
        let acronym: String = target
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.chars().next())
            .collect::<String>()
            .to_lowercase();

        !acronym.is_empty() && acronym == query
    }
}
