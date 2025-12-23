use crate::config::AppConfig;
use crate::error::AppError;
use crate::metadata::ImageMetadata;
use crate::search::Searcher;
use async_trait::async_trait;
use elasticsearch::{
    http::transport::{BuildError, SingleNodeConnectionPool, TransportBuilder},
    SearchParts,
    UpdateParts,
    Elasticsearch, IndexParts,
};
use serde_json::json;
use url::Url;

const INDEX_NAME: &str = "images";

pub struct ElasticsearchSearcher {
    client: Elasticsearch,
}

impl ElasticsearchSearcher {
    pub fn new(config: &AppConfig) -> Result<Self, BuildError> {
        log::debug!("Creating Elasticsearch client for URL: {}", config.elasticsearch_url);
        let url = Url::parse(&config.elasticsearch_url).unwrap();
        let conn_pool = SingleNodeConnectionPool::new(url);
        let transport = TransportBuilder::new(conn_pool).disable_proxy().build()?;
        let client = Elasticsearch::new(transport);
        log::trace!("Elasticsearch client created successfully.");
        Ok(Self { client })
    }
}

#[async_trait]
impl Searcher for ElasticsearchSearcher {
    async fn ensure_index_exists(&self) -> Result<(), AppError> {
        log::debug!("Checking if Elasticsearch index '{}' exists.", INDEX_NAME);
        let index_exists = self
            .client
            .indices()
            .exists(elasticsearch::indices::IndicesExistsParts::Index(&[
                INDEX_NAME,
            ]))
            .send()
            .await?
            .status_code()
            .is_success();

        if !index_exists {
            log::info!("Elasticsearch index '{}' does not exist. Creating it.", INDEX_NAME);
            self.client
                .indices()
                .create(elasticsearch::indices::IndicesCreateParts::Index(
                    INDEX_NAME,
                ))
                .body(json!({
                    "mappings": {
                        "properties": {
                            "file_path": { "type": "keyword" },
                            "file_hash": { "type": "keyword" },
                            "width": { "type": "integer" },
                            "height": { "type": "integer" },
                            "camera_make": { "type": "keyword" },
                            "camera_model": { "type": "keyword" },
                            "date_taken": { "type": "date", "format": "yyyy:MM:dd HH:mm:ss||yyyy-MM-dd HH:mm:ss||epoch_millis" },
                            "gps_latitude": { "type": "geo_point" },
                            "gps_longitude": { "type": "geo_point" },
                            "thumbnail_path": { "type": "keyword" },
                            "duplicate_paths": { "type": "keyword" }
                        }
                    }
                }))
                .send()
                .await?;
            log::info!("Elasticsearch index '{}' created successfully.", INDEX_NAME);
        } else {
            log::debug!("Elasticsearch index '{}' already exists.", INDEX_NAME);
        }

        Ok(())
    }

    async fn index_metadata(&self, metadata: ImageMetadata) -> Result<(), AppError> {
        log::debug!("Attempting to index metadata for file: {}", metadata.file_path);

        let response = self
            .client
            .search(SearchParts::Index(&[INDEX_NAME]))
            .body(json!({
                "query": {
                    "term": {
                        "file_hash": &metadata.file_hash
                    }
                }
            }))
            .send()
            .await?;

        let body = response.json::<serde_json::Value>().await?;
        let hits = body["hits"]["hits"].as_array().unwrap();

        if hits.is_empty() {
            log::trace!("No existing document found for hash: {}. Indexing new document.", metadata.file_hash);
            self.client
                .index(IndexParts::IndexId(INDEX_NAME, &metadata.file_hash))
                .body(metadata.clone())
                .send()
                .await?;
            log::debug!("New document indexed for file: {}", metadata.file_path);
        } else {
            let doc_id = hits[0]["_id"].as_str().unwrap();
            log::trace!("Duplicate image found for hash: {}. Updating existing document ID: {} with new path: {}", metadata.file_hash, doc_id, metadata.file_path);
            self.client
                .update(UpdateParts::IndexId(INDEX_NAME, doc_id))
                .body(json!({
                    "script": {
                        "source": "if (ctx._source.duplicate_paths.indexOf(params.path) == -1) { ctx._source.duplicate_paths.add(params.path) }",
                        "lang": "painless",
                        "params": {
                            "path": metadata.file_path
                        }
                    }
                }))
                .send()
                .await?;
            log::debug!("Existing document updated for file: {}", metadata.file_path);
        }

        Ok(())
    }

    async fn search_images(&self, query: String) -> Result<Vec<ImageMetadata>, AppError> {
        log::debug!("Searching Elasticsearch for images with query: {}", query);
        let search_query = if query.is_empty() {
            json!({
                "query": {
                    "match_all": {}
                },
                "size": 100 // Limit results for UI
            })
        } else {
            json!({
                "query": {
                    "multi_match": {
                        "query": query,
                        "fields": ["file_path", "file_hash", "camera_make", "camera_model", "date_taken", "duplicate_paths"]
                    }
                },
                "size": 100 // Limit results for UI
            })
        };

        let response = self.client
            .search(SearchParts::Index(&[INDEX_NAME]))
            .body(search_query)
            .send()
            .await?;

        let body = response.json::<serde_json::Value>().await?;
        log::trace!("Elasticsearch search response: {:?}", body);

        let mut images = Vec::new();
        if let Some(hits) = body["hits"]["hits"].as_array() {
            for hit in hits {
                if let Some(source) = hit["_source"].as_object() {
                    let metadata: ImageMetadata = serde_json::from_value(serde_json::Value::Object(source.clone()))?;
                    images.push(metadata);
                }
            }
        }
        log::debug!("Found {} images in Elasticsearch for query: {}", images.len(), query);
        Ok(images)
    }
}
