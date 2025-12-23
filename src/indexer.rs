use crate::error::AppError;
use crate::metadata::ImageMetadata;
use crate::search::Searcher;

pub fn start_indexing(
    searcher: std::sync::Arc<dyn Searcher>,
    metadata_rx: crossbeam_channel::Receiver<ImageMetadata>,
) -> Result<(), AppError> {
    log::info!("Starting metadata indexing");

    // Ensure the index exists and has the correct mapping
    futures::executor::block_on(searcher.ensure_index_exists())?;

    for metadata in metadata_rx {
        futures::executor::block_on(searcher.index_metadata(metadata))?;
    }

    Ok(())
}
