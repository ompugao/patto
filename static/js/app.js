// Global state
let currentFile = null;
let files = [];
let fileMetadata = {};

// Establish WebSocket connection
const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
const ws = new WebSocket(`${protocol}//${window.location.host}/ws`);

// Initialize the app
document.addEventListener('DOMContentLoaded', () => {
    // Check if we have a path in the URL
    const path = getPathFromUrl();
    if (path) {
        currentFile = path;
        document.getElementById('current-path').textContent = path;
    }

    // Restore saved sorting preference
    const savedSort = localStorage.getItem('patto-sort-order') || 'modified';
    document.getElementById('sort-select').value = savedSort;

    // Add sort dropdown change handler
    document.getElementById('sort-select').addEventListener('change', (e) => {
        const sortValue = e.target.value;
        localStorage.setItem('patto-sort-order', sortValue);
        sortAndUpdateFileList(sortValue);
    });

    // Setup sidebar toggle
    setupSidebarToggle();
});

ws.onopen = () => {
    console.log('WebSocket connection established');
};

ws.onmessage = (event) => {
    const data = JSON.parse(event.data);

    if (data.type === 'FileList') {
        updateFileList(data.data.files, data.data.metadata);

        // If we have a path from the URL, select that file
        if (currentFile) {
            selectFileFromPath(currentFile);
        }
    } else if (data.type === 'FileChanged') {
        handleFileChanged(data.data.path, data.data.html);
    } else if (data.type === 'FileAdded') {
        handleFileAdded(data.data.path, data.data.metadata);
    } else if (data.type === 'FileRemoved') {
        handleFileRemoved(data.data.path);
    }
};

ws.onerror = (error) => {
    console.error('WebSocket error:', error);
};

ws.onclose = () => {
    console.log('WebSocket connection closed');
};

function updateFileList(newFiles, metadata = {}) {
    files = newFiles;
    fileMetadata = metadata;
    
    // Sort files based on current sort option (restore from localStorage if available)
    const sortOption = document.getElementById('sort-select').value || localStorage.getItem('patto-sort-order') || 'title';
    sortAndUpdateFileList(sortOption);
}

function sortAndUpdateFileList(sortBy) {
    let sortedFiles = [...files];
    
    switch(sortBy) {
        case 'title':
            sortedFiles.sort();
            break;
        case 'modified':
            sortedFiles.sort((a, b) => {
                const aTime = fileMetadata[a]?.modified || 0;
                const bTime = fileMetadata[b]?.modified || 0;
                return bTime - aTime; // Most recent first
            });
            break;
        case 'created':
            sortedFiles.sort((a, b) => {
                const aTime = fileMetadata[a]?.created || 0;
                const bTime = fileMetadata[b]?.created || 0;
                return bTime - aTime; // Most recent first
            });
            break;
        case 'linked':
            sortedFiles.sort((a, b) => {
                const aCount = fileMetadata[a]?.linkCount || 0;
                const bCount = fileMetadata[b]?.linkCount || 0;
                return bCount - aCount; // Most linked first
            });
            break;
    }

    const fileList = document.getElementById('file-list');
    fileList.innerHTML = '';

    sortedFiles.forEach(file => {
        const li = document.createElement('li');
        li.textContent = file;
        li.onclick = () => selectFile(file);

        if (file === currentFile) {
            li.classList.add('active');
        }

        fileList.appendChild(li);
    });
}

function handleFileAdded(filePath, metadata) {
    console.log('File added:', filePath);
    
    // Add to files array if not already present
    if (!files.includes(filePath)) {
        files.push(filePath);
        fileMetadata[filePath] = metadata;
        
        // Re-sort and update the display
        const sortOption = document.getElementById('sort-select').value;
        sortAndUpdateFileList(sortOption);
    }
}

function handleFileRemoved(filePath) {
    console.log('File removed:', filePath);
    
    // Remove from files array and metadata
    const index = files.indexOf(filePath);
    if (index > -1) {
        files.splice(index, 1);
        delete fileMetadata[filePath];
        
        // If the removed file was currently selected, clear the preview
        if (currentFile === filePath) {
            currentFile = null;
            document.getElementById('current-path').textContent = '';
            document.getElementById('preview-content').innerHTML = '<div class="empty-state">Select a file to preview</div>';
        }
        
        // Re-sort and update the display
        const sortOption = document.getElementById('sort-select').value;
        sortAndUpdateFileList(sortOption);
    }
}

