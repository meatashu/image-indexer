// src/metadata.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageMetadata {
    pub file_path: String,
    pub file_hash: String,
    pub width: u32,
    pub height: u32,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub date_taken: Option<String>,
    pub gps_latitude: Option<f64>,
    pub gps_longitude: Option<f64>,
    pub thumbnail_path: String,
    pub duplicate_paths: Vec<String>,
}
