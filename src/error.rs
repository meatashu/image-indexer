use tokio::task::JoinError;
use elasticsearch::Error as ElasticsearchError;
use exif::Error as ExifError;
use serde_json::Error as SerdeJsonError;
use thiserror::Error;
use tantivy::{TantivyError, query::QueryParserError};
use actix_web::{HttpResponse, ResponseError, http::StatusCode};

#[derive(Error, Debug)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Walkdir error: {0}")]
    Walkdir(#[from] walkdir::Error),

    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("EXIF error: {0}")]
    Exif(#[from] ExifError),

    #[error("Elasticsearch error: {0}")]
    Elasticsearch(#[from] ElasticsearchError),

    #[error("Tantivy error: {0}")]
    Tantivy(#[from] TantivyError),

    #[error("Tantivy query parser error: {0}")]
    QueryParser(#[from] QueryParserError),

    #[error("JSON error: {0}")]
    Json(#[from] SerdeJsonError),

    #[error("Tokio join error: {0}")]
    Join(#[from] JoinError),

    #[error("Channel send error")]
    SendError,

    #[error("Channel receive error")]
    RecvError,

    #[error("Generic error: {0}")]
    Generic(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .json(serde_json::json!({
                "error": self.to_string(),
            }))
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            AppError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Walkdir(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Image(_) => StatusCode::BAD_REQUEST, // Or INTERNAL_SERVER_ERROR depending on context
            AppError::Exif(_) => StatusCode::BAD_REQUEST,
            AppError::Elasticsearch(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Tantivy(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::QueryParser(_) => StatusCode::BAD_REQUEST,
            AppError::Json(_) => StatusCode::BAD_REQUEST,
            AppError::Join(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::SendError => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::RecvError => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Generic(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
        }
    }
}

impl<T> From<crossbeam_channel::SendError<T>> for AppError {
    fn from(_: crossbeam_channel::SendError<T>) -> Self {
        AppError::SendError
    }
}

impl From<crossbeam_channel::RecvError> for AppError {
    fn from(_: crossbeam_channel::RecvError) -> Self {
        AppError::RecvError
    }
}
