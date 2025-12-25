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

3.  **Run the server**:
    ```bash
    ./target/release/image_indexer
    ```
    This will start the web server. By default, it will not start indexing any directories.

4.  **Start Indexing**:
    To start indexing, you need to send a POST request to the `/api/indexer/start` endpoint.

## Command-line Arguments

You can override settings from the configuration files by providing command-line arguments when starting the server.

*   `-i, --tantivy-index-path <PATH>`: The path to store the Tantivy index. If the directory is empty, a new index will be created. If it contains a valid index, it will be reused.
*   `--engine <ENGINE>`: The search engine to use (`tantivy` or `elasticsearch`).
*   `--elasticsearch-url <URL>`: The URL of your Elasticsearch instance.
*   `--thumbnail-directory <PATH>`: The directory to store thumbnails.
*   `--allowed-extensions <EXT1> <EXT2> ...`: A list of file extensions to scan.
*   `-p, --web-port <PORT>`: The port for the web server.
*   `--log-level <LEVEL>`: The log level (`trace`, `debug`, `info`, `warn`, `error`).

## API Endpoints

### Start Indexing

- **POST** `/api/indexer/start`

  Starts a new indexing job in the background.

  **Request Body**:
  ```json
  {
    "scan_directory": "/path/to/your/nas/photos",
    "num_workers": 8
  }
  ```

  - `scan_directory` (required): The absolute path to the directory you want to scan.
  - `num_workers` (optional): The number of parallel threads to use for processing images. If not provided, the value from the config file is used.

  **Response**:
  ```json
  {
    "status": "indexing_started"
  }
  ```

## Packaging and Distribution

To package the application for distribution, you need to bundle the release binary with the necessary configuration and static files.

1.  **Build for release**:
    ```bash
    cargo build --release
    ```

2.  **Create a distribution directory**:
    ```bash
    mkdir image-indexer-dist
    ```

3.  **Copy the files**:
    ```bash
    cp ./target/release/image_indexer ./image-indexer-dist/
    cp -r ./config ./image-indexer-dist/
    cp -r ./static ./image-indexer-dist/
    ```

4.  **Archive the directory**:
    ```bash
    tar -czvf image-indexer-dist.tar.gz image-indexer-dist
    ```

The resulting `image-indexer-dist.tar.gz` file contains everything needed to run the application on a compatible system. The end-user can then run the binary as described in the "Usage" section, and they can modify the `config/default.toml` file or use command-line arguments to configure the application.
