'use client';

import { useState, useCallback, useMemo } from 'react';
import { MessageTypes, createSelectFileMessage } from './messageTypes';

/**
 * Initial state for the patto store
 */
const initialState = {
    files: [],
    fileMetadata: {},
    previewHtml: '',
    backLinks: [],
    twoHopLinks: [],
    currentNote: null,
};

/**
 * Reducer function for handling incoming WebSocket messages
 * @param {typeof initialState} state - Current state
 * @param {{type: string, data: Object, currentNote?: string}} action - Action with message type and payload
 * @returns {typeof initialState} New state
 */
function pattoReducer(state, action) {
    const { type, data, currentNote } = action;

    switch (type) {
        case MessageTypes.FILE_LIST:
            return {
                ...state,
                files: data.files || [],
                fileMetadata: data.metadata || {},
            };

        case MessageTypes.FILE_CHANGED: {
            const isCurrentFile = data.path === currentNote;
            return {
                ...state,
                previewHtml: isCurrentFile ? (data.html || '') : state.previewHtml,
                files: state.files.includes(data.path)
                    ? state.files
                    : [...state.files, data.path],
                fileMetadata: {
                    ...state.fileMetadata,
                    [data.path]: data.metadata,
                },
            };
        }

        case MessageTypes.FILE_ADDED:
            return {
                ...state,
                files: state.files.includes(data.path)
                    ? state.files
                    : [...state.files, data.path],
                fileMetadata: {
                    ...state.fileMetadata,
                    [data.path]: data.metadata,
                },
            };

        case MessageTypes.FILE_REMOVED: {
            const newMetadata = { ...state.fileMetadata };
            delete newMetadata[data.path];

            const isCurrentFile = data.path === currentNote;
            return {
                ...state,
                files: state.files.filter(f => f !== data.path),
                fileMetadata: newMetadata,
                // Clear preview if current file was removed
                previewHtml: isCurrentFile ? '' : state.previewHtml,
                backLinks: isCurrentFile ? [] : state.backLinks,
                twoHopLinks: isCurrentFile ? [] : state.twoHopLinks,
            };
        }

        case MessageTypes.BACK_LINKS_DATA:
            if (data.path === currentNote) {
                return {
                    ...state,
                    backLinks: data.back_links || [],
                };
            }
            return state;

        case MessageTypes.TWO_HOP_LINKS_DATA:
            if (data.path === currentNote) {
                return {
                    ...state,
                    twoHopLinks: data.two_hop_links || [],
                };
            }
            return state;

        case 'CLEAR_PREVIEW':
            return {
                ...state,
                previewHtml: '',
                backLinks: [],
                twoHopLinks: [],
            };

        case 'SET_CURRENT_NOTE':
            return {
                ...state,
                currentNote: data.path,
            };

        default:
            console.warn('Unknown message type:', type);
            return state;
    }
}

/**
 * Custom hook for managing patto preview state.
 * Centralizes all state management and provides typed action dispatchers.
 * 
 * @param {string|null} currentNote - Currently selected note path
 * @returns {{
 *   state: typeof initialState,
 *   dispatch: (action: {type: string, data: Object}) => void,
 *   actions: {
 *     clearPreview: () => void,
 *   }
 * }}
 */
export function usePattoStore(currentNote) {
    const [state, setState] = useState(initialState);

    /**
     * Dispatch an action to update state
     * @param {{type: string, data: Object}} action
     */
    const dispatch = useCallback((action) => {
        setState(prevState => pattoReducer(prevState, { ...action, currentNote }));
    }, [currentNote]);

    /**
     * Clear preview content
     */
    const clearPreview = useCallback(() => {
        dispatch({ type: 'CLEAR_PREVIEW', data: {} });
    }, [dispatch]);

    const actions = useMemo(() => ({
        clearPreview,
    }), [clearPreview]);

    return { state, dispatch, actions };
}

/**
 * Create a message handler function that dispatches to the store
 * @param {Function} dispatch - Dispatch function from usePattoStore
 * @param {string|null} currentNote - Currently selected note
 * @param {Function} sendMessage - WebSocket send function
 * @returns {Function} Message handler for WebSocket onMessage
 */
export function createMessageHandler(dispatch, currentNote, sendMessage) {
    return (data) => {
        console.log('WebSocket message:', data);

        // Dispatch the message to update state
        dispatch({ type: data.type, data: data.data });

        // Handle special case: on FileList, request current note if set
        if (data.type === MessageTypes.FILE_LIST && currentNote && sendMessage) {
            sendMessage(createSelectFileMessage(currentNote));
        }
    };
}

export { createSelectFileMessage };
