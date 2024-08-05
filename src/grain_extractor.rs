// File: grain_extractor.rs
// This file contains functionality for grain extraction and analysis.

use aus::{
    analysis::Analysis,
    grain, 
    spectrum::{rfft, complex_to_polar_rfft}
};
use biquad::*;
use crate::{sqlite, io};
use std::path::Path;
use std::sync::mpsc;
use threadpool::ThreadPool;


#[derive(Debug, Clone)]
pub enum GrainError {
    GrainTooShort(String),
    GrainTooLong(String)
}

#[derive(Debug, Clone)]
pub struct GrainEntry {
    pub file: String,
    pub start_frame: usize,
    pub end_frame: usize,
    pub sample_rate: u32,
    pub grain_duration: f64,
    pub energy: f64,
    pub pitch_estimation: f64,
    pub midi: f64,
    pub spectral_centroid: f64,
    pub spectral_entropy: f64,
    pub spectral_flatness: f64,
    pub spectral_kurtosis: f64,
    pub spectral_roll_off_50: f64,
    pub spectral_roll_off_75: f64,
    pub spectral_roll_off_90: f64,
    pub spectral_roll_off_95: f64,
    pub spectral_skewness: f64,
    pub spectral_slope: f64,
    pub spectral_slope_0_1_khz: f64,
    pub spectral_slope_1_5_khz: f64,
    pub spectral_slope_0_5_khz: f64,
    pub spectral_variance: f64
}

/// Computes a basic similarity measurement between two grains. Measurement is between 0.0 (no similarity) and 1.0 (identity).
pub fn similarity(grain1: &GrainEntry, grain2: &GrainEntry) -> f64 {
    let mut similarity = 0.0;
    similarity += f64::max(1.0 - f64::abs((grain1.spectral_centroid - grain2.spectral_centroid) / grain1.spectral_centroid), 0.0);
    similarity += f64::max(1.0 - f64::abs((grain1.spectral_entropy - grain2.spectral_entropy) / grain1.spectral_entropy), 0.0);
    similarity += f64::max(1.0 - f64::abs((grain1.spectral_flatness - grain2.spectral_flatness) / grain1.spectral_flatness), 0.0);
    similarity += f64::max(1.0 - f64::abs((grain1.spectral_kurtosis - grain2.spectral_kurtosis) / grain1.spectral_kurtosis), 0.0);
    similarity += f64::max(1.0 - f64::abs((grain1.spectral_roll_off_50 - grain2.spectral_roll_off_50) / grain1.spectral_roll_off_50), 0.0);
    similarity += f64::max(1.0 - f64::abs((grain1.spectral_roll_off_75 - grain2.spectral_roll_off_75) / grain1.spectral_roll_off_75), 0.0);
    similarity += f64::max(1.0 - f64::abs((grain1.spectral_roll_off_90 - grain2.spectral_roll_off_90) / grain1.spectral_roll_off_90), 0.0);
    similarity += f64::max(1.0 - f64::abs((grain1.spectral_roll_off_95 - grain2.spectral_roll_off_95) / grain1.spectral_roll_off_95), 0.0);
    similarity += f64::max(1.0 - f64::abs((grain1.spectral_skewness - grain2.spectral_skewness) / grain1.spectral_skewness), 0.0);
    similarity += f64::max(1.0 - f64::abs((grain1.spectral_slope - grain2.spectral_slope) / grain1.spectral_slope), 0.0);
    similarity += f64::max(1.0 - f64::abs((grain1.spectral_slope_0_1_khz - grain2.spectral_slope_0_1_khz) / grain1.spectral_slope_0_1_khz), 0.0);
    similarity += f64::max(1.0 - f64::abs((grain1.spectral_slope_1_5_khz - grain2.spectral_slope_1_5_khz) / grain1.spectral_slope_1_5_khz), 0.0);
    similarity += f64::max(1.0 - f64::abs((grain1.spectral_slope_0_5_khz - grain2.spectral_slope_0_5_khz) / grain1.spectral_slope_0_5_khz), 0.0);
    similarity += f64::max(1.0 - f64::abs((grain1.spectral_variance - grain2.spectral_variance) / grain1.spectral_variance), 0.0);
    similarity / 14.0
}

