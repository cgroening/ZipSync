use std::fs::{self, File, Metadata};
use std::path::Path;
use std::io::{BufReader, Error, BufWriter, Read, Write, Seek};
use std::collections::{HashSet, HashMap};
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter, ZipArchive};
use colored::*;

use crate::model::config::Config;
use crate::model::copy_task::CopyTask;


pub struct ZipCreatorController {
    /// The configuration object which holds the settings for the application
    /// and the copy tasks.
    config: Config,

    /// A HashMap for storing items that were missing during ZIP creation.
    missing_items: HashMap<String, String>
}

impl ZipCreatorController {
    pub fn new(config: Config) -> ZipCreatorController {
        ZipCreatorController {
            config,
            missing_items: HashMap::new()
        }
    }

    /// Starts the ZIP creation process.
    pub fn start(&mut self) {
        self.create_zip().unwrap();
    }

    /// Creates a ZIP archive including the folders and files specified in
    /// the config.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the ZIP file was successfully created.
    /// * `Err(zip::result::ZipError)` - If an error occurs during ZIP file
    ///   creation.
    fn create_zip(&mut self) -> zip::result::ZipResult<()> {
        let zip_file_path = self.config.zip_path.clone(); // Path of the .zip
        let file = File::create(&zip_file_path)?; // Create the file
        let writer = BufWriter::new(file);        // For efficient file writing
        let mut zip = ZipWriter::new(writer);     // ZIP archive writer

        // Loop copy tasks
        for task in self.config.copy_tasks.clone().iter() {
            let source_path = Path::new(&task.source);
            println!("Processing: {}", &task.source);

            // Check if source exists; if not skip iteration
            if !source_path.exists() {
                self.store_missing(task.clone(), TaskError::PathNotFound);
                continue;
            }

            // Get metadata of the source path and add item to zip
            let metadata = match fs::metadata(source_path) {
                Ok(meta) => meta,
                Err(_e) => {
                    self.store_missing(task.clone(), TaskError::MetadataError);
                    continue;
                }
            };
            self.add_item_to_zip(&mut zip, task.clone(), metadata);
        }

        // Finish the last file and write all other zip-structures
        zip.finish()?;
        self.display_task_completed_message(zip_file_path.as_str());

        Ok(())
    }

    /// Stores a missing item path in the HashMap and prints and error message.
    ///
    /// # Arguments
    ///
    /// * `task` - The CopyTask that contains the path of the missing item.
    fn store_missing(
        &mut self, task: CopyTask, error_type: TaskError
    ) {
        // Add item to missing items HashMap
        self.missing_items.insert(
            task.source.to_string(),
            task.target.to_string()
        );

        // Print error message
        let error_message = match error_type {
            TaskError::PathNotFound => { "Path not found" },
            TaskError::MetadataError => { "Error reading metadata" },
            TaskError::PathNotFileOrFolder => { "Path not of file or folder" }
            TaskError::FileCopyError => { "Error copying file" }
            TaskError::FolderCopyError  => { "Error copying folder" }
        };

        println!(
            "{}",
            format!("!!! {}: {}", error_message, task.source).red().bold()
        );
    }

    /// Adds a file or folder to the ZIP archive.
    /// If the source is a file, it will be added to the ZIP archive.
    /// If the source is a folder, all files and subfolders will be added.
    ///
    /// # Arguments
    ///
    /// * `zip` - Mutable reference to the ZipWriter.
    /// * `task` - The copy task that contains the source and target paths.
    /// * `metadata` - The metadata of the source path.
    fn add_item_to_zip<W: Write + Seek>(
        &mut self, zip: &mut ZipWriter<W>, task: CopyTask, metadata: Metadata
    ) {
        let source_path = Path::new(&task.source);

        // Check if source is a file or directory
        if metadata.is_file() {
                // Get filename and target path
                let filename = source_path.file_name().unwrap()
                                                 .to_string_lossy().to_string();

                let target_path = Self::get_target_file_path(
                    filename.clone(), task.clone()
                );

                // Add file to ZIP
                println!("Adding file: {} -> {}", task.source, target_path);
                if let Err(_e) = Self::add_file_to_zip(
                    zip, &task.source, &target_path
                ) {
                    self.store_missing(task.clone(), TaskError::FileCopyError);
                }
            } else if metadata.is_dir() {
                if task.target.is_empty() {
                    // If no target is specified, add the whole folder
                    println!("Adding folder: {}", task.source);
                    if let Err(_e) = Self::add_directory(zip, &task.source, "")
                    {
                        self.store_missing(
                            task.clone(), TaskError::FolderCopyError
                        );
                    }
                } else {
                    // Add folder with target path
                    println!("Adding folder with target path: {} -> {}",
                             task.source, task.target);
                    if let Err(_e) = Self::add_directory(
                        zip, &task.source, &task.target
                    ) {
                        self.store_missing(
                            task.clone(), TaskError::FolderCopyError
                        );
                    }
                }
            } else {
                // Path is neither file nor directory; i. e. Symlink etc.
                self.store_missing(task.clone(), TaskError::PathNotFileOrFolder);
            }
    }

