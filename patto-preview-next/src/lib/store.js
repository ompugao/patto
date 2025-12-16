import { create } from 'zustand';
import { MessageTypes, createSelectFileMessage } from './messageTypes';

/**
 * Setup print event listeners to strip .pn extension from title when printing.
 * This ensures PDF filenames are clean (e.g., "file_basename.pdf" instead of "file_basename.pn.pdf").
 * @returns {Function} Cleanup function to remove event listeners
 */
function setupPrintTitleHandler() {
    if (typeof window === 'undefined') return () => {};

    let originalTitle = '';

    const handleBeforePrint = () => {
        originalTitle = document.title;
        if (originalTitle.endsWith('.pn')) {
            document.title = originalTitle.slice(0, -3);
        }
    };

    const handleAfterPrint = () => {
        if (originalTitle) {
            document.title = originalTitle;
        }
    };

    window.addEventListener('beforeprint', handleBeforePrint);
    window.addEventListener('afterprint', handleAfterPrint);

    return () => {
        window.removeEventListener('beforeprint', handleBeforePrint);
        window.removeEventListener('afterprint', handleAfterPrint);
    };
}

/**
 * Connection states for WebSocket
 */
export const ConnectionState = {
    CONNECTING: 'connecting',
    CONNECTED: 'connected',
    DISCONNECTED: 'disconnected',
    RECONNECTING: 'reconnecting',
};

/**
 * Adaptive throttle state for render-time-aware update batching.
 * Tracks render performance and skips intermediate updates when client is slow.
 */
const createAdaptiveThrottle = () => ({
    renderTimeEma: 16,      // Exponential moving average of render time (ms), start at ~60fps
    lastRenderStart: 0,     // Timestamp when last render started
    pendingUpdate: null,    // Queued update when throttled
    throttleTimeout: null,  // Timeout for processing pending update
    isRendering: false,     // Whether we're currently in a render cycle
});

/**
 * Zustand store for patto preview application.
 * Combines data state, UI state, and WebSocket connection management.
 */
