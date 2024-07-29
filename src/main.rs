use aus;
use std::path::Path;
use std::sync::mpsc;
use threadpool::ThreadPool;
mod grain_extractor;
mod io;
mod sqlite;

// The maximum audio chunk length. Files that are longer will be split up into smaller
// chunks for more efficient multithreaded processing.
const MAX_AUDIO_SIZE: usize = 44100 * 120;
const JSON_PATH: &str = "config.json";

fn main() {
    let mut config = io::read_config(JSON_PATH);
    
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

    let audio_file_list = io::find_audio(&config.audio_source_directory);
    println!("Found {} files", audio_file_list.len());
    
    // Read all the files, mix to mono, and split into smaller audio chunks for faster processing
    let mut audio_chunks: Vec<(String, u32, Vec<f64>)> = Vec::new();
    let pool = ThreadPool::new(config.max_num_threads);
    let (tx, rx) = mpsc::channel();  // the message passing channel
    for file in audio_file_list {
        let tx_clone = tx.clone();
        pool.execute(move || {
            let a = aus::read(&file);
            match a {
                Ok(mut x) => {
                    aus::mixdown(&mut x);
                    let mut start_idx = 0;
                    let mut end_idx = usize::min(x.num_frames, MAX_AUDIO_SIZE);
                    while start_idx < x.num_frames {
                        let _ = match tx_clone.send((file.clone(), x.sample_rate, x.samples[0][start_idx..end_idx].to_vec())) {
                            Ok(_) => (),
                            Err(_) => ()
                        };
                        start_idx = end_idx;
                        end_idx = usize::min(x.num_frames, start_idx + MAX_AUDIO_SIZE);
                    }
                },
                Err(_) => ()
            }
        });
    }

    // Drop the original sender. Once all senders are dropped, receiving will end automatically.
    drop(tx);

    // Collect the audio chunks
    for val in rx {
        audio_chunks.push(val);
    }

    pool.join();  // let all threads wrap up
    println!("Audio files loaded.");

    // Iterate through the grain specifications, extracting grains
    for grain_spec in config.grain_profiles.iter() {
        let grain_size = grain_spec["grain_size"];
        let grain_spacing = grain_spec["grain_spacing"];
        println!("-------------------------------------------\nGrain size: {}\nGrain spacing: {}\nStarting grain extraction for {} audio file chunks...", grain_size, grain_spacing, audio_chunks.len());
        let pool = ThreadPool::new(config.max_num_threads);
        let (tx, rx) = mpsc::channel();  // the message passing channel
        for chunk in audio_chunks.iter() {
            let chunk_name = chunk.0.clone();
            let sample_rate = chunk.1;
            let chunk = chunk.2.clone();
            
            let tx_clone = tx.clone();
            // Start the thread
            pool.execute(move || {
                let frames = grain_extractor::extract_grain_frames(&chunk, grain_size, grain_spacing, 20000);
                // the fft size has to be at least as large as the grain size
                let mut fft_size: usize = 512;
                while fft_size < grain_size {
                    fft_size *= 2;
                }
                match grain_extractor::analyze_grains(&chunk_name, &chunk, frames, aus::WindowType::Hanning, 5000, sample_rate, fft_size) {
                    Ok(grains) => {
                        match tx_clone.send((chunk_name.clone(), grains)) {
                            Ok(_) => (),
                            Err(_) => println!("Error sending grains in chunk of file {}", chunk_name)
                        }
                    },
                    Err(err) => println!("Error analyzing grains: {:?}", err)
                };
            });
        }

        // Drop the original sender. Once all senders are dropped, receiving will end automatically.
        drop(tx);

        // Collect the analysis vectors and sort them by thread id
        for (file, grains) in rx {
            match sqlite::insert_grains(&config.database_path, &grains) {
                Ok(_) => println!("Chunk of file {} done.", file),
                Err(err) => println!("Error in file {}: {}", file, err)
            }
        }

        pool.join();  // let all threads wrap up
    }

    println!("Done");
}
