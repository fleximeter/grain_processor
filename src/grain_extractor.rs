// File: grain_extractor.rs
// This file contains functionality for grain extraction and analysis.

use audiorust::{
    analysis,
    grain, 
    spectrum::{generate_window, WindowType, rfft, complex_to_polar_rfft}
};

/// Extracts grains from an audio sequence.
/// You specify the grain size and spacing between grain onsets. 
/// If you don't want grain overlap, the spacing must be at least as large as the grain size.
pub fn extract_grain_frames(audio: Vec<f64>, grain_size: usize, grain_spacing: usize, initial_offset: usize) -> Vec<(usize, usize)> {
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
pub fn analyze_grains(audio: &Vec<f64>, grain_frames: Vec<(usize, usize)>, window_type: WindowType, max_window_length: usize, sample_rate: u32, fft_size: usize) {
    let mut analysis: Vec<audiorust::analysis::Analysis> = Vec::with_capacity(grain_frames.len());
    let mut grains: Vec<Vec<f64>> = Vec::with_capacity(grain_frames.len());
    
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
        let zeros = vec![0.0; fft_size - grains[i].len()];
        grains[i].extend(zeros);
        let spectrum = rfft(&grains[i], fft_size);
        let (magnitude_spectrum, _) = complex_to_polar_rfft(&spectrum);
        let grain_analysis = audiorust::analysis::analyzer(&magnitude_spectrum, fft_size, sample_rate as u16);
        analysis.push(grain_analysis);
    }
}
