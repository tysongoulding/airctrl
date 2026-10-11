use super::schema::{ReaActionSchema, ReaInspectionSchema, RecursiveReaTaskSpec};
use crate::error::ActionError;

pub struct ReaInspector;

impl ReaInspector {
    pub const DEFAULT_MOCK_BINARY: &'static str = "/usr/bin/mock-app";

    /// Inspects the target process by PID and optional binary path.
    /// Enforces PID > 0, rejecting 0 and negative values.
    pub fn inspect(pid: i64, binary_path: Option<&str>) -> Result<ReaInspectionSchema, ActionError> {
        if pid <= 0 {
            return Err(ActionError::InvalidPid(pid));
        }

        let effective_binary = binary_path
            .filter(|p| !p.trim().is_empty())
            .unwrap_or(Self::DEFAULT_MOCK_BINARY)
            .to_string();

        Ok(ReaInspectionSchema {
            target_pid: pid as u32,
            binary_path: effective_binary,
            action_schema: ReaActionSchema {
                buttons: vec!["Save".to_string(), "Cancel".to_string(), "Submit".to_string()],
                inputs: vec!["Username".to_string(), "Password".to_string()],
                menus: vec!["File".to_string(), "Edit".to_string(), "View".to_string()],
            },
            status: "SchemaExtracted".to_string(),
        })
    }

    /// Formats REA reverse-engineering results into recursive task spec format.
    pub fn to_recursive_spec(schema: &ReaInspectionSchema, depth: u32) -> RecursiveReaTaskSpec {
        RecursiveReaTaskSpec {
            rea_version: "1.0".to_string(),
            task_id: format!("rea-spec-{}", uuid::Uuid::new_v4()),
            target_pid: schema.target_pid,
            binary_path: schema.binary_path.clone(),
            recursive_depth: depth,
            action_schema: schema.action_schema.clone(),
            status: schema.status.clone(),
        }
    }
}

/// Dispatches REA inspection by PID with optional binary path.
pub fn execute_rea_inspection(
    pid: u32,
    binary_path: Option<String>,
) -> Result<ReaInspectionSchema, ActionError> {
    ReaInspector::inspect(pid as i64, binary_path.as_deref())
}

/// Dispatches REA inspection by application name / binary path.
pub fn execute_rea_inspection_by_name(
    app_name: &str,
) -> Result<ReaInspectionSchema, ActionError> {
    ReaInspector::inspect(1234, Some(app_name))
}
