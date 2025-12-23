use crate::config::AppConfig;
use crate::error::AppError;
use std::path::PathBuf;
use walkdir::WalkDir;

pub fn start_walking(
    config: AppConfig,
    paths_tx: crossbeam_channel::Sender<PathBuf>,
) -> Result<(), AppError> {
    log::info!("Starting file discovery in {}", config.scan_directory);
    log::debug!("Configured allowed extensions: {:?}", config.allowed_extensions);

    let allowed_extensions = config.allowed_extensions;

    for entry in WalkDir::new(&config.scan_directory)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let path = entry.path();
            log::trace!("Discovered file: {:?}", path);
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                log::trace!("Checking extension: {} for file: {:?}", ext, path);
                if allowed_extensions.contains(&ext.to_lowercase()) {
                    log::debug!("Sending image file to processor: {:?}", path);
                    paths_tx.send(path.to_path_buf())?;
                } else {
                    log::trace!("Skipping file due to unsupported extension: {:?}", path);
                }
            } else {
                log::trace!("Skipping file with no extension: {:?}", path);
            }
        } else {
            log::trace!("Skipping non-file entry: {:?}", entry.path());
        }
    }

    log::info!("File discovery complete.");
    Ok(())
}
