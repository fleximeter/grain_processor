use audiorust;
use std::path::Path;
use std::thread;
use std::sync::mpsc;
use std::usize::MAX;
mod grain_extractor;
mod io;
mod sqlite;

const MAX_AUDIO_SIZE: usize = 44100 * 120;

fn main() {
    // Set up the multithreading
    let (tx, rx) = mpsc::channel();  // the message passing channel

    let grain_size = 10000;
    let grain_spacing = grain_size * 2;
    let fft_size = 16384;
    let db = String::from("data/grains.sqlite3");

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

    let path = String::from("D:\\Recording\\Samples\\freesound\\creative_commons_0\\granulation\\**\\*.wav");
    let audio = io::find_files(&path);
    
    // Read all the files, mix to mono, and split into smaller audio chunks for faster processing
    let mut audio_chunks: Vec<(String, u32, Vec<f64>)> = Vec::new();
    for file in audio {
        let tx_clone = tx.clone();
        thread::spawn(move || {
            let a = audiorust::read(&file);
            match a {
                Ok(mut x) => {
                    audiorust::mixdown(&mut x);
                    let mut start_idx = 0;
                    let mut end_idx = usize::min(x.samples.len(), MAX_AUDIO_SIZE);
                    while start_idx < x.samples.len() {
                        let _ = match tx_clone.send((file.clone(), x.sample_rate, x.samples[start_idx..end_idx][0].to_vec())) {
                            Ok(x) => x,
                            Err(_) => ()
                        };
                        start_idx = end_idx;
                        end_idx = usize::min(x.samples.len(), start_idx + MAX_AUDIO_SIZE);
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

    println!("Starting grain extraction for {} audio file chunks...", audio_chunks.len());
    let (tx, rx) = mpsc::channel();  // the message passing channel
    for chunk in audio_chunks {
        //println!("File: {}", file);
        let tx_clone = tx.clone();
        // Start the thread
        thread::spawn(move || {
            let frames = grain_extractor::extract_grain_frames(&chunk.2, grain_size, grain_spacing, 20000);
            let grains = grain_extractor::analyze_grains(&chunk.0, &chunk.2, frames, audiorust::spectrum::WindowType::Hanning, 5000, chunk.1, fft_size).unwrap();
            let _ = match tx_clone.send((chunk.0, grains)) {
                Ok(x) => x,
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

    println!("Done");
}
