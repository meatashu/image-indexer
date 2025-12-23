use crate::config::AppConfig;
use crate::error::AppError;
use crate::metadata::ImageMetadata;
use exif::Reader;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;

pub fn start_processing(
    config: AppConfig,
    paths_rx: crossbeam_channel::Receiver<PathBuf>,
    metadata_tx: crossbeam_channel::Sender<ImageMetadata>,
) -> Result<(), AppError> {
    log::info!("Starting image processing with {} workers", config.num_workers);
    log::debug!("Processor will use thumbnail directory: {}", config.thumbnail_directory);

    let paths: Vec<PathBuf> = paths_rx.iter().collect();
    log::info!("Received {} paths for processing.", paths.len());

    paths.into_par_iter().try_for_each(|path| {
        log::info!("Processing image started for: {:?}", path); // Log when processing starts for a specific image
        match process_image(&config, &path) {
            Ok(metadata) => {
                log::trace!("Extracted metadata for {:?}: {:?}", path, metadata);
                metadata_tx.send(metadata)?;
                log::info!("Processing image finished for: {:?}", path); // Log when processing finishes
                Ok::<(), AppError>(())
            },
            Err(e) => {
                log::warn!("Failed to process image {:?}: {}", path, e);
                // Continue processing other images, don't propagate the error
                Ok::<(), AppError>(())
            }
        }
    })?;

    log::info!("All images processed.");
    Ok(())
}

fn process_image(config: &AppConfig, path: &PathBuf) -> Result<ImageMetadata, AppError> {
    log::trace!("Calculating hash for image: {:?}", path);
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    let hash = format!("{:x}", hasher.finalize());
    log::debug!("Calculated hash for {:?}: {}", path, hash);

    log::trace!("Extracting EXIF data for image: {:?}", path);
    let file_for_exif = File::open(path)?; // Reopen file for EXIF
    let mut buf_reader = BufReader::new(file_for_exif);
    let exif_reader = Reader::new();
    let exif = exif_reader.read_from_container(&mut buf_reader).ok();

    log::trace!("Getting image dimensions for image: {:?}", path);
    let (width, height) = image::image_dimensions(path).map_err(|e| {
        log::warn!("Could not get dimensions for {:?}: {}", path, e);
        e
    })?;
    log::debug!("Dimensions for {:?}: {}x{}", path, width, height);

    let mut metadata = ImageMetadata {
        file_path: path.to_string_lossy().to_string(),
        file_hash: hash,
        width,
        height,
        camera_make: None,
        camera_model: None,
        date_taken: None,
        gps_latitude: None,
        gps_longitude: None,
        thumbnail_path: "".to_string(),
        duplicate_paths: vec![],
    };

    if let Some(exif) = exif {
        log::trace!("EXIF data found for {:?}", path);
        if let Some(field) = exif.get_field(exif::Tag::Make, exif::In::PRIMARY) {
            metadata.camera_make = Some(field.display_value().to_string());
            log::trace!("Camera make: {}", metadata.camera_make.as_ref().unwrap());
        }
        if let Some(field) = exif.get_field(exif::Tag::Model, exif::In::PRIMARY) {
            metadata.camera_model = Some(field.display_value().to_string());
            log::trace!("Camera model: {}", metadata.camera_model.as_ref().unwrap());
        }
        if let Some(field) =
            exif.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)
        {
            metadata.date_taken = Some(field.display_value().to_string());
            log::trace!("Date taken: {}", metadata.date_taken.as_ref().unwrap());
        }
        if let Some(field) =
            exif.get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY)
        {
            if let exif::Value::Rational(gps) = &field.value {
                metadata.gps_latitude = Some(gps.iter().map(|v| v.to_f64()).sum());
                log::trace!("GPS Latitude: {}", metadata.gps_latitude.unwrap());
            }
        }
        if let Some(field) =
            exif.get_field(exif::Tag::GPSLongitude, exif::In::PRIMARY)
        {
            if let exif::Value::Rational(gps) = &field.value {
                metadata.gps_longitude = Some(gps.iter().map(|v| v.to_f64()).sum());
                log::trace!("GPS Longitude: {}", metadata.gps_longitude.unwrap());
            }
        }
    } else {
        log::debug!("No EXIF data found for {:?}", path);
    }

    log::trace!("Generating thumbnail for image: {:?}", path);
    let thumbnail_dir = std::path::Path::new(&config.thumbnail_directory);
    if !thumbnail_dir.exists() {
        std::fs::create_dir_all(thumbnail_dir)?;
        log::debug!("Created thumbnail directory: {:?}", thumbnail_dir);
    }
    let thumbnail_path = thumbnail_dir
        .join(format!("{}.jpg", metadata.file_hash));
    
    log::trace!("Opening image for thumbnail generation: {:?}", path);
    let image = image::open(path).map_err(|e| {
        log::warn!("Could not open image for thumbnail generation {:?}: {}", path, e);
        e
    })?;
    let thumbnail = image.thumbnail(256, 256);
    thumbnail.save(&thumbnail_path)?;
    metadata.thumbnail_path = thumbnail_path.to_string_lossy().to_string();
    log::debug!("Thumbnail saved to: {:?}", thumbnail_path);

    Ok(metadata)
}
