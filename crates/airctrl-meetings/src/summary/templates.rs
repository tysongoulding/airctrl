use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub user_prompt_template: String,
    pub is_builtin: bool,
}

#[derive(Debug, Clone, Default)]
pub struct TemplateContext<'a> {
    pub transcript: &'a str,
    pub meeting_title: Option<&'a str>,
    pub date: Option<&'a str>,
    pub duration: Option<&'a str>,
    pub participants: Option<&'a str>,
    pub platform: Option<&'a str>,
}

/// Replaces known template variables without external template engines.
/// Unknown tags (e.g. `{{custom}}`) are preserved untouched.
pub fn interpolate_template(template_str: &str, ctx: &TemplateContext<'_>) -> String {
    let mut out = template_str.to_string();
    out = out.replace("{{transcript}}", ctx.transcript);
    out = out.replace("{{meeting_title}}", ctx.meeting_title.unwrap_or("Untitled Meeting"));
    out = out.replace("{{date}}", ctx.date.unwrap_or(""));
    out = out.replace("{{duration}}", ctx.duration.unwrap_or("N/A"));
    out = out.replace("{{participants}}", ctx.participants.unwrap_or(""));
    out = out.replace("{{platform}}", ctx.platform.unwrap_or("Meeting"));
    out
}

pub fn get_builtin_template(name: &str) -> Option<TemplateDefinition> {
    match name.to_lowercase().replace(['_', '-'], "").as_str() {
        "executivesummary" | "executive" => Some(TemplateDefinition {
            id: "executive_summary".to_string(),
            name: "Executive Summary".to_string(),
            description: "High-level overview and key discussion points".to_string(),
            system_prompt: "You are an executive chief of staff. Provide a high-level summary of the meeting formatted in clean Markdown.".to_string(),
            user_prompt_template: "# Executive Summary\n\n## Overview\nMeeting transcript analyzed.\n\n## Key Points\n{{transcript}}\n".to_string(),
            is_builtin: true,
        }),
        "actionitems" | "actions" => Some(TemplateDefinition {
            id: "action_items".to_string(),
            name: "Action Items".to_string(),
            description: "Checklist of action items, owners, and deadlines".to_string(),
            system_prompt: "You are a technical project manager. Extract all action items, owners, and deadlines from the transcript.".to_string(),
            user_prompt_template: "# Action Items\n\n- [ ] Task description (Assignee: Name, Deadline: Date)\n\nTranscript:\n{{transcript}}\n".to_string(),
            is_builtin: true,
        }),
        "technicaldecisions" | "decisions" => Some(TemplateDefinition {
            id: "technical_decisions".to_string(),
            name: "Technical Decisions".to_string(),
            description: "Architecture decision record (ADR) and trade-offs".to_string(),
            system_prompt: "You are a principal systems architect. Extract technical decisions and evaluated alternatives.".to_string(),
            user_prompt_template: "# Technical Decisions\n\n1. Architecture choice\n\nTranscript:\n{{transcript}}\n".to_string(),
            is_builtin: true,
        }),
        _ => None,
    }
}
