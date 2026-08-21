// The messages patto-preview exchanges over the WebSocket. Both directions use
// serde adjacent tagging — #[serde(tag = "type", content = "data")] — so every
// message is { type, data }. See WsServerMessage / WsClientMessage in
// src/bin/patto-preview.rs.
import type { AstNode } from './components/VirtualRenderer'

export interface FileMetadata {
  modified: number;
  created: number;
  linkCount: number;
}

/** A note as the sidebar lists it. */
export interface FileEntry {
  path: string;
  modified: number;
}

export type ServerMessage =
  | { type: 'FileList'; data: { files: string[]; metadata: Record<string, FileMetadata> } }
  // Sent both as the reply to a SelectFile and as the broadcast for any file
  // changing on disk. usePreviewSocket is what tells the two apart.
  | { type: 'FileChanged'; data: { path: string; metadata: FileMetadata; ast: AstNode | null } }
  | { type: 'FileAdded'; data: { path: string; metadata: FileMetadata } }
  | { type: 'FileRemoved'; data: { path: string } }
  // Sent by the server, not surfaced in the UI yet
  | { type: 'BackLinksData'; data: { path: string; back_links: unknown[] } }
  | { type: 'TwoHopLinksData'; data: { path: string; two_hop_links: unknown[] } }
  | { type: 'PinnedFiles'; data: { pinned: string[] } }
