use std::fs;
use std::io::{self, BufReader};
use std::path::Path;
use colored::*;
use fs_extra::{copy_items, dir::*};
use zip::ZipArchive;


use crate::{model::config::Config, model::sync_task::SyncTask};


/// The ZipReaderController is responsible for reading the ZIP file and
/// extracting the files to a folder. It also creates the sync tasks based
/// on the configuration and compares the files in the ZIP archive with the
/// files in the extract paths.
pub struct ZipReaderController {
    /// The configuration object which holds the settings for the application
    /// and the copy tasks.
    config: Config,

    /// The vector of sync tasks which are used to compare the files in the ZIP
    /// archive with the files in the extract paths.
    sync_tasks: Vec<SyncTask>,

    /// The output folder where the ZIP file was extracted to.
    outputfolder: Option<String>
}

impl ZipReaderController {
    pub fn new(config: Config) -> ZipReaderController {
        ZipReaderController {
            config,
            sync_tasks: Vec::new(),
            outputfolder: None
        }
    }

    /// Starts the ZIP reader controller.
    ///
    /// This method extracts the ZIP file to a folder, creates the sync tasks
    /// based on the configuration, checks the ZIP archive for new files that
    /// are not handled by the config, and syncs the files.
    pub fn start(&mut self) {
        if let Err(e) = self.extract_zip_to_folder() {
            eprintln!("Error extracting ZIP file: {}", e);
            return;
        }
        self.create_sync_tasks_from_config();
        self.check_zip_for_new_files();
        self.sync_files();
    }

    /// Creates the sync tasks based on the configuration.
    /// The sync tasks are used to compare the files in the ZIP archive
    /// with the files in the extract paths. If no zip path (= target path)
    /// is given, the last path component of the extract path (= source path)
    /// is used ("/aa/bb/cc" -> "cc").
    /// The sync tasks are stored in the `sync_tasks` vector.
    fn create_sync_tasks_from_config(&mut self) {
        // Loop tasks from config
        for task in &self.config.copy_tasks {
            let mut zip_path: String;

            // Path to extract the file to (= former source path)
            let mut extract_path: String = task.source.clone();

            // Get the path of the item in the zip path (= former target path)
            if task.target.clone().is_empty() {
                // If the zip path is empty, get directory from the extract path
                zip_path = Self::get_last_path_component(&extract_path)
                           .unwrap_or_else(
                               || String::from("default_path")
                           );

                if extract_path.ends_with("/") {
                    extract_path = Self::remove_last_path_component(
                                       &extract_path)
                                   .unwrap_or_else(
                                       || String::from("default_path")
                                   );
                }
            } else {
                zip_path = task.target.clone();
            }

            // Remove leading slash from zip path
            if zip_path.starts_with("/") {
                zip_path = zip_path[1..].to_string();
            }

            // Check if zip_path is a folder but extract_path is a file
            let extract_path_obj = Path::new(&extract_path);

            if Path::new(&zip_path).to_string_lossy().ends_with('/')
            && !Path::new(&extract_path).to_string_lossy().ends_with('/') {
                if let Some(file_name) = extract_path_obj.file_name() {
                    zip_path.push_str(&file_name.to_string_lossy());
                }
            }

            // Create new sync task
            let sync_task: SyncTask = SyncTask {
                zip_path: zip_path,
                extract_path: extract_path,
                zip_date: None,
                extract_date: None
            };
            self.sync_tasks.push(sync_task);
        }
    }

    /// Returns the last path component of a path string.
    ///
    /// If the path ends with a slash, remove it to get the last path component
    /// (e.g. "/aa/bb/cc/" -> "cc").
    /// If the path is just a slash, return it as is.
    /// If the path is empty, return a default value.
    /// Otherwise, return the last path component.
    ///
    /// # Arguments
    ///
    /// * `path_str` - The path string to extract the last path component from.
    ///
    /// # Returns
    ///
    /// The last path component of the path string.
    fn get_last_path_component(path_str: &str) -> Option<String> {
        let normalized_path = if path_str.ends_with('/') && path_str.len() > 1 {
            &path_str[..path_str.len() - 1]
        } else {
            path_str
        };

        Path::new(normalized_path)
            .file_name()
            .and_then(|os_str| os_str.to_str())
            .map(|s| {
                if path_str.ends_with('/') {
                    format!("{}/", s)
                } else {
                    s.to_string()
                }
            })
    }

