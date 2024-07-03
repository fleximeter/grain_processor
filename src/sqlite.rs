// File: sqlite.rs
// This file has database operations.

use rusqlite::{Connection, Result, params};
use crate::grain_extractor::GrainEntry;

/// Inserts a batch of grains into the SQLite database
pub fn insert_grains(db: &String, grains: &Vec<GrainEntry>) -> Result<(), rusqlite::Error> {
    let mut conn = match Connection::open(&db) {
        Ok(x) => x,
        Err(err) => return Err(err)
    };

    let tx = match conn.transaction() {
        Ok(x) => x,
        Err(err) => return Err(err)
    };

    for i in 0..grains.len() {
        match tx.execute(
            "INSERT INTO grains (
                file,
                start_frame,
                end_frame,
                sample_rate,
                grain_duration,
                spectral_centroid,
                spectral_entropy,
                spectral_flatness,
                spectral_kurtosis,
                spectral_roll_off_50,
                spectral_roll_off_75,
                spectral_roll_off_90,
                spectral_roll_off_95,
                spectral_skewness,
                spectral_slope,
                spectral_slope_0_1_khz,
                spectral_slope_1_5_khz,
                spectral_slope_0_5_khz,
                spectral_variance
            ) 
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)", 
            params![
                &grains[i].file.clone(),
                &grains[i].start_frame,
                &grains[i].end_frame,
                &grains[i].sample_rate,
                &grains[i].grain_duration,
                &grains[i].spectral_centroid,
                &grains[i].spectral_entropy,
                &grains[i].spectral_flatness,
                &grains[i].spectral_kurtosis,
                &grains[i].spectral_roll_off_50,
                &grains[i].spectral_roll_off_75,
                &grains[i].spectral_roll_off_90,
                &grains[i].spectral_roll_off_95,
                &grains[i].spectral_skewness,
                &grains[i].spectral_slope,
                &grains[i].spectral_slope_0_1_khz,
                &grains[i].spectral_slope_1_5_khz,
                &grains[i].spectral_slope_0_5_khz,
                &grains[i].spectral_variance
            ],) {
            Ok(_) => (),
            Err(err) => return Err(err)
        }
    }

    match tx.commit() {
        Ok(_) => (),
        Err(err) => return Err(err)
    };

    match conn.close() {
        Ok(_) => (),
        Err((_, err)) => return Err(err)
    }
    Ok(())
}

/// Creates the SQLite database schema
pub fn create_schema(db: &String) -> Result<(), rusqlite::Error> {
    let mut conn = match Connection::open(&db) {
        Ok(x) => x,
        Err(err) => return Err(err)
    };

    match conn.execute("
        CREATE TABLE grains (
            id INTEGER PRIMARY KEY,
            path TEXT NOT NULL,
            start_frame INTEGER NOT NULL,
            end_frame INTEGER NOT NULL,
            sample_rate INTEGER NOT NULL,
            grain_duration REAL NOT NULL,
            spectral_centroid REAL NOT NULL,
            spectral_entropy REAL NOT NULL,
            spectral_flatness REAL NOT NULL,
            spectral_kurtosis REAL NOT NULL,
            spectral_roll_off_50 REAL NOT NULL,
            spectral_roll_off_75 REAL NOT NULL,
            spectral_roll_off_90 REAL NOT NULL,
            spectral_roll_off_95 REAL NOT NULL,
            spectral_skewness REAL NOT NULL,
            spectral_slope REAL NOT NULL,
            spectral_slope_0_1_khz REAL NOT NULL,
            spectral_slope_1_5_khz REAL NOT NULL,
            spectral_slope_0_5_khz REAL NOT NULL,
            spectral_variance REAL NOT NULL
        );
    ", ()) {
        Ok(_) => (),
        Err(err) => return Err(err)
    }

    match conn.close() {
        Ok(_) => (),
        Err((_, err)) => return Err(err)
    }

    Ok(())
}