export const usePattoStore = create((set, get) => ({
    // === Data State ===
    files: [],
    fileMetadata: {},
    previewHtml: '',
    backLinks: [],
    twoHopLinks: [],

    // === Routing State ===
    currentNote: null,
    anchor: null,

    // === UI State ===
    sortBy: 'modified',
    sidebarCollapsed: false,

    // === Connection State ===
    connectionState: ConnectionState.DISCONNECTED,

    // === WebSocket (managed externally, ref stored here) ===
    _ws: null,
    _retryCount: 0,
    _retryTimeout: null,

    // === Adaptive Throttle State ===
    _throttle: createAdaptiveThrottle(),

    // === Actions ===

    /**
     * Mark render start - call this before rendering preview content
     */
    markRenderStart: () => {
        const { _throttle } = get();
        _throttle.lastRenderStart = performance.now();
        _throttle.isRendering = true;
    },

    /**
     * Mark render complete - call this after rendering preview content
     * Updates the exponential moving average of render time
     */
    markRenderComplete: () => {
        const { _throttle } = get();
        if (_throttle.lastRenderStart > 0) {
            const renderTime = performance.now() - _throttle.lastRenderStart;
            // EMA with alpha=0.3 for smoothing
            _throttle.renderTimeEma = 0.3 * renderTime + 0.7 * _throttle.renderTimeEma;
            _throttle.isRendering = false;
        }
    },

    /**
     * Get adaptive throttle delay based on render performance
     * Returns delay in ms (1.5x the EMA, bounded 8-500ms)
     */
    _getThrottleDelay: () => {
        const { _throttle } = get();
        return Math.min(500, Math.max(8, _throttle.renderTimeEma * 1.5));
    },

    /**
     * Process a FILE_CHANGED update (possibly throttled)
     */
    _processFileChanged: (data) => {
        const { currentNote, _throttle, markRenderStart } = get();
        const isCurrentFile = data.path === currentNote;

        if (isCurrentFile) {
            markRenderStart();
        }

        set(state => ({
            previewHtml: isCurrentFile ? (data.html || '') : state.previewHtml,
            files: state.files.includes(data.path)
                ? state.files
                : [...state.files, data.path],
            fileMetadata: {
                ...state.fileMetadata,
                [data.path]: data.metadata,
            },
        }));

        _throttle.pendingUpdate = null;
    },

    /**
     * Handle incoming WebSocket messages
     */
    handleMessage: (message) => {
        const { type, data } = message;
        const { currentNote, _throttle, _processFileChanged, _getThrottleDelay } = get();

        switch (type) {
            case MessageTypes.FILE_LIST:
                set({
                    files: data.files || [],
                    fileMetadata: data.metadata || {},
                });
                break;

            case MessageTypes.FILE_CHANGED: {
                const isCurrentFile = data.path === currentNote;

                // For non-current files, process immediately (cheap update)
                if (!isCurrentFile) {
                    set(state => ({
                        files: state.files.includes(data.path)
                            ? state.files
                            : [...state.files, data.path],
                        fileMetadata: {
                            ...state.fileMetadata,
                            [data.path]: data.metadata,
                        },
                    }));
                    break;
                }

                // For current file: use adaptive throttling
                // If we're still rendering or within throttle window, queue the update
                if (_throttle.isRendering || _throttle.throttleTimeout) {
                    // Replace pending update with latest (drop intermediate updates)
                    _throttle.pendingUpdate = data;

                    // Schedule processing if not already scheduled
                    if (!_throttle.throttleTimeout) {
                        const delay = _getThrottleDelay();
                        _throttle.throttleTimeout = setTimeout(() => {
                            _throttle.throttleTimeout = null;
                            const pending = _throttle.pendingUpdate;
                            if (pending) {
                                _processFileChanged(pending);
                            }
                        }, delay);
                    }
                } else {
                    // Process immediately
                    _processFileChanged(data);

                    // Set up throttle window to batch rapid subsequent updates
                    const delay = _getThrottleDelay();
                    _throttle.throttleTimeout = setTimeout(() => {
                        _throttle.throttleTimeout = null;
                        const pending = _throttle.pendingUpdate;
                        if (pending) {
                            _processFileChanged(pending);
                        }
                    }, delay);
                }
                break;
            }

            case MessageTypes.FILE_ADDED:
                set(state => ({
                    files: state.files.includes(data.path)
                        ? state.files
                        : [...state.files, data.path],
                    fileMetadata: {
                        ...state.fileMetadata,
                        [data.path]: data.metadata,
                    },
                }));
                break;

            case MessageTypes.FILE_REMOVED: {
                const isCurrentFile = data.path === currentNote;
                set(state => {
                    const newMetadata = { ...state.fileMetadata };
                    delete newMetadata[data.path];
                    const newFiles = state.files.filter(f => f !== data.path);
                    return {
                        files: newFiles,
                        fileMetadata: newMetadata,
                        // Clear preview if current file was removed
                        previewHtml: isCurrentFile ? '' : state.previewHtml,
                        backLinks: isCurrentFile ? [] : state.backLinks,
                        twoHopLinks: isCurrentFile ? [] : state.twoHopLinks,
                        // Also clear currentNote if the file was removed
                        currentNote: isCurrentFile ? null : state.currentNote,
                    };
                });
                // Navigate home if current file was removed
                if (isCurrentFile && typeof window !== 'undefined') {
                    document.title = '';
                    const url = new URL(window.location);
                    url.searchParams.delete('note');
                    url.hash = '';
                    window.history.pushState({}, '', url.toString());
                }
                break;
            }

            case MessageTypes.BACK_LINKS_DATA:
                if (data.path === currentNote) {
                    set({ backLinks: data.back_links || [] });
                }
                break;

            case MessageTypes.TWO_HOP_LINKS_DATA:
                if (data.path === currentNote) {
                    set({ twoHopLinks: data.two_hop_links || [] });
                }
                break;
        }
    },

    /**
     * Send a message through WebSocket
     */
    sendMessage: (message) => {
        const { _ws } = get();
        if (_ws?.readyState === WebSocket.OPEN) {
            _ws.send(JSON.stringify(message));
        }
    },

    /**
     * Select a file and navigate to it
     */
    selectFile: (path, anchorId = null) => {
        if (typeof window === 'undefined') return;

        // Clear previous preview
        set({ previewHtml: '', backLinks: [], twoHopLinks: [] });

        // Update URL
        const url = new URL(window.location);
        if (path) {
            url.searchParams.set('note', path);
        } else {
            url.searchParams.delete('note');
        }
        url.hash = anchorId ? `#${anchorId}` : '';
        window.history.pushState({ note: path, anchor: anchorId }, '', url.toString());

        // Update state
        set({ currentNote: path, anchor: anchorId });

        // Update title
        document.title = path || '';

        // Request file content from server
        if (path) {
            get().sendMessage(createSelectFileMessage(path));
        }
    },

    /**
     * Navigate to home (clear selection)
     */
    navigateHome: () => {
        get().selectFile(null, null);
    },

    /**
     * Update sort preference
     */
    setSortBy: (newSort) => {
        set({ sortBy: newSort });
        if (typeof window !== 'undefined') {
            localStorage.setItem('patto-sort-order', newSort);
        }
    },

    /**
     * Toggle sidebar visibility
     */
    toggleSidebar: () => {
        set(state => {
            const newValue = !state.sidebarCollapsed;
            if (typeof window !== 'undefined') {
                localStorage.setItem('sidebar-collapsed', newValue.toString());
            }
            return { sidebarCollapsed: newValue };
        });
    },

    /**
     * Initialize store from URL and localStorage
     */
    initialize: () => {
        if (typeof window === 'undefined') return;

        // Parse URL
        const urlParams = new URLSearchParams(window.location.search);
        const noteParam = urlParams.get('note');
        const hashAnchor = window.location.hash ? window.location.hash.substring(1) : null;

        // Load UI preferences
        const savedSort = localStorage.getItem('patto-sort-order');
        const savedCollapsed = localStorage.getItem('sidebar-collapsed') === 'true';

        set({
            currentNote: noteParam,
            anchor: hashAnchor,
            sortBy: savedSort || 'modified',
            sidebarCollapsed: savedCollapsed,
        });

        // Set initial title
        if (noteParam) {
            document.title = noteParam;
        }

        // Setup print title handler (strips .pn extension when printing to PDF)
        const cleanupPrintHandler = setupPrintTitleHandler();

        // Handle browser navigation
        const handlePopState = () => {
            const params = new URLSearchParams(window.location.search);
            set({
                currentNote: params.get('note'),
                anchor: window.location.hash ? window.location.hash.substring(1) : null,
            });
        };
        window.addEventListener('popstate', handlePopState);

        return () => {
            window.removeEventListener('popstate', handlePopState);
            cleanupPrintHandler();
        };
    },

    /**
     * Connect to WebSocket server
     */
    connect: () => {
        if (typeof window === 'undefined') return;

        const { _ws, _retryCount } = get();
        if (_ws) _ws.close();

        set({
            connectionState: _retryCount > 0 ? ConnectionState.RECONNECTING : ConnectionState.CONNECTING
        });

        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const ws = new WebSocket(`${protocol}//${window.location.host}/ws`);
        set({ _ws: ws });

        ws.onopen = () => {
            set({ connectionState: ConnectionState.CONNECTED, _retryCount: 0 });
            // Request current file if set
            const { currentNote } = get();
            if (currentNote) {
                get().sendMessage(createSelectFileMessage(currentNote));
            }
        };

        ws.onmessage = (event) => {
            try {
                const message = JSON.parse(event.data);
                get().handleMessage(message);
            } catch (error) {
                console.error('WebSocket message error:', error);
            }
        };

        ws.onclose = (event) => {
            set({ connectionState: ConnectionState.DISCONNECTED });
            const { _retryCount } = get();
            if (event.code !== 1000 && _retryCount < 5) {
                const delay = Math.min(1000 * Math.pow(2, _retryCount), 16000);
                const timeout = setTimeout(() => {
                    set({ _retryCount: _retryCount + 1 });
                    get().connect();
                }, delay);
                set({ _retryTimeout: timeout });
            }
        };

        ws.onerror = () => { };
    },

    /**
     * Disconnect WebSocket
     */
    disconnect: () => {
        const { _ws, _retryTimeout } = get();
        if (_retryTimeout) clearTimeout(_retryTimeout);
        if (_ws) _ws.close(1000);
        set({ _ws: null, _retryTimeout: null });
    },
}));

/**
 * Get connection indicator style
 */
export function getConnectionIndicator(connectionState) {
    return {
        width: '6px',
        height: '6px',
        borderRadius: '999px',
        backgroundColor: {
            [ConnectionState.CONNECTED]: '#6ecf82',
            [ConnectionState.CONNECTING]: '#f5c16c',
            [ConnectionState.RECONNECTING]: '#f5c16c',
            [ConnectionState.DISCONNECTED]: '#e27b7b',
        }[connectionState] || '#9aa0a6',
        marginRight: '0px',
    };
}
