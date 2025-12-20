// Zustand store for patto-mobile
// Manages app state: workspace, files, current note, git status

import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

// Views
export const View = {
    FILE_LIST: 'file_list',
    NOTE_VIEW: 'note_view',
    NOTE_EDIT: 'note_edit',
    TASKS: 'tasks',
    GIT_CONFIG: 'git_config',
};

// Sort options
export const SortBy = {
    LAST_MODIFIED: 'lastModified',
    LAST_CREATED: 'lastCreated',
    MOST_LINKED: 'mostLinked',
    ALPHABETICAL: 'alphabetical',
};

export const useStore = create((set, get) => ({
    // === Workspace ===
    workspacePath: null,

    // === View State ===
    currentView: View.FILE_LIST,

    // === Files ===
    files: [],
    sortBy: SortBy.LAST_MODIFIED,
    isLoadingFiles: false,

    // === Current Note ===
    currentNote: null,
    noteContent: '',
    renderedHtml: '',
    isEditing: false,

    // === Tasks ===
    tasks: null,
    isLoadingTasks: false,

    // === Git ===
    gitStatus: null,
    gitCredentials: { username: '', token: '' },
    isGitSyncing: false,

    // === Actions ===

    setWorkspacePath: (path) => set({ workspacePath: path }),

    setView: (view) => set({ currentView: view }),

    setSortBy: async (sortBy) => {
        set({ sortBy });
        await get().loadFiles();
    },

    // Load file list
    loadFiles: async () => {
        const { workspacePath, sortBy } = get();
        if (!workspacePath) return;

        set({ isLoadingFiles: true });
        try {
            const files = await invoke('list_files', { root: workspacePath, sortBy });
            set({ files, isLoadingFiles: false });
        } catch (error) {
            console.error('Failed to load files:', error);
            set({ isLoadingFiles: false });
        }
    },

    // Open a note
    openNote: async (filePath) => {
        const { workspacePath } = get();
        if (!workspacePath) return;

        try {
            const result = await invoke('render_note', {
                root: workspacePath,
                filePath
            });
            set({
                currentNote: filePath,
                noteContent: result.rawContent,
                renderedHtml: result.html,
                currentView: View.NOTE_VIEW,
                isEditing: false,
            });
        } catch (error) {
            console.error('Failed to open note:', error);
        }
    },

    // Toggle edit mode
    toggleEdit: () => {
        const { isEditing } = get();
        set({
            isEditing: !isEditing,
            currentView: isEditing ? View.NOTE_VIEW : View.NOTE_EDIT,
        });
    },

    // Update note content (while editing)
    setNoteContent: (content) => set({ noteContent: content }),

    // Save note
    saveNote: async () => {
        const { workspacePath, currentNote, noteContent } = get();
        if (!workspacePath || !currentNote) return;

        try {
            await invoke('write_note', {
                root: workspacePath,
                filePath: currentNote,
                content: noteContent
            });
            // Re-render after save
            const html = await invoke('render_content', { content: noteContent });
            set({ renderedHtml: html });
        } catch (error) {
            console.error('Failed to save note:', error);
        }
    },

    // Close note and go back to file list
    closeNote: () => set({
        currentNote: null,
        noteContent: '',
        renderedHtml: '',
        isEditing: false,
        currentView: View.FILE_LIST,
    }),

    // Load tasks
    loadTasks: async () => {
        const { workspacePath } = get();
        if (!workspacePath) return;

        set({ isLoadingTasks: true });
        try {
            const tasks = await invoke('get_all_tasks', { root: workspacePath });
            set({ tasks, isLoadingTasks: false });
        } catch (error) {
            console.error('Failed to load tasks:', error);
            set({ isLoadingTasks: false });
        }
    },

    // Git actions
    setGitCredentials: (credentials) => set({ gitCredentials: credentials }),

    gitSync: async () => {
        const { workspacePath, gitCredentials } = get();
        if (!workspacePath) return;

        set({ isGitSyncing: true });
        try {
            // Pull first, then sync
            await invoke('git_pull', { repoPath: workspacePath, credentials: gitCredentials });
            await invoke('git_sync', {
                repoPath: workspacePath,
                message: 'Sync from patto-mobile',
                credentials: gitCredentials
            });
            // Reload files after sync
            await get().loadFiles();
        } catch (error) {
            console.error('Git sync failed:', error);
        } finally {
            set({ isGitSyncing: false });
        }
    },

    loadGitStatus: async () => {
        const { workspacePath } = get();
        if (!workspacePath) return;

        try {
            const status = await invoke('git_status', { repoPath: workspacePath });
            set({ gitStatus: status });
        } catch (error) {
            console.error('Failed to get git status:', error);
        }
    },
}));
