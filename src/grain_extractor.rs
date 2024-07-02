// File: grain_extractor.rs
// This file contains functionality for grain extraction and analysis.

use audiorust::{
    analysis::Analysis,
    grain, 
    spectrum::{generate_window, WindowType, rfft, complex_to_polar_rfft}
};

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
pub fn analyze_grains(file_name: &String, audio: &Vec<f64>, grain_frames: Vec<(usize, usize)>, window_type: WindowType, max_window_length: usize, sample_rate: u32, fft_size: usize) -> Result<Vec<GrainEntry>, GrainError> {
    let mut analysis_vec: Vec<GrainEntry> = Vec::with_capacity(grain_frames.len());
    let mut grains: Vec<Vec<f64>> = Vec::with_capacity(grain_frames.len());

    // Verify grain size
    for i in 0..grain_frames.len() {
        let grain_size = grain_frames[i].1 - grain_frames[i].0;
        if grain_size > fft_size {
            return Err(GrainError::GrainTooLong(String::from(format!("Grain {} is too long. The FFT size is {}, but the grain len is {}.", i, fft_size, grain_size))));
        }
    }
    
    // Extract the grains
    if grain_frames.len() > 0 {
        let window = generate_window(window_type, usize::min(max_window_length, grain_frames[0].1 - grain_frames[0].0));
        for i in 0..grain_frames.len() {
            let mut grain = audio[grain_frames[i].0..grain_frames[i].1].to_vec();
            for j in 0..window.len() / 2 {
                grain[j] *= window[j];
            }
            let mut idx = grain.len() - (window.len() - window.len() / 2);
            for j in window.len() / 2..window.len() {
                grain[idx] *= window[j];
                idx += 1;
            }
            grains.push(grain);
        }
    }

    // Analyze the grains
    for i in 0..grains.len() {
        // Zero pad the grain
        let zeros = vec![0.0; fft_size - grains[i].len()];
        grains[i].extend(zeros);
        audiorust::operations::adjust_level(&mut grains[i], -6.0);

        // Compute spectrum and analyze the grain
        let spectrum = rfft(&grains[i], fft_size);
        let (magnitude_spectrum, _) = complex_to_polar_rfft(&spectrum);
        let grain_analysis = audiorust::analysis::analyzer(&magnitude_spectrum, fft_size, sample_rate);
        let grain_entry: GrainEntry = GrainEntry{
            file: file_name.clone(),
            start_frame: grain_frames[i].0,
            end_frame: grain_frames[i].1,
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
        analysis_vec.push(grain_entry);
    }

    Ok(analysis_vec)
}
