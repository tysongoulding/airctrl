use airctrl_actions::*;
use airctrl_core::types::{AgentPayload, AgentTarget};

#[test]
fn test_agent_dispatch_all_8_supported_agents() {
    let agents = [
        ("Cursor", AgentTarget::Cursor),
        ("Claude", AgentTarget::Claude),
        ("Antigravity", AgentTarget::Antigravity),
        ("Grokbot", AgentTarget::Grokbot),
        ("Dots", AgentTarget::Dots),
        ("Muse", AgentTarget::Muse),
        ("Viktor", AgentTarget::Viktor),
        ("Gemini", AgentTarget::Gemini),
    ];

    for (name, target) in &agents {
        let payload = AgentPayload {
            target: *target,
            prompt: "Help me write a test".to_string(),
            context_window: Some("Context code".to_string()),
            timestamp: "2026-10-11T00:00:00Z".to_string(),
        };

        let result = dispatch_agentic_instruction(*target, &payload).expect("Dispatch failed");
        assert_eq!(result.target, *name);
        assert_eq!(result.status, "Dispatched");
        assert_eq!(result.prompt_len, payload.prompt.len());
        assert_eq!(result.payload.instruction, payload.prompt);
        assert_eq!(result.payload.context_window, "Context code");
        assert!(result.cli.is_some());
    }
}

#[test]
fn test_agent_dispatch_strict_case_sensitivity() {
    // Valid case
    let ok_res = AgentDispatcher::dispatch("Cursor", "test prompt", None);
    assert!(ok_res.is_ok());

    // Invalid lowercase rejected
    let err_res = AgentDispatcher::dispatch("cursor", "test prompt", None);
    assert!(err_res.is_err());
    match err_res.unwrap_err() {
        ActionError::UnsupportedAgent(msg) => {
            assert!(msg.contains("cursor"));
        }
        other => panic!("Expected UnsupportedAgent error, got {:?}", other),
    }

    // Invalid agent rejected
    let err_unknown = AgentDispatcher::dispatch("NonExistentAgent", "test", None);
    assert!(err_unknown.is_err());
}

#[test]
fn test_agent_dispatch_boundaries() {
    // 1-character prompt
    let res_1char = AgentDispatcher::dispatch("Claude", "?", None).expect("1-char dispatch failed");
    assert_eq!(res_1char.prompt_len, 1);
    assert_eq!(res_1char.payload.instruction, "?");

    // Empty context string
    let res_empty_ctx = AgentDispatcher::dispatch("Antigravity", "prompt", Some("")).expect("Empty context failed");
    assert_eq!(res_empty_ctx.payload.context_window, "");

    // Large context window (50,000 characters)
    let large_context = "A".repeat(50_000);
    let res_large = AgentDispatcher::dispatch("Cursor", "prompt", Some(&large_context)).expect("Large context failed");
    assert_eq!(res_large.payload.context_window.len(), 50_000);

    // Multiline prompt (100 newlines)
    let multiline_prompt = "\n".repeat(100);
    let res_multiline = AgentDispatcher::dispatch("Dots", &multiline_prompt, None).expect("Multiline dispatch failed");
    assert_eq!(res_multiline.prompt_len, 100);
    assert_eq!(res_multiline.payload.instruction, multiline_prompt);
}
