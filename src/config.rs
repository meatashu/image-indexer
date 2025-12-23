use serde::Deserialize;
use std::collections::HashSet;
use config::{Config, ConfigError, File};

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub scan_directory: String,
    pub engine: String,
    pub elasticsearch_url: String,
    pub tantivy_index_path: String,
    pub thumbnail_directory: String,
    pub allowed_extensions: HashSet<String>,
    pub num_workers: usize,
    pub web_port: u16,
    pub log_level: String,
}

impl AppConfig {
    pub fn new() -> Result<Self, ConfigError> {
        let env = std::env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let s = Config::builder()
            .add_source(File::with_name("config/default"))
            .add_source(File::with_name(&format!("config/{}", env)).required(false))
            .add_source(File::with_name("config/local").required(false))
            .build()?;
        
        s.try_deserialize()
    }
}
