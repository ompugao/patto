// NoteView component - displays rendered note content with search

import { useStore, View } from '../lib/store';
import { openUrl } from '@tauri-apps/plugin-opener';
import { useEffect, useRef, useState } from 'react';
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
    const [matchCount, setMatchCount] = useState(0);
    const [currentMatch, setCurrentMatch] = useState(0);
    const searchInputRef = useRef(null);

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
            // Links are typically in format: note_name or note_name#anchor
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

    // Highlight search results
    useEffect(() => {
        if (!contentRef.current || !searchQuery.trim()) {
            setMatchCount(0);
            setCurrentMatch(0);
            return;
        }

        const content = contentRef.current;
        const query = searchQuery.toLowerCase();

        // Remove previous highlights
        content.querySelectorAll('.search-highlight').forEach(el => {
            const parent = el.parentNode;
            parent.replaceChild(document.createTextNode(el.textContent), el);
            parent.normalize();
        });

        if (!query) return;

        // Find and highlight matches using TreeWalker for text nodes
        const walker = document.createTreeWalker(content, NodeFilter.SHOW_TEXT, null, false);
        const matches = [];
        let node;

        while (node = walker.nextNode()) {
            const text = node.textContent.toLowerCase();
            let index = 0;
            while ((index = text.indexOf(query, index)) !== -1) {
                matches.push({ node, index });
                index += query.length;
            }
        }

        // Apply highlights (in reverse to preserve indices)
        matches.reverse().forEach((match, i) => {
            const range = document.createRange();
            range.setStart(match.node, match.index);
            range.setEnd(match.node, match.index + query.length);

            const highlight = document.createElement('mark');
            highlight.className = 'search-highlight';
            if (matches.length - 1 - i === currentMatch) {
                highlight.classList.add('current');
            }
            range.surroundContents(highlight);
        });

        setMatchCount(matches.length);
    }, [searchQuery, renderedHtml, currentMatch]);

    const handleSearchKeyDown = (e) => {
        if (e.key === 'Enter') {
            e.preventDefault();
            if (e.shiftKey) {
                // Previous match
                setCurrentMatch(prev => prev > 0 ? prev - 1 : matchCount - 1);
            } else {
                // Next match
                setCurrentMatch(prev => prev < matchCount - 1 ? prev + 1 : 0);
            }
        } else if (e.key === 'Escape') {
            setShowSearch(false);
            setSearchQuery('');
        }
    };

    const scrollToCurrentMatch = () => {
        const current = contentRef.current?.querySelector('.search-highlight.current');
        if (current) {
            current.scrollIntoView({ behavior: 'smooth', block: 'center' });
        }
    };

    useEffect(() => {
        scrollToCurrentMatch();
    }, [currentMatch]);

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
            {showSearch ? (
                <div className="search-bar">
                    <input
                        ref={searchInputRef}
                        type="text"
                        className="search-input"
                        placeholder="Search in note..."
                        value={searchQuery}
                        onChange={(e) => {
                            setSearchQuery(e.target.value);
                            setCurrentMatch(0);
                        }}
                        onKeyDown={handleSearchKeyDown}
                    />
                    {matchCount > 0 && (
                        <span className="match-count">
                            {currentMatch + 1}/{matchCount}
                        </span>
                    )}
                    <button className="search-nav-btn" onClick={() => setCurrentMatch(prev => prev > 0 ? prev - 1 : matchCount - 1)}>
                        ▲
                    </button>
                    <button className="search-nav-btn" onClick={() => setCurrentMatch(prev => prev < matchCount - 1 ? prev + 1 : 0)}>
                        ▼
                    </button>
                    <button className="search-close-btn" onClick={() => { setShowSearch(false); setSearchQuery(''); }}>
                        ✕
                    </button>
                </div>
            ) : (
                <button className="search-toggle-btn" onClick={() => setShowSearch(true)}>
                    🔍 Search
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
