use airctrl_actions::*;

#[test]
fn test_fuzzy_matcher_exact_matches() {
    let matcher = FuzzyMatcher::default();
    let candidates = vec![
        FuzzyCandidate {
            id: Some(1),
            display_name: "Google Chrome",
            process_name: Some("chrome.exe"),
            window_title: Some("Google Chrome - New Tab"),
        },
        FuzzyCandidate {
            id: Some(2),
            display_name: "Visual Studio Code",
            process_name: Some("Code.exe"),
            window_title: Some("airctrl - Visual Studio Code"),
        },
    ];

    let match_exact = matcher.find_best_match("Google Chrome", &candidates);
    assert!(match_exact.is_some());
    assert_eq!(match_exact.unwrap().candidate_id, Some(1));

    let match_proc = matcher.find_best_match("code", &candidates);
    assert!(match_proc.is_some());
    assert_eq!(match_proc.unwrap().candidate_id, Some(2));
}

#[test]
fn test_fuzzy_matcher_alias_registry() {
    let matcher = FuzzyMatcher::default();
    let candidates = vec![
        FuzzyCandidate {
            id: Some(101),
            display_name: "Google Chrome",
            process_name: Some("chrome.exe"),
            window_title: Some("GitHub - airctrl"),
        },
        FuzzyCandidate {
            id: Some(102),
            display_name: "Visual Studio Code",
            process_name: Some("Code.exe"),
            window_title: Some("main.rs - Visual Studio Code"),
        },
        FuzzyCandidate {
            id: Some(103),
            display_name: "Windows Terminal",
            process_name: Some("wt.exe"),
            window_title: Some("Terminal"),
        },
    ];

    // Alias: "chrom" -> Chrome
    let match_chrom = matcher.find_best_match("chrom", &candidates);
    assert!(match_chrom.is_some());
    assert_eq!(match_chrom.unwrap().candidate_id, Some(101));

    // Alias: "vsc" -> VS Code
    let match_vsc = matcher.find_best_match("vsc", &candidates);
    assert!(match_vsc.is_some());
    assert_eq!(match_vsc.unwrap().candidate_id, Some(102));

    // Alias: "term" -> Windows Terminal
    let match_term = matcher.find_best_match("term", &candidates);
    assert!(match_term.is_some());
    assert_eq!(match_term.unwrap().candidate_id, Some(103));
}

#[test]
fn test_fuzzy_matcher_acronym_and_substring() {
    let matcher = FuzzyMatcher::default();
    let candidates = vec![
        FuzzyCandidate {
            id: Some(201),
            display_name: "Slack",
            process_name: Some("slack.exe"),
            window_title: Some("Slack | general"),
        },
        FuzzyCandidate {
            id: Some(202),
            display_name: "Spotify",
            process_name: Some("spotify.exe"),
            window_title: Some("Spotify Free"),
        },
    ];

    let match_spotfy = matcher.find_best_match("spotfy", &candidates);
    assert!(match_spotfy.is_some());
    assert_eq!(match_spotfy.unwrap().candidate_id, Some(202));
}

#[test]
fn test_fuzzy_matcher_empty_and_threshold_rejection() {
    let matcher = FuzzyMatcher::default();
    let candidates = vec![
        FuzzyCandidate {
            id: Some(301),
            display_name: "Calculator",
            process_name: Some("calc.exe"),
            window_title: Some("Calculator"),
        },
    ];

    // Empty query returns None
    assert!(matcher.find_best_match("", &candidates).is_none());

    // Completely unrelated query below threshold returns None
    assert!(matcher.find_best_match("zzzzzzqwerty", &candidates).is_none());
}
