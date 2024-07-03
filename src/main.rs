use audiorust;
use std::path::Path;
use std::thread;
use std::sync::mpsc;
mod grain_extractor;
mod io;
mod sqlite;


fn main() {
    // Set up the multithreading
    let (tx, rx) = mpsc::channel();  // the message passing channel

    let grain_size = 10000;
    let grain_spacing = grain_size * 2;
    let fft_size = 16384;
    let db = String::from("data/db.sqlite");

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

    let path = String::from("D:\\Recording\\Samples\\freesound\\creative_commons_0\\wind_chimes\\eq\\**\\*.wav");
    let audio = io::find_files(&path);

    println!("Starting grain extraction for {} files...", audio.len());
    for file in audio {
        //println!("File: {}", file);
        let tx_clone = tx.clone();
        // Start the thread
        thread::spawn(move || {
            let a = audiorust::read(&file).unwrap();
            let frames = grain_extractor::extract_grain_frames(&a.samples[0], grain_size, grain_spacing, 20000);
            let grains = grain_extractor::analyze_grains(&file, &a.samples[0], frames, audiorust::spectrum::WindowType::Hanning, 5000, a.sample_rate, fft_size).unwrap();
            let _ = match tx_clone.send((file, grains)) {
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
            Ok(_) => println!("File {} done.", file),
            Err(err) => println!("Error in file {}: {}", file, err)
        }
    }

    println!("Done");
}
