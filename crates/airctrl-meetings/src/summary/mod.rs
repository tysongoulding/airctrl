pub mod provider;
pub mod service;
pub mod templates;
pub mod types;

pub use provider::HttpSummaryClient;
pub use service::AiSummaryService;
pub use templates::{interpolate_template, TemplateContext, TemplateDefinition};
pub use types::{SummaryProvider, SummaryProviderConfig, SummaryResult, SummaryTemplate};