    /// Returns the target file path for a file based on the target path in the
    /// copy task.
    ///
    /// # Arguments
    ///
    /// * `filename` - The name of the file.
    /// * `task` - The copy task that contains the target path.
    ///
    /// # Returns
    ///
    /// * `String` - The target file path.
    fn get_target_file_path(filename: String, task: CopyTask) -> String {
        if task.target.is_empty() {
            // Use file name if target path is empty
            filename
        } else if task.target.ends_with('/') {
            // If target path ends with a slash, append the file name
            format!("{}{}", task.target, filename)
        } else if Path::new(&task.target).extension().is_none() {
            // If the target doesn't have a file extension, treat it as a
            // directory path
            format!("{}/{}", task.target, filename)
        } else {
            // Use target as full path
            task.target.clone()
        }
    }

    /// Displays a message that the ZIP files was created and checks if all
    /// files and folders are in it. Prints a message depending on the result.
    ///
    /// # Arguments
    ///
    /// * `zip_file_path` - The path of the ZIP file that was created.
    fn display_task_completed_message(&mut self, zip_file_path: &str) {
        println!(
            "{}",
            format!(
                "ZIP file '{}' created!", &zip_file_path
            ).green().bold()
        );

        // Check ZIP: Are all files and folders in the ZIP?
        match self.check_zip() {
            Ok(true) => {
                println!(
                    "{}",
                    "Check sucessfull: The ZIP file contains all expected \
                    files.".green().bold()
                        )
            },
            Ok(false) => {
                println!(
                    "{}",
                    "Error during ZIP file check: It does NOT contain all \
                     expected files.".red().bold())
            },
            Err(e) => {
                println!(
                    "{}",
                    format!(
                        "Error while check the ZIP file: {}", e
                    ).red().bold()
                )
            },
        }
    }

