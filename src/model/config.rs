use serde::{Serialize, Deserialize};
use crate::model::copy_task::CopyTask;


/// The configuration object which holds the settings for the application
/// and the copy tasks.
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub zip_path: String,
    pub direction: String,
    pub confirm_new: String,
    pub confirm_overwrite: String,
    pub confirm_delete: String,
    pub copy_tasks: Vec<CopyTask>
}

/// Implementation of the `Config` struct.
///
/// # Methods
///
/// * `new` - Creates a new instance of the `Config` struct.
/// * `clean` - Removes leading slashes from the target paths of the copy tasks.
///
/// # Examples
///
/// ```
/// let mut config = Config::new();
/// config.clean();
/// ```
impl Config {
    /// Creates a new instance of the `Config` struct.
    ///
    /// # Returns
    ///
    /// * `Config` - A new instance of the `Config` struct.
    pub fn new() -> Self {
        Self {
            zip_path: String::new(),
            direction: String::new(),
            confirm_new: String::new(),
            confirm_overwrite: String::new(),
            confirm_delete: String::new(),
            copy_tasks: Vec::new(),
        }
    }

    /// Removes leading slashes from the target paths of the copy tasks.
    pub fn clean(&mut self) {
        // Remove leading slashes from the target paths of the copy tasks
        for task in self.copy_tasks.iter_mut() {
            if task.target.starts_with("/") {
                task.target = task.target[1..].to_string();
            }
        }
    }
}