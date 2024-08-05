use std::path::Path;
mod grain_extractor;
mod io;
mod sqlite;

// The maximum audio chunk length. Files that are longer will be split up into smaller
// chunks for more efficient multithreaded processing.
const MAX_AUDIO_SIZE: usize = 44100 * 120;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut valid_config = false;
    if args.len() == 2 {
        if Path::new(&args[1]).exists() {
            valid_config = true;
        }
    }
    
    if !valid_config {
        println!("Grain Processor\n--------------------------------------------------------\nUsage: grain_processor path_to_config.json");
    } else {
        let mut config = io::read_config(&args[1]);
        
        // the number of cpu cores available for the thread pool
        if config.max_num_threads < 1 {
            config.max_num_threads = match std::thread::available_parallelism() {
                Ok(x) => x.get(),
                Err(_) => 1
            };
        }

        println!("Grain Processor\n--------------------------------------------------------\nDatabase path: {}\nAudio path: {}\nMax audio chunk size: {}\nMax threads: {}", 
            config.database_path, config.audio_source_directory, config.max_audio_chunk_size, config.max_num_threads);

        // Create the database if it doesn't exist
        if !Path::new(&config.database_path).exists() {
            match sqlite::create_schema(&config.database_path) {
                Ok(_) => (),
                Err(err) => {
                    println!("Error creating database schema: {}", err.to_string());
                    return;
                }
            }
        }

        grain_extractor::process_grains(&config, MAX_AUDIO_SIZE);

        println!("Done");
    }
}
