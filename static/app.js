document.addEventListener('DOMContentLoaded', () => {
    // Image Grid and Lightbox Elements
    const imageGrid = document.getElementById('imageGrid');
    const searchInput = document.getElementById('searchInput');
    const searchButton = document.getElementById('searchButton');
    const imageModal = document.getElementById('imageModal');
    const modalImage = document.getElementById('modalImage');
    const modalCaption = document.getElementById('modal-caption');
    const closeImageModal = document.querySelector('#imageModal .close');
    const prevButton = document.querySelector('.prev');
    const nextButton = document.querySelector('.next');

    // Settings Elements
    const settingsIcon = document.getElementById('settingsIcon');
    const settingsModal = document.getElementById('settingsModal');
    const closeSettingsModal = document.querySelector('#settingsModal .close-settings');
    const totalImagesSpan = document.getElementById('totalImages');

    let allImages = [];
    let currentlyDisplayedImages = [];
    let currentImageIndex = 0;

    // --- Image Fetching and Display ---
    async function fetchImages(query = '') {
        try {
            imageGrid.innerHTML = '<p>Loading images...</p>';
            const response = await fetch(`/api/images`);
            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }
            allImages = await response.json();
            performSearch(query);
        } catch (error) {
            console.error("Error fetching images:", error);
            imageGrid.innerHTML = `<p>Error loading images: ${error.message}</p>`;
        }
    }

    function performSearch(query = '') {
        const lowerCaseQuery = query.toLowerCase();
        const filteredImages = allImages.filter(image =>
            image.file_path.toLowerCase().includes(lowerCaseQuery) ||
            image.file_hash.toLowerCase().includes(lowerCaseQuery) ||
            (image.camera_make && image.camera_make.toLowerCase().includes(lowerCaseQuery)) ||
            (image.camera_model && image.camera_model.toLowerCase().includes(lowerCaseQuery)) ||
            (image.date_taken && image.date_taken.toLowerCase().includes(lowerCaseQuery))
        );
        displayImages(filteredImages);
    }
    
    function displayImages(images) {
        currentlyDisplayedImages = images;
        imageGrid.innerHTML = '';
        if (images.length === 0) {
            imageGrid.innerHTML = '<p>No images found.</p>';
            return;
        }

        images.forEach((image, index) => {
            const imageCard = document.createElement('div');
            imageCard.classList.add('image-card');
            imageCard.dataset.index = index;

            const img = document.createElement('img');
            img.src = `/api/thumbnails/${image.file_hash}`;
            img.alt = image.file_path;
            img.loading = 'lazy';
            img.onerror = () => {
                img.src = 'https://via.placeholder.com/200?text=No+Thumb';
            };
            
            img.addEventListener('click', () => {
                openImageModal(index);
            });

            const info = document.createElement('div');
            info.classList.add('image-info');
            let duplicatesHTML = '';
            if (image.duplicate_paths && image.duplicate_paths.length > 0) {
                const duplicateCount = image.duplicate_paths.length;
                duplicatesHTML = `
                    <div class="duplicates">
                        <p class="duplicate">Duplicates: ${duplicateCount}</p>
                        <button class="delete-btn" data-hash="${image.file_hash}" data-count="${duplicateCount}">Delete</button>
                    </div>
                `;
            }

            info.innerHTML = `
                <h3>${image.file_path.split('/').pop()}</h3>
                <p><strong>Path:</strong> ${image.file_path}</p>
                <p><strong>Hash:</strong> ${image.file_hash.substring(0, 16)}...</p>
                <p><strong>Dimensions:</strong> ${image.width}x${image.height}</p>
                ${image.camera_make ? `<p><strong>Make:</strong> ${image.camera_make}</p>` : ''}
                ${image.camera_model ? `<p><strong>Model:</strong> ${image.camera_model}</p>` : ''}
                ${image.date_taken ? `<p><strong>Date:</strong> ${image.date_taken}</p>` : ''}
                ${image.gps_latitude && image.gps_longitude ? `<p><strong>GPS:</strong> ${image.gps_latitude.toFixed(4)}, ${image.gps_longitude.toFixed(4)}</p>` : ''}
                ${duplicatesHTML}
            `;

            imageCard.appendChild(img);
            imageCard.appendChild(info);
            imageGrid.appendChild(imageCard);
        });

        // Add event listeners to new delete buttons
        document.querySelectorAll('.delete-btn').forEach(button => {
            button.addEventListener('click', handleDeleteClick);
        });
    }

    // --- Delete Duplicates ---
    function handleDeleteClick(event) {
        event.stopPropagation(); // Prevent card click event
        closeDeleteMenus(); // Close any other open menus
        
        const button = event.target;
        const hash = button.dataset.hash;
        const count = parseInt(button.dataset.count);
        
        const menu = document.createElement('div');
        menu.classList.add('delete-menu');
        menu.innerHTML = `
            <button data-hash="${hash}" data-mode="keep-one">Delete ${count} copies (keep 1)</button>
            <button data-hash="${hash}" data-mode="all">Delete all ${count + 1} images</button>
        `;
        
        // Position menu relative to the button
        button.parentElement.appendChild(menu);

        // Add listeners to the new menu buttons
        menu.querySelectorAll('button').forEach(menuButton => {
            menuButton.addEventListener('click', (e) => {
                e.stopPropagation();
                const mode = e.target.dataset.mode;
                const message = mode === 'keep-one' 
                    ? `Are you sure you want to delete ${count} duplicate(s)? One copy will be kept.`
                    : `Are you sure you want to delete all ${count + 1} images (including the original)? This action cannot be undone.`;

                if (confirm(message)) {
                    deleteDuplicates(hash, mode);
                }
                closeDeleteMenus();
            });
        });
    }

    function closeDeleteMenus() {
        document.querySelectorAll('.delete-menu').forEach(menu => menu.remove());
    }

    async function deleteDuplicates(hash, mode) {
        try {
            const response = await fetch(`/api/images/${hash}/duplicates`, {
                method: 'DELETE',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({ mode })
            });

            if (!response.ok) {
                const error = await response.json().catch(() => ({ error: 'Failed to delete duplicates.' }));
                throw new Error(error.error);
            }

            const result = await response.json();
            alert(result.message || 'Successfully deleted files.');
            fetchImages(searchInput.value); // Refresh the view
        } catch (error) {
            console.error('Error deleting duplicates:', error);
            alert(`Error: ${error.message}`);
        }
    }


    // --- Image Lightbox Modal ---
    function openImageModal(index) {
        currentImageIndex = index;
        imageModal.style.display = 'block';
        showImage(currentImageIndex);
    }

    function showImage(index) {
        if (index < 0 || index >= currentlyDisplayedImages.length) {
            return;
        }
        const image = currentlyDisplayedImages[index];
        modalImage.src = `/api/images/${image.file_hash}`;
        modalImage.onerror = () => {
            modalImage.src = 'https://via.placeholder.com/800x600?text=Image+Not+Found';
        };
        modalCaption.innerHTML = `
            <p><strong>Path:</strong> ${image.file_path}</p>
            <p><strong>Dimensions:</strong> ${image.width}x${image.height}</p>
            ${image.camera_make ? `<p><strong>Make:</strong> ${image.camera_make}</p>` : ''}
            ${image.camera_model ? `<p><strong>Model:</strong> ${image.camera_model}</p>` : ''}
        `;
        currentImageIndex = index;
    }

    function closeImageModalFunction() {
        imageModal.style.display = 'none';
    }

    function showNextImage() {
        showImage((currentImageIndex + 1) % currentlyDisplayedImages.length);
    }

    function showPrevImage() {
        showImage((currentImageIndex - 1 + currentlyDisplayedImages.length) % currentlyDisplayedImages.length);
    }
    
    // --- Settings Modal ---
    async function openSettingsModal() {
        settingsModal.style.display = 'block';
        totalImagesSpan.textContent = 'Loading...';
        try {
            const response = await fetch('/api/status');
            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }
            const status = await response.json();
            totalImagesSpan.textContent = status.total_images;
        } catch (error) {
            console.error('Error fetching status:', error);
            totalImagesSpan.textContent = 'Error';
        }
    }

    function closeSettingsModalFunction() {
        settingsModal.style.display = 'none';
    }


    // --- Event Listeners ---
    searchButton.addEventListener('click', () => {
        performSearch(searchInput.value);
    });
    searchInput.addEventListener('keyup', (e) => {
        if (e.key === 'Enter') {
            performSearch(searchInput.value);
        }
    });

    // Image Modal Listeners
    closeImageModal.addEventListener('click', closeImageModalFunction);
    prevButton.addEventListener('click', showPrevImage);
    nextButton.addEventListener('click', showNextImage);

    // Settings Modal Listeners
    settingsIcon.addEventListener('click', openSettingsModal);
    closeSettingsModal.addEventListener('click', closeSettingsModalFunction);

    // General Listeners
    window.addEventListener('click', (e) => {
        if (e.target == imageModal) {
            closeImageModalFunction();
        }
        if (e.target == settingsModal) {
            closeSettingsModalFunction();
        }
        // Check if the click is outside of a delete button and its menu
        if (!e.target.closest('.delete-btn') && !e.target.closest('.delete-menu')) {
            closeDeleteMenus();
        }
    });

    document.addEventListener('keydown', (e) => {
        if (imageModal.style.display === 'block') {
            if (e.key === 'ArrowRight') {
                showNextImage();
            } else if (e.key === 'ArrowLeft') {
                showPrevImage();
            } else if (e.key === 'Escape') {
                closeImageModalFunction();
            }
        } else if (settingsModal.style.display === 'block') {
            if (e.key === 'Escape') {
                closeSettingsModalFunction();
            }
        }
    });

    // --- Initial Load ---
    fetchImages();
});
