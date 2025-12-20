// NoteEditor component - simple text editor for patto notes

import { useStore } from '../lib/store';
import { useEffect, useRef } from 'react';
import './NoteEditor.css';

export function NoteEditor() {
    const {
        currentNote,
        noteContent,
        setNoteContent,
        saveNote,
        toggleEdit,
        closeNote,
    } = useStore();

    const textareaRef = useRef(null);

    // Auto-focus on textarea
    useEffect(() => {
        if (textareaRef.current) {
            textareaRef.current.focus();
        }
    }, []);

    // Auto-save on debounce
    useEffect(() => {
        const timer = setTimeout(() => {
            saveNote();
        }, 1000);
        return () => clearTimeout(timer);
    }, [noteContent, saveNote]);

    const noteName = currentNote?.replace(/\.pn$/, '') || 'Note';

    const handleKeyDown = (e) => {
        // Handle Tab for indentation
        if (e.key === 'Tab') {
            e.preventDefault();
            const start = e.target.selectionStart;
            const end = e.target.selectionEnd;
            const value = noteContent;

            // Insert tab character
            const newValue = value.substring(0, start) + '\t' + value.substring(end);
            setNoteContent(newValue);

            // Move cursor after tab
            setTimeout(() => {
                e.target.selectionStart = e.target.selectionEnd = start + 1;
            }, 0);
        }
    };

    return (
        <div className="note-editor">
            <header className="editor-header">
                <button className="back-btn" onClick={closeNote}>
                    ← Back
                </button>
                <h1 className="editor-title">{noteName}</h1>
                <button className="preview-btn" onClick={toggleEdit}>
                    Preview
                </button>
            </header>

            <textarea
                ref={textareaRef}
                className="editor-textarea"
                value={noteContent}
                onChange={(e) => setNoteContent(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder="Start writing..."
                spellCheck={false}
            />

            <footer className="editor-footer">
                <span className="save-status">Auto-saving...</span>
            </footer>
        </div>
    );
}
