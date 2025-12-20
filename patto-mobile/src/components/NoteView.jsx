// NoteView component - displays rendered note content with simple search
// Re-queries elements on each navigation to avoid stale DOM references

import { useStore, View } from '../lib/store';
import { openUrl } from '@tauri-apps/plugin-opener';
import { invoke } from '@tauri-apps/api/core';
import { useEffect, useRef, useState } from 'react';
import './NoteView.css';

export function NoteView() {
    const {
        currentNote,
        renderedHtml,
        toggleEdit,
        closeNote,
        openNote,
    } = useStore();

    const contentRef = useRef(null);
    const [showSearch, setShowSearch] = useState(false);
    const inputRef = useRef(null);

    // Store only the query and index, not DOM elements
    const searchRef = useRef({ query: '', idx: 0, total: 0 });
    const [displayInfo, setDisplayInfo] = useState('');

    // Convert local image paths to base64 data URLs
    useEffect(() => {
        if (!contentRef.current) return;

        const loadImages = async () => {
            const images = contentRef.current.querySelectorAll('img');
            for (const img of images) {
                const src = img.getAttribute('src');
                if (src && src.startsWith('https://asset.localhost/')) {
                    const filePath = src.replace('https://asset.localhost/', '');
                    try {
                        const dataUrl = await invoke('get_image_base64', { path: filePath });
                        img.src = dataUrl;
                    } catch (e) {
                        img.alt = 'Image not found';
                    }
                }
            }
        };

        loadImages();
    }, [renderedHtml]);

    // Handle link clicks
    useEffect(() => {
        if (!contentRef.current) return;

        const handleClick = async (e) => {
            const link = e.target.closest('a');
            if (!link) return;

            e.preventDefault();
            const href = link.getAttribute('href');
            if (!href) return;

            if (href.startsWith('http://') || href.startsWith('https://')) {
                try { await openUrl(href); } catch (e) { }
                return;
            }

            const [noteName] = href.split('#');
            if (noteName) {
                openNote(noteName.endsWith('.pn') ? noteName : `${noteName}.pn`);
            }
        };

        contentRef.current.addEventListener('click', handleClick);
        return () => contentRef.current?.removeEventListener('click', handleClick);
    }, [renderedHtml, openNote]);

    // Focus input when search opens
    useEffect(() => {
        if (showSearch && inputRef.current) {
            inputRef.current.focus();
        }
    }, [showSearch]);

    // Get all matching elements fresh each time
    const getMatchingElements = () => {
        if (!contentRef.current || !searchRef.current.query) return [];

        const query = searchRef.current.query;
        const lines = contentRef.current.querySelectorAll('.patto-line');
        const found = [];

        lines.forEach(line => {
            if (line.textContent.toLowerCase().includes(query)) {
                found.push(line);
            }
        });

        return found;
    };

    // Navigate to index
    const navigateToIndex = (idx) => {
        const elements = getMatchingElements();
        if (elements.length === 0) return;

        // Wrap index
        idx = ((idx % elements.length) + elements.length) % elements.length;
        searchRef.current.idx = idx;
        searchRef.current.total = elements.length;

        // Clear previous highlight
        contentRef.current?.querySelectorAll('.search-active').forEach(e => {
            e.classList.remove('search-active');
        });

        // Highlight and scroll to current
        const el = elements[idx];
        el.classList.add('search-active');

        // Use native scrollIntoView
        try {
            el.scrollIntoView({ block: 'center', inline: 'nearest' });
        } catch (e) {
            // Fallback for older browsers
            el.scrollIntoView(true);
        }

        setDisplayInfo(`${idx + 1}/${elements.length}`);
    };

    // Find - starts new search
    const doFind = () => {
        const query = inputRef.current?.value?.trim().toLowerCase();
        if (!query || !contentRef.current) {
            searchRef.current = { query: '', idx: 0, total: 0 };
            setDisplayInfo('');
            contentRef.current?.querySelectorAll('.search-active').forEach(e => {
                e.classList.remove('search-active');
            });
            return;
        }

        searchRef.current.query = query;
        searchRef.current.idx = -1; // Will become 0 on navigateToIndex

        const elements = getMatchingElements();
        if (elements.length > 0) {
            navigateToIndex(0);
        } else {
            setDisplayInfo('0/0');
        }
    };

    // Go to next match
    const goNext = () => {
        if (!searchRef.current.query) return;
        navigateToIndex(searchRef.current.idx + 1);
    };

    // Go to previous match
    const goPrev = () => {
        if (!searchRef.current.query) return;
        navigateToIndex(searchRef.current.idx - 1);
    };

    const handleKeyDown = (e) => {
        if (e.key === 'Enter') {
            e.preventDefault();
            if (!searchRef.current.query || searchRef.current.total === 0) {
                doFind();
            } else if (e.shiftKey) {
                goPrev();
            } else {
                goNext();
            }
        } else if (e.key === 'Escape') {
            closeSearch();
        }
    };

    const closeSearch = () => {
        setShowSearch(false);
        searchRef.current = { query: '', idx: 0, total: 0 };
        setDisplayInfo('');
        contentRef.current?.querySelectorAll('.search-active').forEach(e => {
            e.classList.remove('search-active');
        });
    };

    const noteName = currentNote?.replace(/\.pn$/, '') || 'Note';

    return (
        <div className="note-view">
            <header className="note-header">
                <button className="back-btn" onClick={closeNote}>← Back</button>
                <h1 className="note-title">{noteName}</h1>
                <button className="edit-btn" onClick={toggleEdit}>Edit</button>
            </header>

            {showSearch && (
                <div className="search-bar">
                    <input
                        ref={inputRef}
                        type="text"
                        className="search-input"
                        placeholder="Search..."
                        onKeyDown={handleKeyDown}
                    />
                    <button type="button" className="search-action-btn" onClick={doFind}>🔍</button>
                    <span className="match-info">{displayInfo}</span>
                    <button type="button" className="search-action-btn" onClick={goPrev}>↑</button>
                    <button type="button" className="search-action-btn" onClick={goNext}>↓</button>
                    <button type="button" className="search-close-btn" onClick={closeSearch}>✕</button>
                </div>
            )}

            {!showSearch && (
                <button className="search-toggle-btn" onClick={() => setShowSearch(true)}>🔍</button>
            )}

            <article
                ref={contentRef}
                className="note-content"
                dangerouslySetInnerHTML={{ __html: renderedHtml }}
            />
        </div>
    );
}