    /// Adds a single file to the ZIP archive.
    ///
    /// # Arguments
    ///
    /// * `zip` - Mutable reference to the ZipWriter.
    /// * `file_path` - Path to the file to be added.
    /// * `zip_path` - Path within the ZIP archive where the file should be
    ///                stored.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the file was successfully added.
    /// * `Err(zip::result::ZipError)` - If an error occurs.
    fn add_file_to_zip<T: Write + Seek>(
        zip: &mut ZipWriter<T>,
        file_path: &str,
        zip_path: &str,
    ) -> zip::result::ZipResult<()> {
        // Create directories in ZIP if necessary (this avoids errors)
        if let Some(parent) = Path::new(zip_path).parent() {
            let dir = parent.to_string_lossy().to_string();
            if !dir.is_empty() {
                zip.add_directory::<_, ()>(
                    format!("{}/", dir), FileOptions::default()
                )?;
            }
        }

        // Read file
        let mut file = File::open(file_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        // Add file to ZIP
        zip.start_file::<_, ()>(
            zip_path,
            FileOptions::default().compression_method(
                CompressionMethod::Deflated
            )
        )?;
        zip.write_all(&buffer)?;

        Ok(())
    }

    /// Adds a directory to the ZIP archive.
    ///
    /// # Arguments
    ///
    /// * `zip` - Mutable reference to the ZipWriter.
    /// * `dir_path` - The path to the directory to add.
    /// * `target_prefix` - The target prefix in the ZIP archive.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the directory was successfully added.
    /// * `Err(zip::result::ZipError)` - If an error occurs.
    fn add_directory<T: Write + Seek>(
        zip: &mut ZipWriter<T>,
        dir_path: &str,
        target_prefix: &str
    ) -> zip::result::ZipResult<()> {
        // Extract the directory name from the path
        let dir_name = Path::new(dir_path)
            .file_name()
            .map_or(
                dir_path.to_string(), |name| name.to_string_lossy().into_owned()
            );

        // Determine the target directory name in the ZIP
        let target_dir_name = if target_prefix.is_empty() {
            dir_name
        } else {
            target_prefix.to_string()
        };

        // Add main directory to ZIP with trailing "/"
        println!("Adding directory: '{}'", target_dir_name);
        zip.add_directory::<_, ()>(
            format!("{}/", target_dir_name), FileOptions::default()
        )?;

        // Add all subdirectories and files recursively
        Self::add_directory_recursively(zip, dir_path, &target_dir_name)?;

        Ok(())
    }

    /// Recursively adds all files and subdirectories within a directory to the
    /// ZIP archive.
    ///
    /// # Arguments
    ///
    /// * `zip` - Mutable reference to the ZipWriter.
    /// * `base_path` - The base path of the directory being processed.
    /// * `zip_base` - The base path within the ZIP archive.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If all files and subdirectories were successfully added.
    /// * `Err(zip::result::ZipError)` - If an error occurs.
    fn add_directory_recursively<T: Write + Seek>(
        zip: &mut ZipWriter<T>,
        base_path: &str,
        zip_base: &str,
    ) -> zip::result::ZipResult<()> {
        for entry in fs::read_dir(base_path)? {
            let entry = entry?;
            let path = entry.path();
            let name = path.file_name().unwrap().to_string_lossy();
            let zip_path = if zip_base.ends_with('/') {
                format!("{}{}", zip_base, name)
            } else {
                format!("{}/{}", zip_base, name)
            };

            // Check if entry is a directory or file
            if path.is_dir() {
                // Recursively go into the directory
                zip.add_directory::<_, ()>(
                    format!("{}/", zip_path), FileOptions::default()
                )?;
                Self::add_directory_recursively(
                    zip, path.to_str().unwrap(), &zip_path
                )?;
            } else if path.is_file() {
                // Add file to ZIP
                let mut file = File::open(&path)?;
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;

                zip.start_file::<_, ()>(
                    &zip_path,
                    FileOptions::default().compression_method(
                        CompressionMethod::Deflated
                    )
                )?;
                zip.write_all(&buffer)?;
            }
        }

        Ok(())
    }

    /// Checks if all files were correctly stored in the ZIP archive.
    ///
    /// # Arguments
    ///
    /// * `config` - Reference to the configuration.
    /// * `missing_items` - HashMap of items that were missing during
    ///                     ZIP creation.
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - If all expected files are in the ZIP archive.
    /// * `Ok(false)` - If some expected files are missing from the ZIP archive.
    /// * `Err(Error)` - If an error occurs during the check.
    fn check_zip(&mut self) -> Result<bool, Error> {
        // Collect all file paths from the ZIP
        let zip_files = Self::get_file_list_from_zip(&self.config.zip_path)?;

        // Check all copy tasks
        for task in &self.config.copy_tasks {
            // Skip iteration if the task was marked as missing during creation
            if self.check_missing(&task.source, &task.target) { continue; }

            // Check if the source path is a file or directory
            let path = Path::new(&task.source);
            println!("Checking task: Source={}, Target={}",
                     task.source, task.target);

            // Check file or directory
            if path.is_file() {
                if Self::check_if_file_in_zip(
                    zip_files.clone(), task.clone(), &path
                ) {
                    return Ok(false);
                }
            } else if path.is_dir() {
                if Self::check_if_directory_in_zip(
                    zip_files.clone(), task.clone(), &path
                ) {
                    return Ok(false);
                }
            } else {
                // Path not found
                println!(
                    "{}",
                    format!(
                        "!!! The path doesn't exist or is neither a directory \
                        nor a file: '{}'", task.source
                    ).red().bold()
                );
                return Ok(false);
            }
        }

        // Print a summary of ignored items during the check
        if !self.missing_items.is_empty() {
            println!("\n{}", "Ignored items during the check:".yellow().bold());
            for (source, target) in &self.missing_items {
                println!(
                    "{}",
                    format!("  - Source: '{}', Target: '{}'",
                            source, target).yellow());
            }
        }

        Ok(true)
    }

    /// Returns a list of all files in the ZIP archive.
    ///
    /// # Returns
    ///
    /// * `Ok(HashSet<String>)` - A HashSet with all file paths in the ZIP.
    /// * `Err(Error)` - If an error occurs during the operation.
    pub fn get_file_list_from_zip(zip_path: &str)
    -> Result<HashSet<String>, Error> {
        // Open the ZIP file that was just created
        let file = File::open(zip_path)?;
        let reader = BufReader::new(file);
        let mut archive = ZipArchive::new(reader)?;

        // Collect all file paths from the ZIP
        let mut zip_files = HashSet::new();

        println!("Files in the ZIP archive:");
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let name = file.name().to_string();

            // // Remove trailing slash
            // let name = if name.ends_with('/') {
            //     name[..name.len() - 1].to_string()
            // } else {
            //     name
            // };

            println!("  - '{}'", name);
            zip_files.insert(name);
        }

        zip_files = Self::add_reconstructed_dirs_to_file_list(zip_files);
        Ok(zip_files)
    }

