use audiorust;
use std::path::Path;
mod grain_extractor;
mod io;
mod sqlite;

fn main() {
    let grain_size = 10000;
    let grain_spacing = grain_size * 2;
    let fft_size = 16384;
    let db = String::from("data/db.sqlite");

    // Create the database if it doesn't exist
    if !Path::new(&db).exists() {
        sqlite::create_schema(&db).unwrap();
    }

    let path = String::from("D:\\Recording\\Samples\\freesound\\creative_commons_0");
    let audio = io::find_files(&path);
    for file in audio {
        let a = audiorust::read(&file).unwrap();
        let frames = grain_extractor::extract_grain_frames(&a.samples[0], grain_size, grain_spacing, 20000);
        let grains = grain_extractor::analyze_grains(&file, &a.samples[0], frames, audiorust::spectrum::WindowType::Hanning, 5000, a.sample_rate, fft_size).unwrap();
        sqlite::insert_grains(&db, &grains).unwrap();
    }
}
