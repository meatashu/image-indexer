document.addEventListener('DOMContentLoaded', () => {
    const imageGrid = document.getElementById('imageGrid');
    const searchInput = document.getElementById('searchInput');
    const searchButton = document.getElementById('searchButton');

    let allImages = []; // Store all fetched images for client-side filtering

    // Function to fetch images from the API
    async function fetchImages(query = '') {
        try {
            imageGrid.innerHTML = '<p>Loading images...</p>';
            const response = await fetch(`/api/images?q=${encodeURIComponent(query)}`);
            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }
            const images = await response.json();
            allImages = images; // Store all images
            displayImages(images);
        } catch (error) {
            console.error("Error fetching images:", error);
            imageGrid.innerHTML = `<p>Error loading images: ${error.message}</p>`;
        }
    }

    // Function to display images in the grid
    function displayImages(images) {
        imageGrid.innerHTML = ''; // Clear previous content
        if (images.length === 0) {
            imageGrid.innerHTML = '<p>No images found.</p>';
            return;
        }

        images.forEach(image => {
            const imageCard = document.createElement('div');
            imageCard.classList.add('image-card');

            const img = document.createElement('img');
            // Assuming thumbnails are served from /api/thumbnails/{hash}
            img.src = `/api/thumbnails/${image.file_hash}`;
            img.alt = image.file_path;
            img.loading = 'lazy'; // Lazy load images
            img.onerror = () => {
                img.src = 'https://via.placeholder.com/150?text=No+Thumbnail'; // Placeholder for broken images
            };

            const info = document.createElement('div');
            info.classList.add('image-info');
            info.innerHTML = `
                <h3>${image.file_path.split('/').pop()}</h3>
                <p>Hash: ${image.file_hash}</p>
                ${image.camera_make ? `<p>Make: ${image.camera_make}</p>` : ''}
                ${image.camera_model ? `<p>Model: ${image.camera_model}</p>` : ''}
                ${image.date_taken ? `<p>Date: ${image.date_taken}</p>` : ''}
                ${image.duplicate_paths && image.duplicate_paths.length > 0 ? `<p class="duplicate">Duplicates: ${image.duplicate_paths.length}</p>` : ''}
            `;

            imageCard.appendChild(img);
            imageCard.appendChild(info);
            imageGrid.appendChild(imageCard);
        });
    }

    // Event listener for search button
    searchButton.addEventListener('click', () => {
        const query = searchInput.value.toLowerCase();
        const filteredImages = allImages.filter(image => 
            image.file_path.toLowerCase().includes(query) ||
            image.file_hash.toLowerCase().includes(query) ||
            (image.camera_make && image.camera_make.toLowerCase().includes(query)) ||
            (image.camera_model && image.camera_model.toLowerCase().includes(query))
            // Add more fields to search as needed
        );
        displayImages(filteredImages);
    });

    // Initial load of images
    fetchImages();
});