/// Checks to see if a grain has more than N consecutive zero samples in it.
/// This is useful for screening out silent grains.
pub fn check_zeros(grain: &Vec<f64>, num_consecutive_zeros: usize, effective_zero: f64) -> bool {
    let mut consecutive: usize = 0;
    for i in 0..grain.len() {
        if grain[i].abs() < effective_zero {
            consecutive += 1;
            if consecutive >= num_consecutive_zeros {
                return true;
            }
        } else {
            consecutive = 0;
        }
    }
    false
}

/// Extracts grains from an audio sequence.
/// You specify the grain size and spacing between grain onsets. 
/// If you don't want grain overlap, the spacing must be at least as large as the grain size.
pub fn extract_grain_frames(audio: &Vec<f64>, grain_size: usize, grain_spacing: usize, initial_offset: usize) -> Vec<(usize, usize)> {
    let mut grains: Vec<(usize, usize)> = Vec::new();
    let mut i = initial_offset;
    while i + grain_size < audio.len() {
        grains.push((i, i + grain_size));
        i += grain_spacing;
    }
    grains
}

/// Analyzes grains
/// Note: the fft size must be at least as large as the grain size!
pub fn analyze_grains(file_name: &str, audio: &Vec<f64>, grain_frames: Vec<(usize, usize)>, window_type: aus::WindowType, max_window_length: usize, sample_rate: u32, fft_size: usize) -> Result<Vec<GrainEntry>, GrainError> {
    let mut analysis_vec: Vec<GrainEntry> = Vec::with_capacity(grain_frames.len());
    let mut grains: Vec<Vec<f64>> = Vec::with_capacity(grain_frames.len());
    let mut filtered_audio = vec![0.0; audio.len()];

    // Filter the audio for checking super low frequencies.
    // This is necessary because some grains might only have very low frequencies, and this could cause
    // problems with consistent audio levels if they are filtered during the synthesis process.
    // Here we'll use a cutoff of 220 Hz, which should make super low frequencies almost completely disappear.
    let filter_type = Type::HighPass;
    let fs = Hertz::<f64>::from_hz(sample_rate as f64).unwrap();
    let cutoff = Hertz::<f64>::from_hz(220.0).unwrap();
    let coefs = Coefficients::<f64>::from_params(filter_type, fs, cutoff, Q_BUTTERWORTH_F64).unwrap();
    let mut filter = DirectForm2Transposed::<f64>::new(coefs);
    for i in 0..audio.len() {
        filtered_audio[i] = filter.run(audio[i]);
    }

    // For pyin
    const F_MIN: f64 = 50.0;
    const F_MAX: f64 = 800.0;

    // Verify grain size
    for i in 0..grain_frames.len() {
        let grain_size = grain_frames[i].1 - grain_frames[i].0;
        if grain_size > fft_size {
            return Err(GrainError::GrainTooLong(String::from(format!("Grain {} is too long. The FFT size is {}, but the grain len is {}.", i, fft_size, grain_size))));
        }
    }
    
    // Extract the grains
    if grain_frames.len() > 0 {
        let window = aus::generate_window(window_type, usize::min(max_window_length, grain_frames[0].1 - grain_frames[0].0));
        for i in 0..grain_frames.len() {
            let mut grain = audio[grain_frames[i].0..grain_frames[i].1].to_vec();
            let mut filtered_grain = filtered_audio[grain_frames[i].0..grain_frames[i].1].to_vec();
            for j in 0..window.len() / 2 {
                grain[j] *= window[j];
                filtered_grain[j] *= window[j];
            }
            let mut idx = grain.len() - (window.len() - window.len() / 2);
            for j in window.len() / 2..window.len() {
                grain[idx] *= window[j];
                filtered_grain[idx] *= window[j];
                idx += 1;
            }
            // If more than 12.5% of the samples in order are 0 for the filtered grain, we don't add the grain
            // We use the filtered grain because we don't want grains with only super low frequency content.
            if !check_zeros(&filtered_grain, filtered_grain.len() / 8, 1e-3) && !check_zeros(&filtered_grain, 50, 1e-5) {
                grains.push(grain);
            }
        }
    }

    // Analyze the grains
    for i in 0..grains.len() {
        // Zero pad the grain
        let zeros = vec![0.0; fft_size - grains[i].len()];
        grains[i].extend(zeros);
        aus::operations::adjust_level(&mut grains[i], -6.0);

        // Compute spectrum and analyze the grain
        let spectrum = rfft(&grains[i], fft_size);
        let (magnitude_spectrum, _) = complex_to_polar_rfft(&spectrum);
        let grain_analysis = aus::analysis::analyzer(&magnitude_spectrum, fft_size, sample_rate);
        let pitch_estimation = aus::analysis::pyin_pitch_estimator_single(&grains[i], sample_rate, F_MIN, F_MAX);
        let midi = aus::tuning::freq_to_midi(pitch_estimation);

        let grain_entry: GrainEntry = GrainEntry{
            file: file_name.to_string(),
            start_frame: grain_frames[i].0,
            end_frame: grain_frames[i].1,
            sample_rate: sample_rate,
            grain_duration: sample_rate as f64 / (grain_frames[i].1 - grain_frames[i].0) as f64,
            energy: aus::analysis::energy(&grains[i]),
            pitch_estimation: pitch_estimation,
            midi: midi,
            spectral_centroid: grain_analysis.spectral_centroid,
            spectral_entropy: grain_analysis.spectral_entropy,
            spectral_flatness: grain_analysis.spectral_flatness,
            spectral_kurtosis: grain_analysis.spectral_kurtosis,
            spectral_roll_off_50: grain_analysis.spectral_roll_off_50,
            spectral_roll_off_75: grain_analysis.spectral_roll_off_75,
            spectral_roll_off_90: grain_analysis.spectral_roll_off_90,
            spectral_roll_off_95: grain_analysis.spectral_roll_off_95,
            spectral_skewness: grain_analysis.spectral_skewness,
            spectral_slope: grain_analysis.spectral_slope,
            spectral_slope_0_1_khz: grain_analysis.spectral_slope_0_1_khz,
            spectral_slope_0_5_khz: grain_analysis.spectral_slope_0_5_khz,
            spectral_slope_1_5_khz: grain_analysis.spectral_slope_1_5_khz,
            spectral_variance: grain_analysis.spectral_variance
        };
        if i > 0 {
            //println!("similarity: {}", similarity(&analysis_vec[analysis_vec.len() - 1], &grain_entry));
        }
        analysis_vec.push(grain_entry);
    }

    Ok(analysis_vec)
}

/// Processes the grains. Reads audio files and extracts and analyzes grains.
pub fn process_grains(config: &io::GranulatorConfig, max_audio_size: usize) {
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
                    let mut end_idx = usize::min(x.num_frames, max_audio_size);
                    while start_idx < x.num_frames {
                        let _ = match tx_clone.send((file.clone(), x.sample_rate, x.samples[0][start_idx..end_idx].to_vec())) {
                            Ok(_) => (),
                            Err(_) => ()
                        };
                        start_idx = end_idx;
                        end_idx = usize::min(x.num_frames, start_idx + max_audio_size);
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
                let frames = extract_grain_frames(&chunk, grain_size, grain_spacing, 20000);
                // the fft size has to be at least as large as the grain size
                let mut fft_size: usize = 512;
                while fft_size < grain_size {
                    fft_size *= 2;
                }
                match analyze_grains(&chunk_name, &chunk, frames, aus::WindowType::Hanning, 5000, sample_rate, fft_size) {
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
}