'use client';

import { useMemo, useCallback } from 'react';
import {
    // Twitter
    isTwitterPlaceholder,
    transformTwitterPlaceholder,
    // SpeakerDeck
    isSpeakerDeckPlaceholder,
    transformSpeakerDeckPlaceholder,
    // Links
    isSelfLink,
    isWikiLink,
    isLocalFileLink,
    transformSelfLink,
    transformWikiLink,
    transformLocalFileLink,
    // Images
    isImage,
    transformImage,
    // Code
    isMermaidBlock,
    isCodeElement,
    transformMermaidBlock,
    transformCodeElement,
    // Form elements
    isTable,
    isButton,
    isForm,
    isCheckbox,
    isInput,
    isBlockquote,
    transformTable,
    transformButton,
    transformForm,
    transformCheckbox,
    transformInput,
    transformBlockquote,
    // Patto elements
    isPattoLine,
    isPattoItem,
    isAnchorSpan,
    isTaskDeadline,
    isAside,
    transformPattoLine,
    transformPattoItem,
    transformAnchorSpan,
    transformTaskDeadline,
    transformAside,
} from '../components/transforms';

/**
 * Custom hook that creates memoized HTML transform options for html-react-parser.
 * This centralizes all the transformation logic and makes it reusable.
 * 
 * @param {Function} onSelectFile - Callback for navigating to a file
 * @returns {Object} Transform options object for html-react-parser
 */
export function useHtmlTransformer(onSelectFile) {
    /**
     * Create transform options with the replace function
     */
    const transformOptions = useMemo(() => {
        // Self-referencing transform options for recursive transforms
        const options = {
            replace: (domNode) => {
                // Embed transforms (highest priority)
                if (isTwitterPlaceholder(domNode)) {
                    return transformTwitterPlaceholder(domNode);
                }

                if (isSpeakerDeckPlaceholder(domNode)) {
                    return transformSpeakerDeckPlaceholder(domNode);
                }

                // Link transforms
                if (isSelfLink(domNode)) {
                    return transformSelfLink(domNode);
                }

                if (isWikiLink(domNode)) {
                    return transformWikiLink(domNode, onSelectFile);
                }

                if (isLocalFileLink(domNode)) {
                    return transformLocalFileLink(domNode);
                }

                // Image transforms
                if (isImage(domNode)) {
                    return transformImage(domNode);
                }

                // Patto line/item transforms
                if (isPattoLine(domNode)) {
                    return transformPattoLine(domNode, options);
                }

                if (isPattoItem(domNode)) {
                    return transformPattoItem(domNode, options);
                }

                // Code transforms
                if (isMermaidBlock(domNode)) {
                    return transformMermaidBlock(domNode);
                }

                if (isCodeElement(domNode)) {
                    return transformCodeElement(domNode);
                }

                // Form/table transforms
                if (isTable(domNode)) {
                    return transformTable(domNode, options);
                }

                if (isButton(domNode)) {
                    return transformButton(domNode, options);
                }

                if (isForm(domNode)) {
                    return transformForm(domNode, options);
                }

                if (isCheckbox(domNode)) {
                    return transformCheckbox(domNode);
                }

                if (isInput(domNode)) {
                    return transformInput(domNode);
                }

                if (isBlockquote(domNode)) {
                    return transformBlockquote(domNode, options);
                }

                // Other patto-specific transforms
                if (isAnchorSpan(domNode)) {
                    return transformAnchorSpan(domNode, options);
                }

                if (isTaskDeadline(domNode)) {
                    return transformTaskDeadline(domNode, options);
                }

                if (isAside(domNode)) {
                    return transformAside(domNode, options);
                }

                // Return undefined to let html-react-parser handle the node normally
                return undefined;
            }
        };

        return options;
    }, [onSelectFile]);

    return transformOptions;
}

/**
 * Escape invalid HTML tags to prevent parsing errors
 * @param {string} html - Raw HTML string
 * @returns {string} HTML with invalid tags escaped
 */
export function escapeInvalidTags(html) {
    if (typeof document === 'undefined') {
        // Server-side: return as-is
        return html;
    }

    const tagRegex = /<\/?([a-zA-Z][\w:-]*)\b[^>]*>/g;

    return html.replace(tagRegex, (match, tagName) => {
        const testEl = document.createElement(tagName.toLowerCase());
        const isKnown = !(testEl instanceof HTMLUnknownElement);

        if (isKnown) {
            return match; // leave valid tag alone
        }

        // escape the entire tag
        return match
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;');
    });
}
