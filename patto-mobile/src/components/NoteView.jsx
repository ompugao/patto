// NoteView component - displays rendered note content with search

import { useStore, View } from '../lib/store';
import { openUrl } from '@tauri-apps/plugin-opener';
import { invoke } from '@tauri-apps/api/core';
import { useEffect, useRef, useState, useMemo } from 'react';
import './NoteView.css';

export function NoteView() {
    const {
        currentNote,
        renderedHtml,
        toggleEdit,
        closeNote,
        openNote,
        workspacePath,
    } = useStore();

    const contentRef = useRef(null);
    const [searchQuery, setSearchQuery] = useState('');
    const [activeQuery, setActiveQuery] = useState(''); // Query that's actually being searched
    const [showSearch, setShowSearch] = useState(false);
    const [currentMatchIndex, setCurrentMatchIndex] = useState(0);
    const [totalMatches, setTotalMatches] = useState(0);
    const searchInputRef = useRef(null);

    // Convert local image paths to base64 data URLs
    useEffect(() => {
        if (!contentRef.current) return;

        const loadImages = async () => {
            const images = contentRef.current.querySelectorAll('img');
            for (const img of images) {
                const src = img.getAttribute('src');
                // Check for https://asset.localhost paths (local images)
                if (src && src.startsWith('https://asset.localhost/')) {
                    const filePath = src.replace('https://asset.localhost/', '');
                    try {
                        const dataUrl = await invoke('get_image_base64', { path: filePath });
                        img.src = dataUrl;
                    } catch (e) {
                        console.error('Failed to load image:', e);
                        img.alt = 'Image not found';
                    }
                }
            }
        };

        loadImages();
    }, [renderedHtml, activeQuery]);

    // Create highlighted HTML
    const highlightedHtml = useMemo(() => {
        if (!activeQuery.trim() || !renderedHtml) {
            return renderedHtml;
        }

        const query = activeQuery;
        const regex = new RegExp(`(${escapeRegex(query)})`, 'gi');

        // Count matches
        const matches = renderedHtml.match(regex);
        setTotalMatches(matches ? matches.length : 0);

        // Simple text replacement - but avoid replacing inside HTML tags
        let matchIndex = 0;
        const result = renderedHtml.replace(/>([^<]*)</g, (fullMatch, textContent) => {
            if (!textContent.trim()) return fullMatch;

            const highlighted = textContent.replace(regex, (match) => {
                const idx = matchIndex++;
                const isCurrent = idx === currentMatchIndex;
                return `<mark class="search-match${isCurrent ? ' current' : ''}" data-idx="${idx}">${match}</mark>`;
            });
            return '>' + highlighted + '<';
        });

        return result;
    }, [renderedHtml, activeQuery, currentMatchIndex]);

    // Scroll to current match when it changes
    useEffect(() => {
        if (!contentRef.current || totalMatches === 0) return;

        const currentEl = contentRef.current.querySelector('.search-match.current');
        if (currentEl) {
            currentEl.scrollIntoView({ behavior: 'smooth', block: 'center' });
        }
    }, [currentMatchIndex, highlightedHtml]);

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
                try {
                    await openUrl(href);
                } catch (err) {
                    console.error('Failed to open URL:', err);
                }
                return;
            }

            const [noteName, anchor] = href.split('#');
            if (noteName) {
                const notePath = noteName.endsWith('.pn') ? noteName : `${noteName}.pn`;
                openNote(notePath);
            }
        };

        contentRef.current.addEventListener('click', handleClick);
        return () => {
            contentRef.current?.removeEventListener('click', handleClick);
        };
    }, [highlightedHtml, openNote]);

    // Focus search input when shown
    useEffect(() => {
        if (showSearch && searchInputRef.current) {
            searchInputRef.current.focus();
        }
    }, [showSearch]);

    // Perform search
    const doSearch = () => {
        setActiveQuery(searchQuery);
        setCurrentMatchIndex(0);
    };

    // Navigate matches
    const goToNext = () => {
        if (totalMatches === 0) return;
        setCurrentMatchIndex((currentMatchIndex + 1) % totalMatches);
    };

    const goToPrev = () => {
        if (totalMatches === 0) return;
        setCurrentMatchIndex(currentMatchIndex === 0 ? totalMatches - 1 : currentMatchIndex - 1);
    };

    // Handle key events
    const handleKeyDown = (e) => {
        if (e.key === 'Enter') {
            e.preventDefault();
            if (!activeQuery || activeQuery !== searchQuery) {
                doSearch();
            } else if (e.shiftKey) {
                goToPrev();
            } else {
                goToNext();
            }
        } else if (e.key === 'Escape') {
            handleClose();
        }
    };

    // Close search
    const handleClose = () => {
        setSearchQuery('');
        setActiveQuery('');
        setTotalMatches(0);
        setCurrentMatchIndex(0);
        setShowSearch(false);
    };

    const noteName = currentNote?.replace(/\.pn$/, '') || 'Note';

    return (
        <div className="note-view">
            <header className="note-header">
                <button className="back-btn" onClick={closeNote}>
                    ← Back
                </button>
                <h1 className="note-title">{noteName}</h1>
                <button className="edit-btn" onClick={toggleEdit}>
                    Edit
                </button>
            </header>

            {/* Search bar */}
            {showSearch && (
                <div className="search-bar">
                    <input
                        ref={searchInputRef}
                        type="text"
                        className="search-input"
                        placeholder="Search..."
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                        onKeyDown={handleKeyDown}
                    />
                    <button type="button" className="search-action-btn" onClick={doSearch}>
                        🔍
                    </button>
                    {totalMatches > 0 && (
                        <span className="match-info">
                            {currentMatchIndex + 1}/{totalMatches}
                        </span>
                    )}
                    <button type="button" className="search-action-btn" onClick={goToPrev} disabled={totalMatches === 0}>
                        ↑
                    </button>
                    <button type="button" className="search-action-btn" onClick={goToNext} disabled={totalMatches === 0}>
                        ↓
                    </button>
                    <button type="button" className="search-close-btn" onClick={handleClose}>
                        ✕
                    </button>
                </div>
            )}

            {!showSearch && (
                <button className="search-toggle-btn" onClick={() => setShowSearch(true)}>
                    🔍
                </button>
            )}

            <article
                ref={contentRef}
                className="note-content"
                dangerouslySetInnerHTML={{ __html: highlightedHtml }}
            />
        </div>
    );
}

// Escape special regex characters
function escapeRegex(str) {
    return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
