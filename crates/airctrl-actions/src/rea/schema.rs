use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaActionSchema {
    pub buttons: Vec<String>,
    pub inputs: Vec<String>,
    pub menus: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaInspectionSchema {
    pub target_pid: u32,
    pub binary_path: String,
    pub action_schema: ReaActionSchema,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecursiveReaTaskSpec {
    pub rea_version: String,
    pub task_id: String,
    pub target_pid: u32,
    pub binary_path: String,
    pub recursive_depth: u32,
    pub action_schema: ReaActionSchema,
    pub status: String,
}
