pub mod detector;
pub mod dispatcher;
pub mod payload;

pub use detector::{AgentEnvironmentDetector, AgentEnvironmentStatus};
pub use dispatcher::{dispatch_agentic_instruction, AgentDispatcher, AgenticHarness};
pub use payload::{DispatchPayloadDetails, DispatchResult, FormattedCliPayload};