    /// Adds reconstructed directories to the file list.
    /// If a file is in a directory that is not in the ZIP archive, the
    /// directory is added to the file list. This is necessary for directories
    /// that were not explicitly added to the ZIP archive.
    ///
    /// # Arguments
    ///
    /// * `zip_files` - A HashSet with all file paths in the ZIP archive.
    ///
    /// # Returns
    ///
    /// * `HashSet<String>` - A HashSet with all file paths in the ZIP archive
    ///                       and the reconstructed directories.
    pub fn add_reconstructed_dirs_to_file_list(mut zip_files: HashSet<String>)
    -> HashSet<String> {
        let mut reconstructed_dirs = HashSet::new();

        for path in &zip_files {
            let mut current = Path::new(path);
            while let Some(parent) = current.parent() {
                // Skip if directory path is empty or already in the set
                if parent.to_string_lossy().is_empty() ||
                   reconstructed_dirs.contains(&parent.to_string_lossy()
                                               .to_string())
                {
                    break;
                }

                println!("  - '{}'", parent.to_string_lossy().to_string());
                reconstructed_dirs.insert(parent.to_string_lossy().to_string());
                current = parent;
            }
        }
        zip_files.extend(reconstructed_dirs);
        zip_files
    }

    /// Checks if a task was marked as missing during ZIP creation.
    /// If the task was marked as missing, a message is printed and the
    /// function returns true.
    ///
    /// # Arguments
    /// * `task_source` - The source path of the task.
    /// * `task_target` - The target path of the task.
    ///
    /// # Returns
    /// * `bool` - True if the task was marked as missing, false otherwise.
    fn check_missing(&self, task_source: &str, task_target: &str) -> bool {
        if self.missing_items.contains_key(task_source) {
            println!(
                "{}",
                format!(
                    "Ignoring during check: '{}' -> '{}' \
                     (was marked as missing during creation)",
                    task_source, task_target
                ).yellow().bold()
            );
            return true;
        } else {
            return false;
        }
    }

    /// Checks if a file is in the ZIP archive.
    ///
    /// # Arguments
    ///
    /// * `zip_files` - A HashSet with all file paths in the ZIP archive.
    /// * `task` - The copy task that contains the source and target paths.
    /// * `path` - The path of the file to check.
    ///
    /// # Returns
    ///
    /// * `bool` - True if the file is not in the ZIP archive, false otherwise.
    fn check_if_file_in_zip(
        zip_files: HashSet<String>,
        task: CopyTask,
        path: &Path
    ) -> bool {
        // Get the filename
        let filename = path.file_name().unwrap().to_string_lossy().to_string();

        // Check if the file is in the ZIP
        let expected_path = Self::get_target_file_path(
            filename.clone(), task.clone()
        );
        println!("Loooking for file: '{}'", expected_path);

        // Check if the file is in the ZIP archive
        if !zip_files.contains(&expected_path) {
            println!("{}", format!("!!! Datei nicht gefunden in ZIP: '{}' \
                                   (Quelle: '{}')",
                                   expected_path, task.source).red().bold());
            return true;
        } else {
            return false;
        }
    }

    /// Checks if a directory is in the ZIP archive.
    ///
    /// # Arguments
    ///
    /// * `zip_files` - A HashSet with all file paths in the ZIP archive.
    /// * `task` - The copy task that contains the source and target paths.
    /// * `path` - The path of the directory to check.
    ///
    /// # Returns
    ///
    /// * `bool` - True if the directory is not in the ZIP archive, false
    ///            otherwise.
    fn check_if_directory_in_zip(
        zip_files: HashSet<String>,
        task: CopyTask,
        path: &Path
    ) -> bool {
        // Determine the expected directory path in the ZIP
        let expected_dir = if task.target.is_empty() {
            // If target is empty, use the directory name
            let dir_name = path.file_name()
                .map_or(
                    path.to_string_lossy().to_string(),
                    |name| name.to_string_lossy().to_string()
                );
            format!("{}/", dir_name)
        } else {
            // If target is not empty, use the target path
            if task.target.ends_with('/') {
                task.target.clone()
            } else {
                format!("{}/", task.target)
            }
        };

        // Check if the directory is in the ZIP archive
        println!("Looking for directory: '{}'", expected_dir);
        let dir_exists = zip_files.contains(&expected_dir) ||
                        zip_files.iter().any(|p| p.starts_with(&expected_dir));

        if !dir_exists {
            println!(
                "{}",
                format!(
                    "!!! Verzeichnis nicht gefunden in ZIP: '{}'\
                    (Quelle: '{}')",
                    expected_dir, task.source
                ).red().bold()
            );
            println!("Verfügbare Verzeichnisse in ZIP:");
            for p in zip_files.iter().filter(|p| p.ends_with("/")) {
                println!("  - '{}'", p);
            }
            return true;
        } else {
            return false;
        }
    }
}


/// Enumeration for the error types "PathNotFound" and "MetadataError".
///
/// The "PathNotFound" error is used when a path does not exist.
/// The "MetadataError" error is used when metadata cannot be retrieved.
#[derive(Debug)]
enum TaskError {
    PathNotFound,
    MetadataError,
    PathNotFileOrFolder,
    FileCopyError,
    FolderCopyError,
}