function selectFile(file) {
    currentFile = file;

    // Update URL without reloading the page
    const newUrl = `/notes/${file}`;
    history.pushState({ path: file }, '', newUrl);
    document.getElementById('current-path').textContent = file;

    // Update UI to show active file
    document.querySelectorAll('#file-list li').forEach(li => {
        li.classList.remove('active');
        if (li.textContent === file) {
            li.classList.add('active');
        }
    });

    // Send file selection to server
    ws.send(JSON.stringify({
        type: 'SelectFile',
        data: {
            path: file
        }
    }));
}

function handleFileChanged(path, html) {
    if (path === currentFile || (currentFile === null && files.length === 0)) {
        currentFile = path;
        updatePreview(html);

        // Update UI to show active file
        document.querySelectorAll('#file-list li').forEach(li => {
            li.classList.remove('active');
            if (li.textContent === path) {
                li.classList.add('active');
            }
        });

        // Update URL and path display if needed
        if (getPathFromUrl() !== path) {
            const newUrl = `/notes/${path}`;
            history.pushState({ path }, '', newUrl);
            document.getElementById('current-path').textContent = path;
        }
    }
}

function updatePreview(html) {
    const preview = document.getElementById('preview-content');
    Idiomorph.morph(preview, "<div id='preview-content'>" + html + "</div>");
    
    // Load Twitter embeds asynchronously after DOM update
    loadTwitterEmbeds();
    renderMermaids();
}

async function renderMermaids() {
    const nodes = document.querySelectorAll('.mermaid');
    await mermaid.run({ nodes: nodes, suppressErrors: false });
}

async function loadTwitterEmbeds() {
    const placeholders = document.querySelectorAll('.twitter-placeholder');
    
    for (const placeholder of placeholders) {
        const url = placeholder.dataset.url;
        if (!url) continue;
        
        try {
            // Fetch Twitter embed through our proxy
            const response = await fetch(`/api/twitter-embed?url=${encodeURIComponent(url)}`);
            if (response.ok) {
                const data = await response.json();
                if (data.html) {
                    placeholder.innerHTML = data.html;
                    placeholder.classList.remove('twitter-placeholder');
                }
            }
        } catch (error) {
            console.warn('Failed to load Twitter embed:', error);
            // Keep the fallback link if embed fails
        }
    }
}

function getPathFromUrl() {
    const path = window.location.pathname;
    if (path.startsWith('/notes/')) {
        return decodeURIComponent(path.substring(7)); // Remove '/notes/' prefix
    }
    return null;
}

function selectFileFromPath(path) {
    // Check if the file exists in our list
    if (files.includes(path)) {
        selectFile(path);
    } else {
        console.warn(`File not found: ${path}`);
        // Optionally, redirect to home or show an error
    }
}

// Setup sidebar toggle functionality
function setupSidebarToggle() {
    const sidebar = document.getElementById('sidebar');
    const toggle = document.getElementById('sidebar-toggle');
    
    // Restore sidebar state from localStorage
    const isCollapsed = localStorage.getItem('sidebar-collapsed') === 'true';
    if (isCollapsed) {
        sidebar.classList.add('collapsed');
        toggle.classList.add('collapsed');
    }
    
    // Add click handler
    toggle.addEventListener('click', () => {
        const isCurrentlyCollapsed = sidebar.classList.contains('collapsed');
        
        if (isCurrentlyCollapsed) {
            sidebar.classList.remove('collapsed');
            toggle.classList.remove('collapsed');
            localStorage.setItem('sidebar-collapsed', 'false');
        } else {
            sidebar.classList.add('collapsed');
            toggle.classList.add('collapsed');
            localStorage.setItem('sidebar-collapsed', 'true');
        }
    });
}

// Handle browser back/forward navigation
window.addEventListener('popstate', (event) => {
    if (event.state && event.state.path) {
        selectFile(event.state.path);
    } else {
        // Handle navigation to root
        currentFile = null;
        document.getElementById('current-path').textContent = '';
        document.getElementById('preview').innerHTML = '<div class="empty-state">Select a file to preview</div>';
        document.querySelectorAll('#file-list li').forEach(li => {
            li.classList.remove('active');
        });
    }
});