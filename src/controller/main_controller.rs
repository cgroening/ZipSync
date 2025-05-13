use std::fs::File;
use std::io::{BufReader, Error};
use colored::*;

use crate::model::config::Config;
use crate::controller::zip_creator_controller::ZipCreatorController;
use crate::controller::zip_reader_controller::ZipReaderController;


/// The main controller of the application.
pub struct MainController {
    /// The configuration object which holds the settings for the application
    /// and the copy tasks.
    config: Config,
}

impl MainController {
    /// Creates a new `MainController` instance.
    ///
    /// # Returns
    ///
    /// * `MainController` - A new instance of the main controller.
    pub fn new() -> Self {
        let controller: MainController = Self {
            config: Config::new()
        };
        controller
    }

    /// Starts the main logic of the application, including reading the config
    /// and executing the zip operations.
    /// TODO: implement sync back from ZIP file
    pub fn start(&mut self) {
        // Read config file
        match Self::read_config() {
            Ok(config) => {
                println!("=== {} ===", "ZipSync".bold());

                // Check copy direction
                if config.direction == "to_zip" {
                    // Copy files to the ZIP archive
                    println!("Zip path:  {}", self.config.zip_path);
                    println!("Direction: {}\n", self.config.direction);

                    let mut zip_creator = ZipCreatorController::new(config);
                    zip_creator.start();
                } else if config.direction == "from_zip" {
                    // Copy files from the ZIP archive to the paths in the config
                    let mut zip_reader = ZipReaderController::new(config);
                    zip_reader.start();
                } else {
                    println!("{}", format!("Unknown copy direction: {}",
                    self.config.direction).red().bold());
                }
            }
            Err(e) => {
                println!("Error reading JSON file: {}", e);
            }
        }
    }

    /// Reads the configuration file and returns a `Config` instance.
    ///
    /// # Returns
    ///
    /// * `Ok(Config)` - Configuration object parsed from the JSON file.
    /// * `Err(Error)` - If the file cannot be read or parsed.
    fn read_config() -> Result<Config, Error> {
        let file_path = "data/config.json";
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let mut config: Config = serde_json::from_reader(reader)?;
        config.clean();
        Ok(config)
    }
}