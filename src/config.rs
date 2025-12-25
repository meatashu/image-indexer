use serde::Deserialize;
use std::collections::HashSet;
use config::{Config, ConfigError, File};
use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct CliConfig {
    #[clap(long, short)]
    pub scan_directory: Option<String>,
    #[clap(long, short)]
    pub engine: Option<String>,
    #[clap(long)]
    pub elasticsearch_url: Option<String>,
    #[clap(long, short)]
    pub tantivy_index_path: Option<String>,
    #[clap(long, short)]
    pub thumbnail_directory: Option<String>,
    #[clap(long)]
    pub allowed_extensions: Option<Vec<String>>,
    #[clap(long, short)]
    pub num_workers: Option<usize>,
    #[clap(long, short)]
    pub web_port: Option<u16>,
    #[clap(long)]
    pub log_level: Option<String>,
}

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
        
        let mut config: AppConfig = s.try_deserialize()?;

        let cli_config = CliConfig::parse();

        if let Some(scan_directory) = cli_config.scan_directory {
            config.scan_directory = scan_directory;
        }
        if let Some(engine) = cli_config.engine {
            config.engine = engine;
        }
        if let Some(elasticsearch_url) = cli_config.elasticsearch_url {
            config.elasticsearch_url = elasticsearch_url;
        }
        if let Some(tantivy_index_path) = cli_config.tantivy_index_path {
            config.tantivy_index_path = tantivy_index_path;
        }
        if let Some(thumbnail_directory) = cli_config.thumbnail_directory {
            config.thumbnail_directory = thumbnail_directory;
        }
        if let Some(allowed_extensions) = cli_config.allowed_extensions {
            config.allowed_extensions = allowed_extensions.into_iter().collect();
        }
        if let Some(num_workers) = cli_config.num_workers {
            config.num_workers = num_workers;
        }
        if let Some(web_port) = cli_config.web_port {
            config.web_port = web_port;
        }
        if let Some(log_level) = cli_config.log_level {
            config.log_level = log_level;
        }

        Ok(config)
    }
}
