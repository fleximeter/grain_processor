use audiorust;
use std::path::Path;
use std::sync::mpsc;
use threadpool::ThreadPool;
mod grain_extractor;
mod io;
mod sqlite;

// The maximum audio chunk length. Files that are longer will be split up into smaller
// chunks for more efficient multithreaded processing.
const MAX_AUDIO_SIZE: usize = 44100 * 120;

fn main() {    
    let grain_size = 10000;  // grain duration in frames
    let grain_spacing = grain_size * 2;  // distance between grain onsets

    // the fft size has to be at least as large as the grain size
    let fft_size = f64::ceil(f64::log2(grain_size as f64)) as usize;
    
    let db = String::from("data/grains.sqlite3");  // the db path

    // the number of cpu cores available for the thread pool
    let num_cpus = match std::thread::available_parallelism() {
        Ok(x) => x.get(),
        Err(_) => 1
    };

    // Create the database if it doesn't exist
    if !Path::new(&db).exists() {
        match sqlite::create_schema(&db) {
            Ok(_) => (),
            Err(err) => {
                println!("Error creating database schema: {}", err.to_string());
                return;
            }
        }
    }

    let audio_source_path = String::from("D:\\Recording\\Samples\\freesound\\creative_commons_0\\granulation\\**\\*.wav");
    let audio_file_list = io::find_files(&audio_source_path);
    
    // Read all the files, mix to mono, and split into smaller audio chunks for faster processing
    let mut audio_chunks: Vec<(String, u32, Vec<f64>)> = Vec::new();
    let pool = ThreadPool::new(num_cpus);
    let (tx, rx) = mpsc::channel();  // the message passing channel
    for file in audio_file_list {
        let tx_clone = tx.clone();
        pool.execute(move || {
            let a = audiorust::read(&file);
            match a {
                Ok(mut x) => {
                    audiorust::mixdown(&mut x);
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

    println!("Starting grain extraction for {} audio file chunks...", audio_chunks.len());
    let pool = ThreadPool::new(num_cpus);
    let (tx, rx) = mpsc::channel();  // the message passing channel
    for chunk in audio_chunks {
        //println!("File: {}", file);
        let tx_clone = tx.clone();
        // Start the thread
        pool.execute(move || {
            let file = chunk.0.clone();
            let frames = grain_extractor::extract_grain_frames(&chunk.2, grain_size, grain_spacing, 20000);
            match grain_extractor::analyze_grains(&chunk.0, &chunk.2, frames, audiorust::spectrum::WindowType::Hanning, 5000, chunk.1, fft_size) {
                Ok(grains) => {
                    match tx_clone.send((chunk.0, grains)) {
                        Ok(_) => (),
                        Err(_) => println!("Error sending grains in chunk of file {}", file)
                    }
                },
                Err(_) => ()
            };
        });
    }

    // Drop the original sender. Once all senders are dropped, receiving will end automatically.
    drop(tx);

    // Collect the analysis vectors and sort them by thread id
    for (file, grains) in rx {
        match sqlite::insert_grains(&db, &grains) {
            Ok(_) => println!("Chunk of file {} done.", file),
            Err(err) => println!("Error in file {}: {}", file, err)
        }
    }

    pool.join();  // let all threads wrap up

    println!("Done");
}
