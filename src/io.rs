// File: io.rs
// This file has IO operations.

use glob::glob;
use std::fs;
use serde_json;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct GranulatorConfig {
    pub database_path: String,
    pub audio_source_directory: String,
    pub grain_size: usize,
    pub grain_spacing: usize,
    pub max_audio_chunk_size: usize,
    pub max_num_threads: usize
}

/// Finds all files in a directory and its subdirectories
/// Takes a Unix file pattern
/// Returns a vector of file paths
pub fn find_files(directory: &String) -> Vec<String> {
    let mut file_paths: Vec<String> = Vec::new();
    let entries = glob(&directory);
    match entries {
        Ok(paths) => {
            for entry in paths {
                match entry {
                    Ok(path) => {
                        match path.to_str() {
                            Some(x) => file_paths.push(x.to_string()),
                            None => ()
                        }
                    },
                    Err(_) => ()
                };
            }
        },
        Err(_) => ()
    }
    file_paths
}

/// Reads the configuration for the granulator
pub fn read_config(config_file_path: &str) -> GranulatorConfig {
    let config_contents = match fs::read_to_string(config_file_path) {
        Ok(x) => x,
        Err(_) => String::from("")
    };
    let json_contents: GranulatorConfig = match serde_json::from_str(&config_contents){
        Ok(x) => x,
        Err(_) => GranulatorConfig{database_path: String::from("grains.sqlite3"), audio_source_directory: String::from("."), grain_size: 1024, grain_spacing: 2048, max_audio_chunk_size: 44100 * 60, max_num_threads: 0}
    };
    json_contents
}
