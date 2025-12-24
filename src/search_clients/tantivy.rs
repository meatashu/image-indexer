use crate::config::AppConfig;
use crate::error::AppError;
use crate::metadata::ImageMetadata;
use crate::search::Searcher;
use async_trait::async_trait;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Document, Schema, Term, STORED, TEXT, STRING};
use tantivy::{Index, IndexWriter};

pub struct TantivySearcher {
    index: Index,
    schema: Schema,
}

impl TantivySearcher {
    pub fn new(config: &AppConfig) -> Result<Self, AppError> {
        let index_path = &config.tantivy_index_path;
        log::debug!("Initializing Tantivy searcher with index path: {}", index_path);

        let mut schema_builder = Schema::builder();

        schema_builder.add_text_field("file_path", TEXT | STORED);
        schema_builder.add_text_field("file_hash", STRING | STORED);
        schema_builder.add_u64_field("width", STORED);
        schema_builder.add_u64_field("height", STORED);
        schema_builder.add_text_field("camera_make", TEXT | STORED);
        schema_builder.add_text_field("camera_model", TEXT | STORED);
        schema_builder.add_text_field("date_taken", TEXT | STORED);
        schema_builder.add_text_field("thumbnail_path", TEXT | STORED);
        schema_builder.add_text_field("duplicate_paths", TEXT | STORED);

        let schema = schema_builder.build();
        let index = Index::open_in_dir(index_path).unwrap_or_else(|_| {
            log::info!("Tantivy index not found at {}. Creating new index.", index_path);
            std::fs::create_dir_all(index_path).unwrap();
            Index::create_in_dir(index_path, schema.clone()).unwrap()
        });
        log::debug!("Tantivy searcher initialized successfully.");
        Ok(Self { index, schema })
    }
}

#[async_trait]
impl Searcher for TantivySearcher {
    async fn ensure_index_exists(&self) -> Result<(), AppError> {
        // Tantivy creates the index on new, so this is a no-op
        log::debug!("Tantivy index existence is handled during initialization.");
        Ok(())
    }

    async fn index_metadata(&self, metadata: ImageMetadata) -> Result<(), AppError> {
        let index = self.index.clone();
        let schema = self.schema.clone();

        tokio::task::spawn_blocking(move || {
            log::debug!("Attempting to index metadata for file: {}", metadata.file_path);
            let mut index_writer: IndexWriter = index.writer(50_000_000)?;

            let file_path_field = schema.get_field("file_path").unwrap();
            let file_hash_field = schema.get_field("file_hash").unwrap();
            let width_field = schema.get_field("width").unwrap();
            let height_field = schema.get_field("height").unwrap();
            let camera_make_field = schema.get_field("camera_make").unwrap();
            let camera_model_field = schema.get_field("camera_model").unwrap();
            let date_taken_field = schema.get_field("date_taken").unwrap();
            let thumbnail_path_field = schema.get_field("thumbnail_path").unwrap();
            let duplicate_paths_field = schema.get_field("duplicate_paths").unwrap();

            let searcher = index.reader()?.searcher();
            let query_parser = QueryParser::for_index(&index, vec![file_hash_field]);
            let query = query_parser.parse_query(&metadata.file_hash)?;
            log::trace!("Searching for existing document with hash: {}", metadata.file_hash);
            let top_docs = searcher.search(&query, &TopDocs::with_limit(1))?;

            if top_docs.is_empty() {
                log::trace!("No existing document found for hash: {}. Indexing new document.", metadata.file_hash);
                let mut doc = Document::default();
                doc.add_text(file_path_field, &metadata.file_path);
                doc.add_text(file_hash_field, &metadata.file_hash);
                doc.add_u64(width_field, metadata.width as u64);
                doc.add_u64(height_field, metadata.height as u64);
                if let Some(make) = &metadata.camera_make {
                    doc.add_text(camera_make_field, make);
                }
                if let Some(model) = &metadata.camera_model {
                    doc.add_text(camera_model_field, model);
                }
                if let Some(date) = &metadata.date_taken {
                    doc.add_text(date_taken_field, date);
                }
                doc.add_text(thumbnail_path_field, &metadata.thumbnail_path);
                doc.add_text(
                    duplicate_paths_field,
                    &metadata.duplicate_paths.join(","),
                );
                index_writer.add_document(doc)?;
                log::debug!("New document indexed for file: {}", metadata.file_path);
            } else {
                let (score, doc_address) = top_docs[0];
                log::trace!("Duplicate image found for hash: {}. Score: {}. Doc Address: {:?}", metadata.file_hash, score, doc_address);
                let existing_doc = searcher.doc(doc_address)?;
                let mut new_doc = Document::default();

                for field_value in existing_doc.field_values() {
                    if field_value.field() == duplicate_paths_field {
                        let mut paths = field_value.value().as_text().unwrap_or("").to_string();
                        // Only add if the path is not already present
                        if !paths.split(',').any(|s| s == metadata.file_path) {
                             if !paths.is_empty() {
                                paths.push(',');
                            }
                            paths.push_str(&metadata.file_path);
                        }
                        new_doc.add_text(duplicate_paths_field, &paths);
                    } else {
                        new_doc.add_field_value(field_value.field(), field_value.value().clone());
                    }
                }
                index_writer.delete_term(Term::from_field_text(file_hash_field, &metadata.file_hash));
                index_writer.add_document(new_doc)?;
                log::debug!("Existing document updated for file: {}", metadata.file_path);
            }

            index_writer.commit()?;
            log::trace!("Tantivy index writer committed changes.");
            Ok(())
        })
        .await?
    }

