import type { Property } from './tasks'

// Matches the actual JSON shape from the Rust backend:
// AstNode is #[serde(transparent)] -> Annotation<AstNodeInternal>
// So: { value: { contents, children, kind, stable_id }, location: { row, span, input } }

export interface AstNodeKind {
    type: string;
    // Line / QuoteContent
    properties?: Property[];
    // Math / Code
    inline?: boolean;
    lang?: string;
    // Table
    caption?: string | null;
    // Image
    src?: string;
    alt?: string | null;
    // WikiLink
    link?: string;
    anchor?: string | null;
    // Link / Embed
    title?: string | null;
    // Decoration
    fontsize?: number;
    italic?: boolean;
    underline?: boolean;
    deleted?: boolean;
}

export interface AstNode {
    location: {
        row: number;
        span: [number, number];
        input: string;
    };
    value: {
        kind: AstNodeKind;
        contents: AstNode[];
        children: AstNode[];
        stable_id: number | null;
    };
}

// Rust generates span offsets as raw UTF-8 byte offsets (not JS UTF-16 characters)
// To extract the correct substring (especially necessary for Japanese CJK chars),
// we must convert the string to a UTF-8 ArrayBuffer, slice the byte range, and decode.
const decoder = new TextDecoder();
const encoder = new TextEncoder();

export function nodeText(node: AstNode): string {
    if (!node.location.span) return '';
    const [start, end] = node.location.span;
    const bytes = encoder.encode(node.location.input);
    return decoder.decode(bytes.slice(start, end));
}

export function joinedText(nodes: AstNode[], separator: string): string {
    return nodes.map(nodeText).join(separator);
}

/** The top-level blocks of a document: the root is a Dummy node wrapping them. */
export function flattenAst(node: AstNode): AstNode[] {
    if (!node) return [];
    if (node.value?.kind?.type === 'Dummy') {
        return node.value.children ?? [];
    }
    return [node];
}
