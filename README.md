# Image Indexer

## About

Image Indexer is a high-performance, Rust-based command-line application designed to scan large collections of images from a Network Attached Storage (NAS) or any local directory. It extracts valuable metadata, generates thumbnails, and indexes everything into a local Elasticsearch database. This tool helps you organize, search, and deduplicate your photo library with incredible speed and efficiency.

## Features

- **High-Speed Scanning**: Leverages Rust's performance and a multi-threaded architecture to quickly traverse large directory structures.
- **Comprehensive Metadata Extraction**: Gathers key information from your images, including:
  - EXIF data (camera make/model, date taken)
  - Image dimensions
  - GPS coordinates
- **Efficient Deduplication**: Uses SHA-256 hashing to accurately identify and flag duplicate images, saving storage space and keeping your library clean.
- **Fast Search & Retrieval**: Indexes all metadata in Elasticsearch, enabling near-instant search and filtering capabilities.
- **Automatic Thumbnail Generation**: Creates lightweight thumbnails for each image, perfect for powering a fast and responsive photo browser UI.
- **Configurable**: Easily customize settings through a simple TOML configuration file.

## Prerequisites

Before you begin, ensure you have the following installed:

- **Rust**: The application is built with Rust. You can install it from [rust-lang.org](https://www.rust-lang.org/tools/install).
- **Elasticsearch**: An Elasticsearch instance is required to store the image index. You can run it locally via Docker or install it directly.

## Configuration

The application is configured via TOML files in the `config/` directory.

1.  **Default Settings**: The file `config/default.toml` contains the default configuration.
2.  **Local Overrides**: You can create a `config/local.toml` file to override any of the default settings. This file is ignored by git.

### Key Configuration Options

-   `scan_directory`: The absolute path to the directory you want to scan (e.g., `/mnt/nas/photos`).
-   `engine`: The search engine to use.
    -   `"tantivy"` (default): An embedded, file-based search engine. No external services required.
    -   `"elasticsearch"`: Uses an external Elasticsearch cluster.
-   `elasticsearch_url`: The URL of your Elasticsearch instance (only used if `engine` is `"elasticsearch"`).
-   `tantivy_index_path`: The local file system path to store the Tantivy index (only used if `engine` is `"tantivy"`).
-   `thumbnail_directory`: A path where generated thumbnails will be stored.
-   `allowed_extensions`: A list of image file extensions to include in the scan.
-   `num_workers`: The number of parallel threads to use for processing images.

### Example: Using Elasticsearch

To use Elasticsearch instead of the default Tantivy engine, create a `config/local.toml` file with the following content:

```toml
engine = "elasticsearch"
elasticsearch_url = "http://localhost:9200"
```

## Usage

1.  **Clone the repository**:
    ```bash
    git clone <repository_url>
    cd image_indexer
    ```

2.  **Build the application**:
    ```bash
    cargo build --release
    ```

3.  **Run the indexer**:
    ```bash
    ./target/release/image_indexer
    ```

The application will start scanning the directory specified in your configuration, and you will see log output in your terminal as it discovers, processes, and indexes your images.
