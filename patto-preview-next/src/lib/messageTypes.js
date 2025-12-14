'use client';

/**
 * WebSocket message types for patto-preview communication.
 * These must match the WsMessage enum in patto-preview.rs
 */
export const MessageTypes = {
    // Incoming messages (server -> client)
    FILE_LIST: 'FileList',
    FILE_CHANGED: 'FileChanged',
    FILE_ADDED: 'FileAdded',
    FILE_REMOVED: 'FileRemoved',
    BACK_LINKS_DATA: 'BackLinksData',
    TWO_HOP_LINKS_DATA: 'TwoHopLinksData',

    // Outgoing messages (client -> server)
    SELECT_FILE: 'SelectFile',
};

/**
 * @typedef {Object} FileMetadata
 * @property {number} modified - Unix timestamp of last modification
 * @property {number} created - Unix timestamp of creation
 * @property {number} linkCount - Number of incoming links
 */

/**
 * @typedef {Object} BackLinkData
 * @property {string} source_file - Source file path
 * @property {Array<{line: number, context: string}>} locations - Link locations
 */

/**
 * @typedef {Object} FileListPayload
 * @property {string[]} files - List of file paths
 * @property {Object<string, FileMetadata>} metadata - File metadata map
 */

/**
 * @typedef {Object} FileChangedPayload
 * @property {string} path - File path
 * @property {FileMetadata} metadata - File metadata
 * @property {string} html - Rendered HTML content
 */

/**
 * @typedef {Object} FileAddedPayload
 * @property {string} path - File path
 * @property {FileMetadata} metadata - File metadata
 */

/**
 * @typedef {Object} FileRemovedPayload
 * @property {string} path - File path
 */

/**
 * @typedef {Object} BackLinksPayload
 * @property {string} path - File path
 * @property {BackLinkData[]} back_links - Back link data
 */

/**
 * @typedef {Object} TwoHopLinksPayload
 * @property {string} path - File path
 * @property {Array<[string, string[]]>} two_hop_links - Two-hop link data
 */

/**
 * Create a SelectFile message to send to the server
 * @param {string} path - File path to select
 * @returns {{type: string, data: {path: string}}}
 */
export function createSelectFileMessage(path) {
    return {
        type: MessageTypes.SELECT_FILE,
        data: { path }
    };
}
