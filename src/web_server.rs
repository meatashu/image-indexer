use actix_files::NamedFile;
use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use serde::{Deserialize, Serialize};
use std::path::{PathBuf, Path};
use std::sync::Arc;
use crate::config::AppConfig;
use crate::error::AppError;
use crate::search::Searcher;

async fn read_file_bytes(path: &Path) -> std::io::Result<Vec<u8>> {
    tokio::fs::read(path).await
}

#[derive(Deserialize)]
pub struct DeleteDuplicatesRequest {
    mode: String, // "all" or "keep-one"
}

#[derive(Serialize)]
struct IndexingStatus {
    total_images: u64,
}

async fn get_status(
    searcher_data: web::Data<Arc<dyn Searcher>>,
) -> Result<HttpResponse, AppError> {
    log::debug!("Received request for indexing status.");
    let count = searcher_data.count_images().await?;
    let status = IndexingStatus {
        total_images: count,
    };
    Ok(HttpResponse::Ok().json(status))
}

async fn delete_duplicates(
    path: web::Path<String>,
    searcher_data: web::Data<Arc<dyn Searcher>>,
    payload: web::Json<DeleteDuplicatesRequest>,
) -> Result<HttpResponse, AppError> {
    let hash = path.into_inner();
    log::info!("Received request to delete duplicates for hash: {} with mode: {}", &hash, &payload.mode);

    // 1. Find the document
    let results = searcher_data.search_images(format!("\"{}\"", &hash)).await?;
    let mut metadata = if let Some(meta) = results.into_iter().next() {
        meta
    } else {
        return Err(AppError::NotFound(format!("Image with hash {} not found", &hash)));
    };

    // 2. Determine which files to delete
    let mut files_to_delete: Vec<String> = Vec::new();
    if payload.mode == "all" {
        files_to_delete.push(metadata.file_path.clone());
        files_to_delete.extend(metadata.duplicate_paths.clone());
    } else if payload.mode == "keep-one" {
        files_to_delete.extend(metadata.duplicate_paths.clone());
    } else {
        return Ok(HttpResponse::BadRequest().body("Invalid mode. Use 'all' or 'keep-one'."));
    }

    // 3. Delete the files
    for file_path in &files_to_delete {
        match std::fs::remove_file(file_path) {
            Ok(_) => log::debug!("Deleted file: {}", file_path),
            Err(e) => log::error!("Failed to delete file {}: {}", file_path, e), // Log error but continue
        }
    }

    // 4. Update the index
    if payload.mode == "all" {
        searcher_data.delete_document(&hash).await?;
        log::info!("Deleted document from index for hash: {}", &hash);
    } else if payload.mode == "keep-one" {
        metadata.duplicate_paths.clear();
        searcher_data.update_document(metadata).await?;
        log::info!("Updated document in index for hash: {}", &hash);
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({ "status": "success", "deleted_files": files_to_delete })))
}

#[derive(Serialize, Debug)]
struct WebImage {
    file_path: String,
    file_hash: String,
    width: u32,
    height: u32,
    camera_make: Option<String>,
    camera_model: Option<String>,
    date_taken: Option<String>,
    gps_latitude: Option<f64>,
    gps_longitude: Option<f64>,
    thumbnail_path: String,
    duplicate_paths: Vec<String>,
}

async fn index() -> Result<NamedFile, AppError> {
    NamedFile::open_async("./static/index.html").await.map_err(|e| {
        log::error!("Error serving index.html: {}", e);
        AppError::Io(e)
    })
}

async fn get_images(
    searcher_data: web::Data<Arc<dyn Searcher>>,
    query: web::Query<std::collections::HashMap<String, String>>, // For future filtering
) -> Result<HttpResponse, AppError> {
    log::debug!("Received request for images with query: {:?}", query);
    
    let search_query = query.get("q").cloned().unwrap_or_default();
    let metadata_results = searcher_data.search_images(search_query).await?;

    let web_images: Vec<WebImage> = metadata_results
        .into_iter()
        .map(|m| WebImage {
            file_path: m.file_path,
            file_hash: m.file_hash,
            width: m.width,
            height: m.height,
            camera_make: m.camera_make,
            camera_model: m.camera_model,
            date_taken: m.date_taken,
            gps_latitude: m.gps_latitude,
            gps_longitude: m.gps_longitude,
            thumbnail_path: m.thumbnail_path,
            duplicate_paths: m.duplicate_paths,
        })
        .collect();

    Ok(HttpResponse::Ok().json(web_images))
}

async fn get_thumbnail(
    path: web::Path<String>,
    app_config: web::Data<AppConfig>,
) -> Result<NamedFile, AppError> {
    let hash = path.into_inner();
    log::debug!("Received request for thumbnail with hash: {}", hash);
    
    let thumbnail_path: PathBuf = Path::new(&app_config.thumbnail_directory)
        .join(format!("{}.jpg", hash));
    
    log::trace!("Attempting to serve thumbnail from: {:?}", thumbnail_path);
    Ok(NamedFile::open_async(&thumbnail_path).await?)
}

async fn get_full_image(
    path: web::Path<String>,
    searcher_data: web::Data<Arc<dyn Searcher>>,
) -> Result<HttpResponse, AppError> {
    let hash = path.into_inner();
    log::debug!("Received request for full image with hash: {}", hash);

    let results = searcher_data.search_images(format!("\"{}\"", hash)).await?;

    if let Some(metadata) = results.into_iter().next() {
        let file_path = PathBuf::from(metadata.file_path);
        log::trace!("Attempting to read full image from: {:?}", file_path);

        match read_file_bytes(&file_path).await {
            Ok(bytes) => {
                let mime_type = mime_guess::from_path(&file_path).first_or(mime::APPLICATION_OCTET_STREAM);
                
                Ok(HttpResponse::Ok()
                    .content_type(mime_type.as_ref())
                    .body(bytes))
            }
            Err(e) => {
                log::error!("Failed to read file for full image at {:?}: {}", file_path, e);
                Err(AppError::Io(e))
            }
        }
    } else {
        Err(AppError::NotFound(format!("Image with hash {} not found", hash)))
    }
}


pub async fn start_web_server(
    config: Arc<AppConfig>,
    searcher: Arc<dyn Searcher>,
) -> std::io::Result<()> {
    let port = config.web_port;
    let config_data = web::Data::from(config);
    let searcher_data = web::Data::new(searcher.clone());

    log::info!("Starting web server on port: {}", port);
    log::debug!("Serving static files from ./static directory.");

    HttpServer::new(move || {
        App::new()
            .app_data(config_data.clone())
            .app_data(searcher_data.clone())
            .service(actix_files::Files::new("/static", "./static").show_files_listing())
            .service(web::resource("/api/images").to(get_images))
            .service(web::resource("/api/status").to(get_status))
            .service(web::resource("/api/thumbnails/{hash}").to(get_thumbnail))
            .service(web::resource("/api/images/{hash}").to(get_full_image))
            .service(
                web::resource("/api/images/{hash}/duplicates")
                    .route(web::delete().to(delete_duplicates)),
            )
            .default_service(web::to(index)) // Serve index.html for any unmatched route
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}