    async fn search_images(&self, query: String) -> Result<Vec<ImageMetadata>, AppError> {
        let index = self.index.clone();
        let schema = self.schema.clone();

        tokio::task::spawn_blocking(move || {
            log::debug!("Searching Tantivy for images with query: {}", query);
            let searcher = index.reader()?.searcher();
            let mut images = Vec::new();

            let file_path_field = schema.get_field("file_path").unwrap();
            let file_hash_field = schema.get_field("file_hash").unwrap();
            let camera_make_field = schema.get_field("camera_make").unwrap();
            let camera_model_field = schema.get_field("camera_model").unwrap();
            let date_taken_field = schema.get_field("date_taken").unwrap();
            let thumbnail_path_field = schema.get_field("thumbnail_path").unwrap();
            let duplicate_paths_field = schema.get_field("duplicate_paths").unwrap();
            let width_field = schema.get_field("width").unwrap();
            let height_field = schema.get_field("height").unwrap();


            let query_parser = QueryParser::for_index(
                &index,
                vec![
                    file_path_field,
                    file_hash_field,
                    camera_make_field,
                    camera_model_field,
                    date_taken_field,
                ],
            );

            let query_obj = if query.is_empty() {
                query_parser.parse_query("*")? // Match all if query is empty
            } else {
                query_parser.parse_query(&query)?
            };

            let top_docs = searcher.search(&query_obj, &TopDocs::with_limit(100))?; // Limit results for UI

            for (_score, doc_address) in top_docs {
                let retrieved_doc = searcher.doc(doc_address)?;
                let file_path_val = retrieved_doc.get_first(file_path_field).and_then(|v| v.as_text()).unwrap_or("").to_string();
                let file_hash_val = retrieved_doc.get_first(file_hash_field).and_then(|v| v.as_text()).unwrap_or("").to_string();
                let width_val = retrieved_doc.get_first(width_field).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let height_val = retrieved_doc.get_first(height_field).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let camera_make_val = retrieved_doc.get_first(camera_make_field).and_then(|v| v.as_text()).map(|s| s.to_string());
                let camera_model_val = retrieved_doc.get_first(camera_model_field).and_then(|v| v.as_text()).map(|s| s.to_string());
                let date_taken_val = retrieved_doc.get_first(date_taken_field).and_then(|v| v.as_text()).map(|s| s.to_string());
                let thumbnail_path_val = retrieved_doc.get_first(thumbnail_path_field).and_then(|v| v.as_text()).unwrap_or("").to_string();
                let duplicate_paths_val = retrieved_doc.get_first(duplicate_paths_field).and_then(|v| v.as_text()).unwrap_or("").split(',').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect();

                images.push(ImageMetadata {
                    file_path: file_path_val,
                    file_hash: file_hash_val,
                    width: width_val,
                    height: height_val,
                    camera_make: camera_make_val,
                    camera_model: camera_model_val,
                    date_taken: date_taken_val,
                    gps_latitude: None, // Tantivy doesn't have direct geo_point
                    gps_longitude: None, // Tantivy doesn't have direct geo_point
                    thumbnail_path: thumbnail_path_val,
                    duplicate_paths: duplicate_paths_val,
                });
            }
            log::debug!("Found {} images in Tantivy for query: {}", images.len(), query);
            Ok(images)
        }).await?
    }

    async fn count_images(&self) -> Result<u64, AppError> {
        let index = self.index.clone();
        let result = tokio::task::spawn_blocking(move || {
            let reader = index.reader()?;
            let searcher = reader.searcher();
            Ok(searcher.num_docs())
        })
        .await?;
        result
    }

    async fn delete_document(&self, hash: &str) -> Result<(), AppError> {
        let index = self.index.clone();
        let schema = self.schema.clone();
        let hash_for_closure = hash.to_string();

        tokio::task::spawn_blocking(move || {
            let mut index_writer: IndexWriter = index.writer(50_000_000)?;
            let file_hash_field = schema.get_field("file_hash").unwrap();
            let term = Term::from_field_text(file_hash_field, &hash_for_closure);
            index_writer.delete_term(term);
            index_writer.commit()?;
            log::debug!("Deleted document with hash: {}", hash_for_closure);
            Ok::<(), tantivy::error::TantivyError>(())
        })
        .await??;
        Ok(())
    }

    async fn update_document(&self, metadata: ImageMetadata) -> Result<(), AppError> {
        // This is not the most efficient way, but it is simple and reuses existing code.
        // A better implementation would combine delete and add into a single commit.
        self.delete_document(&metadata.file_hash).await?;
        self.index_metadata(metadata).await
    }
}
