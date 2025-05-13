use serde::{Serialize, Deserialize};


/// A struct representing a sync task.
///
/// # Fields
///
/// * `zip_path` - The path to the ZIP file.
/// * `extract_path` - The path to extract the ZIP file to.
/// * `zip_date` - The date of the ZIP file.
/// * `extract_date` - The date of the extracted files.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncTask {
    pub zip_path: String,
    pub extract_path: String,
    pub zip_date: Option<i32>,
    pub extract_date: Option<i32>
}