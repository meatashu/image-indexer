use crate::error::AppError;
use crate::metadata::ImageMetadata;
use async_trait::async_trait;

#[async_trait]
pub trait Searcher: Send + Sync {
    async fn ensure_index_exists(&self) -> Result<(), AppError>;
    async fn index_metadata(&self, metadata: ImageMetadata) -> Result<(), AppError>;
    async fn search_images(&self, query: String) -> Result<Vec<ImageMetadata>, AppError>;
    async fn count_images(&self) -> Result<u64, AppError>;
    async fn delete_document(&self, hash: &str) -> Result<(), AppError>;
    async fn update_document(&self, metadata: ImageMetadata) -> Result<(), AppError>;
}