    /// Returns the path string without the last path component.
    ///
    /// If the path ends with a slash, remove it to get the parent directory
    /// (e.g. "/aa/bb/cc/" -> "/aa/bb/").
    /// If the path is just a slash, return it as is.
    /// If the path is empty, return a default value.
    /// Otherwise, return the path without the last path component.
    ///
    /// # Arguments
    ///
    /// * `path_str` - The path string to remove the last path component from.
    ///
    /// # Returns
    ///
    /// The path string without the last path component.
    fn remove_last_path_component(path_str: &str) -> Option<String> {
        let path = Path::new(path_str);

        // If the path ends with a slash, remove it to get the parent directory
        let clean_path = if path_str.ends_with('/') && path_str.len() > 1 {
            Path::new(&path_str[..path_str.len() - 1])
        } else {
            path
        };

        // Get the parent directory of the path
        clean_path.parent().map(|p| {
            let result = p.to_path_buf();

            // Ensure trailing slash
            let mut result_str = result.to_string_lossy().to_string();
            if !result_str.ends_with('/') {
                result_str.push('/');
            }
            result_str
        })
    }

    /// Extracts the ZIP file into a folder next to the ZIP file.
    /// The folder will have the same name as the ZIP file
    /// (without the extension).
    ///
    /// # Returns
    ///
    /// An `io::Result` indicating the success of the operation.
    pub fn extract_zip_to_folder(&mut self) -> io::Result<()> {
        let zip_file_path = &self.config.zip_path;
        let zip_path = Path::new(zip_file_path);

        // Ensure the ZIP file exists
        if !zip_path.exists() || !zip_path.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("ZIP file '{}' not found", zip_file_path),
            ));
        }

        // Determine the output folder name
        let output_folder = zip_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(zip_path.file_stem().unwrap_or_default());

        // Create the output folder if it doesn't exist
        if !output_folder.exists() {
            fs::create_dir_all(&output_folder)?;
        }

        // Open the ZIP file
        let file = fs::File::open(zip_file_path)?;
        let reader = BufReader::new(file);
        let mut archive = ZipArchive::new(reader)?;

        // Extract each file in the ZIP archive
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let out_path = output_folder.join(file.name());

            if file.is_dir() {
                fs::create_dir_all(&out_path)?;
            } else {
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut outfile = fs::File::create(&out_path)?;
                io::copy(&mut file, &mut outfile)?;
            }
        }

        println!(
            "ZIP file '{}' extracted to folder '{}'",
            zip_file_path,
            output_folder.display()
        );

        // Set the output folder
        self.outputfolder = Some(output_folder.to_string_lossy().to_string());

        Ok(())
    }

    /// Checks the ZIP archive for new files that are not handled by the config.
    fn check_zip_for_new_files(&self) {
        // TODO: Implement this method
    }

    /// Gathers metadata for the files in the ZIP archive and the extract paths.
    fn sync_files(&self) {
        println!("{}", self.outputfolder.as_ref().unwrap());

        // Loop over all sync tasks
        for task in &self.sync_tasks {
            // println!("Fange an mit: {:#?}", task);

            // Check if the file is in the zip
            let zip_path = Path::new(&self.outputfolder.as_ref().unwrap())
                        .join(Path::new(&task.zip_path));
            let mut extract_path = Path::new(&task.extract_path).to_path_buf();

            // Check if the file is in the zip
            if zip_path.exists() {
                println!("{}", format!(
                    "{}    ---->    {}",
                    &zip_path.display(), &extract_path.display()
                ).green().bold());
            } else {
                println!("{}", format!(
                    "Skipping {}    ---->    {}",
                    &zip_path.display(), &extract_path.display()
                ).red().bold());
                continue;
            }

            // Directory or file?
            if zip_path.is_file() {
                // If extract_path is an existing directory, append the
                // filename to it
                if extract_path.exists() && extract_path.is_dir() {
                    if let Some(file_name) = zip_path.file_name() {
                        extract_path = extract_path.join(file_name);
                    }
                }

                // Create target directory if it doesn't exist
                if let Some(parent) = extract_path.parent() {
                    fs::create_dir_all(parent)
                    .expect("Couldn't create target directory!");
                }

                // Set copy options with overwrite enabled
                let mut file_options = fs_extra::file::CopyOptions::new();
                file_options.overwrite = true;  // Enable overwrite

                // Datei kopieren
                fs_extra::file::copy(&zip_path, &extract_path, &file_options)
                    .expect("File could not be copied!");
            } else if zip_path.is_dir() {
                // Create target directory if it doesn't exist
                fs::create_dir_all(&extract_path)
                .expect("Couldn't create target directory!");

                // Set directory copy options
                let mut options = CopyOptions::new();
                options.overwrite = true;
                options.copy_inside = true;

                // Copy directory
                let from_paths = vec![zip_path.clone()];
                copy_items(&from_paths, &extract_path, &options)
                    .expect("Directory could not be copied!");
            } else {
                eprintln!("{}", format!(
                    "Path is neither a dir nor a file: {}", zip_path.display()
                ).yellow().bold());
            }
        }
    }
}