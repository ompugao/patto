// The currently previewed note is kept in the URL as `?note=<repo-relative path>`,
// which is also what `lua/patto_preview.lua` hands to the browser when Neovim
// launches the preview. Keeping it there is what makes Back/Forward work.

export interface HistoryState {
  note: string | null;
  /** Identifies this history entry, so its scroll offset can be found again. */
  key: string;
}

/** The note the current URL points at, or null when there is none. */
export function noteFromLocation(): string | null {
  const note = new URLSearchParams(window.location.search).get('note');
  return note && note.trim() !== '' ? note : null;
}

/**
 * URL for previewing `path`. Paths may contain non-ASCII, so they are encoded,
 * but slashes are left literal to keep the address bar readable and to match
 * what the lua launcher emits.
 */
export function noteUrl(path: string): string {
  return `?note=${encodeURIComponent(path).replace(/%2F/g, '/')}`;
}

/** The state attached to the current history entry, if we put one there. */
export function currentHistoryState(): HistoryState | null {
  const state = window.history.state as Partial<HistoryState> | null;
  return state && typeof state.key === 'string' ? (state as HistoryState) : null;
}

// Entry keys must stay unique across a reload, since history.state survives one
// while the counter does not.
const SESSION = Math.random().toString(36).slice(2, 10);
let entrySeq = 0;
export function nextEntryKey(): string {
  return `${SESSION}-${++entrySeq}`;
}

// Scroll offsets live in sessionStorage rather than in history.state: they are
// written continuously while scrolling, and replaceState at that rate runs into
// browser rate limits. sessionStorage is per-tab and survives a reload, which is
// exactly the lifetime a history entry has.
const SCROLL_PREFIX = 'patto-preview:scroll:';

export function saveScrollForEntry(key: string, offset: number): void {
  try {
    window.sessionStorage.setItem(SCROLL_PREFIX + key, String(Math.round(offset)));
  } catch {
    // Private browsing or a full quota — scroll restoration is not worth failing over
  }
}

export function scrollForEntry(key: string | null): number {
  if (!key) return 0;
  try {
    const raw = window.sessionStorage.getItem(SCROLL_PREFIX + key);
    const offset = raw === null ? 0 : Number(raw);
    return Number.isFinite(offset) && offset > 0 ? offset : 0;
  } catch {
    return 0;
  }
}
