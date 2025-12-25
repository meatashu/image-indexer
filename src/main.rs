mod config;
mod error;
mod indexer;
mod metadata;
mod processor;
mod search;
mod search_clients;
mod walker;
mod web_server;

use crate::config::AppConfig;
use crate::search::Searcher;
use crate::search_clients::{elasticsearch::ElasticsearchSearcher, tantivy::TantivySearcher};
use anyhow::Result;
use log::info;
use std::sync::Arc;

pub fn start_indexing_job(config: AppConfig, searcher: Arc<dyn Searcher>) {
    tokio::spawn(async move {
        let result = async {
            searcher.ensure_index_exists().await?;
            let existing_hashes = searcher.get_all_hashes().await?;
            info!("Found {} existing images in the index.", existing_hashes.len());

            let (paths_tx, paths_rx) = crossbeam_channel::unbounded();
            let (metadata_tx, metadata_rx) = crossbeam_channel::unbounded();

            let searcher_clone_for_indexer = searcher.clone();
            let config_for_processor = config.clone();

            // Run indexing in the background
            let walker_handle = tokio::task::spawn_blocking(move || {
                if let Err(e) = walker::start_walking(config, paths_tx) {
                    log::error!("Walker error: {}", e);
                }
            });

            let processor_handle = tokio::task::spawn_blocking(move || {
                if let Err(e) = processor::start_processing(config_for_processor, paths_rx, metadata_tx, existing_hashes) {
                    log::error!("Processor error: {}", e);
                }
            });

            let indexer_handle = tokio::task::spawn_blocking(move || {
                if let Err(e) = indexer::start_indexing(searcher_clone_for_indexer, metadata_rx) {
                    log::error!("Indexer error: {}", e);
                }
            });
            
            // Wait for all indexing tasks to complete
            tokio::try_join!(walker_handle, processor_handle, indexer_handle)?;

            log::info!("All indexing tasks have completed.");

            Ok::<(), anyhow::Error>(())
        }.await;

        if let Err(e) = result {
            log::error!("Indexing failed: {}", e);
        }
    });
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::new()?;
    
    // Initialize env_logger based on config.log_level
    env_logger::Builder::new()
        .filter_level(config.log_level.parse().unwrap_or(log::LevelFilter::Info))
        .init();

    info!("Starting image-indexer");

    let searcher: Arc<dyn Searcher> = if config.engine == "elasticsearch" {
        Arc::new(ElasticsearchSearcher::new(&config)?)
    } else {
        Arc::new(TantivySearcher::new(&config)?)
    };

    // Run the web server in the foreground
    if let Err(e) = web_server::start_web_server(Arc::new(config), searcher).await {
        log::error!("Web server error: {}", e);
    }
    
    info!("Image-indexer finished");

    Ok(())
}