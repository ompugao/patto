// NoteView component - displays rendered note content with search
// Uses browser's native window.find() for better performance

import { useStore, View } from '../lib/store';
import { openUrl } from '@tauri-apps/plugin-opener';
import { convertFileSrc } from '@tauri-apps/api/core';
import { useEffect, useRef, useState, useCallback } from 'react';
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
    const [showSearch, setShowSearch] = useState(false);
    const [matchInfo, setMatchInfo] = useState('');
    const searchInputRef = useRef(null);

    // Convert file:// URLs to proper Tauri asset URLs for images
    useEffect(() => {
        if (!contentRef.current) return;

        const images = contentRef.current.querySelectorAll('img');
        images.forEach(img => {
            const src = img.getAttribute('src');
            if (src && src.startsWith('file://')) {
                // Convert file:// to proper Tauri asset URL
                const filePath = src.replace('file://', '');
                try {
                    img.src = convertFileSrc(filePath);
                } catch (e) {
                    console.error('Failed to convert image src:', e);
                }
            }
        });
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

            // External URL - open in system browser
            if (href.startsWith('http://') || href.startsWith('https://')) {
                try {
                    await openUrl(href);
                } catch (err) {
                    console.error('Failed to open URL:', err);
                }
                return;
            }

            // Internal wiki link - navigate to note
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
    }, [renderedHtml, openNote]);

    // Focus search input when shown
    useEffect(() => {
        if (showSearch && searchInputRef.current) {
            searchInputRef.current.focus();
        }
    }, [showSearch]);

    // Clear search highlights when closing
    const clearSearch = useCallback(() => {
        // Clear selection
        if (window.getSelection) {
            window.getSelection().removeAllRanges();
        }
        setSearchQuery('');
        setMatchInfo('');
    }, []);

    // Perform search using browser's find
    const doSearch = useCallback((forward = true) => {
        if (!searchQuery.trim()) {
            setMatchInfo('');
            return;
        }

        // Use CSS to highlight matches
        if (window.CSS && CSS.highlights) {
            // Modern Highlight API (if available)
            const content = contentRef.current;
            if (!content) return;

            const text = content.textContent.toLowerCase();
            const query = searchQuery.toLowerCase();
            const matches = [];
            let index = 0;
            while ((index = text.indexOf(query, index)) !== -1) {
                matches.push(index);
                index += query.length;
            }
            setMatchInfo(matches.length > 0 ? `${matches.length} matches` : 'No matches');
        } else {
            // Fallback: use window.find (deprecated but works)
            const found = window.find(searchQuery, false, !forward, true, false, false, false);
            setMatchInfo(found ? 'Found' : 'No matches');
        }
    }, [searchQuery]);

    const handleSearchSubmit = (e) => {
        e.preventDefault();
        doSearch(true);
    };

    const handleSearchKeyDown = (e) => {
        if (e.key === 'Escape') {
            e.preventDefault();
            clearSearch();
            setShowSearch(false);
        }
    };

    const findNext = () => doSearch(true);
    const findPrev = () => doSearch(false);

    const handleClose = () => {
        clearSearch();
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
                <form className="search-bar" onSubmit={handleSearchSubmit}>
                    <input
                        ref={searchInputRef}
                        type="text"
                        className="search-input"
                        placeholder="Search..."
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                        onKeyDown={handleSearchKeyDown}
                    />
                    <span className="match-info">{matchInfo}</span>
                    <button type="submit" className="search-action-btn">
                        Find
                    </button>
                    <button type="button" className="search-action-btn" onClick={findPrev}>
                        ↑
                    </button>
                    <button type="button" className="search-action-btn" onClick={findNext}>
                        ↓
                    </button>
                    <button type="button" className="search-close-btn" onClick={handleClose}>
                        ✕
                    </button>
                </form>
            )}

            {!showSearch && (
                <button className="search-toggle-btn" onClick={() => setShowSearch(true)}>
                    🔍
                </button>
            )}

            <article
                ref={contentRef}
                className="note-content"
                dangerouslySetInnerHTML={{ __html: renderedHtml }}
            />
        </div>
    );
}
