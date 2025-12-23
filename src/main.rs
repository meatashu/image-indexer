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

    let (paths_tx, paths_rx) = crossbeam_channel::unbounded();
    let (metadata_tx, metadata_rx) = crossbeam_channel::unbounded();

    let searcher_clone_for_indexer = searcher.clone();
    let config_for_indexing = config.clone();

    // Run indexing in the background
    let indexing_handle = tokio::spawn(async move {
        let config_for_processor = config_for_indexing.clone();
        let walker_handle = tokio::task::spawn_blocking(move || {
            if let Err(e) = walker::start_walking(config_for_indexing, paths_tx) {
                log::error!("Walker error: {}", e);
            }
        });

        let processor_handle = tokio::task::spawn_blocking(move || {
            if let Err(e) = processor::start_processing(config_for_processor, paths_rx, metadata_tx) {
                log::error!("Processor error: {}", e);
            }
        });

        let indexer_handle = tokio::task::spawn_blocking(move || {
            if let Err(e) = indexer::start_indexing(searcher_clone_for_indexer, metadata_rx) {
                log::error!("Indexer error: {}", e);
            }
        });
        
        // Wait for all indexing tasks to complete
        let (walker_res, processor_res, indexer_res) = tokio::join!(walker_handle, processor_handle, indexer_handle);

        if let Err(e) = walker_res {
            log::error!("Walker task panicked: {}", e);
        }
        if let Err(e) = processor_res {
            log::error!("Processor task panicked: {}", e);
        }
        if let Err(e) = indexer_res {
            log::error!("Indexer task panicked: {}", e);
        }

        log::info!("All indexing tasks have completed.");
    });

    // Run the web server in the foreground
    // The web server runs indefinitely, so this will keep the main function alive.
    if let Err(e) = web_server::start_web_server(Arc::new(config), searcher).await {
        log::error!("Web server error: {}", e);
    }

    // Wait for the indexing to finish gracefully if the web server stops.
    if let Err(e) = indexing_handle.await {
        log::error!("Indexing handle panicked: {}", e);
    }
    
    info!("Image-indexer finished");

    Ok(())
}