use serde::{Serialize, Deserialize};


/// A struct representing a copy task.
///
/// A copy task is a task that copies files from a source to a target directory.
///
/// # Fields
///
/// * `source` - The source directory or file.
/// * `description` - A human-readable description of the task.
/// * `target` - The target directory.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CopyTask {
    pub source: String,
    pub description: String,
    pub target: String
}