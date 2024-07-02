// File: io.rs
// This file has IO operations.

use glob::glob;

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
