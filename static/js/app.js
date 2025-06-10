// Global state
let currentFile = null;
let currentAnchor = null
let files = [];
let fileMetadata = {};

// Establish WebSocket connection
const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
const ws = new WebSocket(`${protocol}//${window.location.host}/ws`);

// Initialize the app
document.addEventListener('DOMContentLoaded', () => {
    // Check if we have a path in the URL
    const path = getPathFromUrl();
    const anchor = getAnchorFromUrl();
    if (path) {
        currentFile = path;
        currentAnchor = anchor;
        document.getElementById('current-path').textContent = path;
        console.log('Initial load - Path:', path, 'Anchor:', anchor);
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
    
    // Setup anchor monitoring for dynamic content
    setupAnchorMonitoring();
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
            console.log('Selecting file from URL:', currentFile, ', Anchor: ', currentAnchor);
            selectFileFromPath(currentFile, currentAnchor);
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

function handleFileChanged(path, html) {
    console.log('handleFileChanged called:', path, 'currentFile:', currentFile);
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
        const currentPath = getPathFromUrl();
        if (currentPath !== path) {
            // Preserve existing anchor when updating URL
            const anchor = getAnchorFromUrl();
            const newUrl = anchor ? `/notes/${path}#${anchor}` : `/notes/${path}`;
            history.pushState({ path: path, anchor: anchor }, '', newUrl);
            document.getElementById('current-path').textContent = path;
        }
    }
}

function updatePreview(html) {
    const preview = document.getElementById('preview-content');
    
    // Use a callback to handle anchor scrolling after morphing is complete
    Idiomorph.morph(preview, "<div id='preview-content'>" + html + "</div>", {
        callbacks: {
            afterNodeMorphed: () => {
                // Check for anchor after each node is morphed
                const hash = window.location.hash;
                if (hash) {
                    const anchorId = hash.substring(1);
                    const element = document.getElementById(anchorId);
                    if (element) {
                        requestAnimationFrame(() => {
                            element.scrollIntoView({ behavior: 'smooth', block: 'start' });
                        });
                    }
                }
            }
        }
    });
    
    // Load Twitter embeds asynchronously after DOM update
    loadTwitterEmbeds();
    renderMermaids();
    if (typeof hljs !== 'undefined') {
        hljs.highlightAll();
    }
    
    // Handle anchor scrolling after all content is processed
    setTimeout(() => handleAnchorScroll(), 100);
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
                    Idiomorph.morph(placeholder, data.html, {morphStyle: 'innerHTML'});
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

function getAnchorFromUrl() {
    const hash = window.location.hash;
    return hash ? hash.substring(1) : null;
}

function selectFileFromPath(path, anchor = null) {
    // Check if the file exists in our list
    if (files.includes(path)) {
        selectFile(path, anchor);
    } else {
        console.warn(`File not found: ${path}`);
        // Optionally, redirect to home or show an error
    }
}

// Handle anchor scrolling from URL fragment
function handleAnchorScroll() {
    const hash = window.location.hash;
    console.log('handleAnchorScroll called, hash:', hash);
    if (hash) {
        const anchorId = hash.substring(1);
        console.log('Looking for element with id:', anchorId);
        
        // Wait for content to be fully rendered with retry mechanism
        const tryScroll = (attempts = 0) => {
            const element = document.getElementById(anchorId);
            console.log(`Attempt ${attempts + 1}: Found element:`, element);
            
            if (element) {
                console.log('Scrolling to element:', element);
                element.scrollIntoView({ behavior: 'smooth', block: 'start' });
                // Add visual highlight
                element.style.transition = 'background-color 0.3s ease';
                const originalBg = element.style.backgroundColor;
                element.style.backgroundColor = '#fff3cd';
                setTimeout(() => {
                    element.style.backgroundColor = originalBg;
                }, 2000);
            } else if (attempts < 10) {
                // Retry up to 10 times with increasing delay
                console.log(`No element found with id: ${anchorId}, retrying...`);
                setTimeout(() => tryScroll(attempts + 1), 100 * (attempts + 1));
            } else {
                console.log('No element found with id:', anchorId);
                // Debug: List all available anchors
                const anchors = document.querySelectorAll('.anchor');
                console.log('All anchor elements found:', anchors);
                anchors.forEach(a => console.log('- Anchor id:', a.id, 'text:', a.textContent));
                console.log('Available IDs in document:', Array.from(document.querySelectorAll('[id]')).map(el => el.id));
            }
        };
        
        tryScroll();
    }
}

function selectFile(file, anchor = null) {
    currentFile = file;
    currentAnchor = anchor;

    // Update URL with anchor if provided
    const newUrl = anchor ? `/notes/${file}#${anchor}` : `/notes/${file}`;
    //history.pushState({ path: file, anchor: anchor }, '', newUrl);
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

// Setup anchor monitoring for dynamic content
function setupAnchorMonitoring() {
    // Create a MutationObserver to watch for anchor elements being added
    const observer = new MutationObserver((mutations) => {
        let anchorAdded = false;
        mutations.forEach((mutation) => {
            if (mutation.type === 'childList') {
                mutation.addedNodes.forEach((node) => {
                    if (node.nodeType === Node.ELEMENT_NODE) {
                        // Check if the added node or its children contain anchor elements
                        if (node.classList && node.classList.contains('anchor') || 
                            node.querySelectorAll && node.querySelectorAll('.anchor').length > 0) {
                            anchorAdded = true;
                        }
                    }
                });
            }
        });
        
        // If anchors were added and we have a hash in the URL, try to scroll
        if (anchorAdded && window.location.hash) {
            console.log('Anchors detected in DOM, attempting scroll');
            setTimeout(() => handleAnchorScroll(), 50);
        }
    });

    // Start observing the preview content area
    const previewContent = document.getElementById('preview-content');
    if (previewContent) {
        observer.observe(previewContent, {
            childList: true,
            subtree: true
        });
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
        selectFile(event.state.path, event.state.anchor);
        // Handle anchor after file is loaded
        if (event.state.anchor) {
            setTimeout(() => handleAnchorScroll(), 500);
        }
    } else {
        // Handle navigation to root
        // console.log("cleared");
        // currentFile = null;
        // document.getElementById('current-path').textContent = '';
        // document.getElementById('preview').innerHTML = '<div class="empty-state">Select a file to preview</div>';
        // document.querySelectorAll('#file-list li').forEach(li => {
        //     li.classList.remove('active');
        // });
    }
});

// Handle direct anchor navigation within the same page
window.addEventListener('hashchange', () => {
    console.log('hash changed');
    handleAnchorScroll();
});
