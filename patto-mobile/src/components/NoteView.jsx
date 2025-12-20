// NoteView component - displays rendered note content

import { useStore, View } from '../lib/store';
import { openUrl } from '@tauri-apps/plugin-opener';
import { useEffect, useRef } from 'react';
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

            <article
                ref={contentRef}
                className="note-content"
                dangerouslySetInnerHTML={{ __html: renderedHtml }}
            />
        </div>
    );
}
