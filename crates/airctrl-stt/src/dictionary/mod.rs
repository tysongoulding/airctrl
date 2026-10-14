use crate::types::DictionaryRule;
use regex::{Regex, RegexBuilder};

/// Rule-based dictionary replacement and text normalization engine.
///
/// Features:
/// - Topological rule sorting (longest triggers first to prevent partial masking).
/// - Word boundary matching (`\b`) for literal replacements.
/// - Case preservation heuristics (UPPERCASE, Title Case, lowercase).
/// - Regex replacement with capture group and case-sensitivity controls.
/// - Handy post-processing (whitespace collapsing, trimming, leading capitalization).
pub struct DictionaryEngine {
    rules: Vec<DictionaryRule>,
    post_processing_enabled: bool,
}

impl Default for DictionaryEngine {
    fn default() -> Self {
        Self::new(Vec::new(), true)
    }
}

impl DictionaryEngine {
    pub fn new(rules: Vec<DictionaryRule>, post_processing_enabled: bool) -> Self {
        let mut sorted = rules;
        // Topological sort: longest triggers evaluated first to avoid partial sub-token collisions
        sorted.sort_by_key(|a| std::cmp::Reverse(a.trigger.len()));
        Self {
            rules: sorted,
            post_processing_enabled,
        }
    }

    pub fn set_rules(&mut self, rules: Vec<DictionaryRule>) {
        let mut sorted = rules;
        sorted.sort_by_key(|a| std::cmp::Reverse(a.trigger.len()));
        self.rules = sorted;
    }

    pub fn rules(&self) -> &[DictionaryRule] {
        &self.rules
    }

    pub fn set_post_processing(&mut self, enabled: bool) {
        self.post_processing_enabled = enabled;
    }

    pub fn is_post_processing_enabled(&self) -> bool {
        self.post_processing_enabled
    }

    /// Replaces dictionary trigger words and applies optional post-processing.
    pub fn replace_words(&self, text: &str) -> String {
        if text.trim().is_empty() {
            return String::new();
        }

        let mut result = text.to_string();

        for rule in &self.rules {
            if rule.is_regex {
                if let Ok(re) = RegexBuilder::new(&rule.trigger)
                    .case_insensitive(!rule.preserve_case)
                    .build()
                {
                    result = re.replace_all(&result, &rule.replacement).to_string();
                }
            } else {
                // Word-boundary literal substitution: \b{trigger}\b
                let escaped = regex::escape(&rule.trigger);
                let pattern = format!(r"(?i)\b{}\b", escaped);
                if let Ok(re) = Regex::new(&pattern) {
                    result = re
                        .replace_all(&result, |caps: &regex::Captures| {
                            let matched = caps.get(0).map(|m| m.as_str()).unwrap_or("");
                            if rule.preserve_case {
                                Self::apply_case_preservation(matched, &rule.replacement)
                            } else {
                                rule.replacement.clone()
                            }
                        })
                        .to_string();
                }
            }
        }

        if self.post_processing_enabled {
            self.apply_post_processing(&result)
        } else {
            result
        }
    }

    /// Case preservation heuristics:
    /// - ALL_UPPERCASE ("AIR CONTROL") -> "AIRCTRL"
    /// - Title Case / Capitalized ("Air Control") -> "Airctrl" / "AirCtrl"
    /// - lowercase ("air control") -> "airctrl"
    fn apply_case_preservation(matched: &str, replacement: &str) -> String {
        let is_all_upper = matched.chars().all(|c| !c.is_alphabetic() || c.is_uppercase());
        let is_first_upper = matched
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false);

        if is_all_upper {
            replacement.to_uppercase()
        } else if is_first_upper {
            let mut chars = replacement.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        } else {
            replacement.to_string()
        }
    }

    /// Standard Handy post-processing: collapses consecutive whitespace, strips boundaries,
    /// and capitalizes leading character of final transcription.
    fn apply_post_processing(&self, text: &str) -> String {
        let whitespace_re = match Regex::new(r"\s+") {
            Ok(r) => r,
            Err(_) => return text.trim().to_string(),
        };
        let cleaned = whitespace_re.replace_all(text, " ").trim().to_string();
        if cleaned.is_empty() {
            return cleaned;
        }

        let mut chars = cleaned.chars();
        match chars.next() {
            Some(first) if first.is_lowercase() => {
                first.to_uppercase().collect::<String>() + chars.as_str()
            }
            _ => cleaned,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dictionary_literal_replacement_and_case_preservation() {
        let rules = vec![
            DictionaryRule::literal("air control", "airctrl"),
            DictionaryRule::literal("ty sun", "Tyson"),
        ];
        let engine = DictionaryEngine::new(rules, false);

        assert_eq!(engine.replace_words("I use air control"), "I use airctrl");
        assert_eq!(engine.replace_words("I USE AIR CONTROL"), "I USE AIRCTRL");
        assert_eq!(engine.replace_words("Hello Air Control"), "Hello Airctrl");
        assert_eq!(engine.replace_words("My name is ty sun"), "My name is Tyson");
    }

    #[test]
    fn test_dictionary_word_boundary_isolation() {
        let rules = vec![DictionaryRule::literal("cat", "feline")];
        let engine = DictionaryEngine::new(rules, false);

        // "cat" is replaced, but "concatenate" is preserved!
        assert_eq!(engine.replace_words("the cat was black"), "the feline was black");
        assert_eq!(engine.replace_words("do not concatenate"), "do not concatenate");
    }

    #[test]
    fn test_dictionary_post_processing_toggle() {
        let engine_with_post = DictionaryEngine::new(vec![], true);
        assert_eq!(engine_with_post.replace_words("  hello   world  "), "Hello world");

        let engine_raw = DictionaryEngine::new(vec![], false);
        assert_eq!(engine_raw.replace_words("  hello   world  "), "  hello   world  ");
    }

    #[test]
    fn test_dictionary_regex_rule() {
        let rules = vec![DictionaryRule::regex(r"\b(\d+)\s*dollars\b", "$$${1}")];
        let engine = DictionaryEngine::new(rules, false);

        assert_eq!(engine.replace_words("that costs 100 dollars"), "that costs $100");
    }
}
