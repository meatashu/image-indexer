use actix_files::NamedFile;
use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use serde::Serialize;
use std::path::{PathBuf, Path};
use std::sync::Arc;
use crate::error::AppError;
use crate::search::Searcher;
use crate::config::AppConfig;

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
            .service(web::resource("/api/thumbnails/{hash}").to(get_thumbnail))
            .default_service(web::to(index)) // Serve index.html for any unmatched route
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}