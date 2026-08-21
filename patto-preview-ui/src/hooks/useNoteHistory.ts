import { useCallback, useEffect, useLayoutEffect, useRef } from 'react'

// The previewed note lives in the URL as `?note=<repo-relative path>` — which is
// also what lua/patto_preview.lua hands the browser when Neovim launches the
// preview. Keeping it there is what makes Back/Forward work.

interface NoteHistoryState {
  note: string | null;
  /** Identifies this history entry, so its scroll offset can be found again. */
  key: string;
}

/** The note the current URL points at, or null when there is none. */
export function noteFromLocation(): string | null {
  const note = new URLSearchParams(window.location.search).get('note')
  return note && note.trim() !== '' ? note : null
}

/**
 * URL for previewing `path`. Paths may contain non-ASCII so they are encoded,
 * but slashes stay literal to keep the address bar readable and to match what
 * the lua launcher emits.
 */
function noteUrl(path: string): string {
  return `?note=${encodeURIComponent(path).replace(/%2F/g, '/')}`
}

/** The state on the entry we are on, if we are the ones who wrote it. */
function entryState(): NoteHistoryState | null {
  const state = window.history.state as Partial<NoteHistoryState> | null
  return state && typeof state.key === 'string' ? (state as NoteHistoryState) : null
}

// Entry keys stay unique across a reload, since history.state survives one while
// the counter does not.
const SESSION = Math.random().toString(36).slice(2, 10)
let entrySeq = 0
const nextEntryKey = () => `${SESSION}-${++entrySeq}`

// Scroll offsets live in sessionStorage rather than in history.state: they are
// written continuously while scrolling, and replaceState at that rate runs into
// browser rate limits. sessionStorage is per-tab and survives a reload, which is
// exactly the lifetime of a history entry.
const SCROLL_PREFIX = 'patto-preview:scroll:'

function storeScroll(key: string, offset: number): void {
  try {
    window.sessionStorage.setItem(SCROLL_PREFIX + key, String(Math.round(offset)))
  } catch {
    // Private browsing or a full quota — not worth failing a navigation over
  }
}

function readScroll(key: string | null): number {
  if (!key) return 0
  try {
    const offset = Number(window.sessionStorage.getItem(SCROLL_PREFIX + key))
    return Number.isFinite(offset) && offset > 0 ? offset : 0
  } catch {
    return 0
  }
}

// How long to keep re-applying a restored offset. The virtualizer measures rows
// lazily, so the scroll container keeps growing for a few frames after the
// content lands, and an early write gets clamped.
const RESTORE_FRAMES = 20

interface Options {
  /** Whatever is currently rendered; the scroll effects re-run when it changes. */
  content: unknown;
  /** Back/Forward wants this note shown, or the empty state when null. */
  onNavigate: (note: string | null) => void;
}

/**
 * Keeps the previewed note in the URL and the scroll offset with the history
 * entry it belongs to, so the browser's Back/Forward buttons work.
 */
export function useNoteHistory({ content, onNavigate }: Options) {
  /** Attach to the scroll container whose offset should be remembered. */
  const scrollRef = useRef<HTMLDivElement | null>(null)
  const entryKeyRef = useRef<string | null>(null)
  // An offset a Back/Forward asked for, pending until the content renders
  const pendingScrollRef = useRef<number | null>(null)

  const onNavigateRef = useRef(onNavigate)
  useLayoutEffect(() => { onNavigateRef.current = onNavigate })

  // `push` starts a new entry; otherwise the entry we are on is corrected in
  // place. Leaving `url` out keeps the current one. Every entry gets a key.
  const write = useCallback((note: string | null, url?: string, push = false) => {
    const key = push ? nextEntryKey() : entryState()?.key ?? nextEntryKey()
    const state: NoteHistoryState = { note, key }
    if (push) window.history.pushState(state, '', url)
    else window.history.replaceState(state, '', url)
    entryKeyRef.current = key
  }, [])

  /** Record a note the user navigated to, as a new entry. */
  const pushNote = useCallback((note: string) => write(note, noteUrl(note), true), [write])
  /** Point the entry we are on at a different note, without adding to the stack. */
  const replaceNote = useCallback((note: string) => write(note, noteUrl(note)), [write])
  /** Drop the note from the URL, leaving the empty state. */
  const clearNote = useCallback(() => write(null, window.location.pathname), [write])

  // Seed a key on the entry we landed on, so scrolling has somewhere to record
  // to, and pick up an offset a reload left behind.
  useEffect(() => {
    const existing = entryState()
    if (existing) entryKeyRef.current = existing.key
    else write(noteFromLocation())
    pendingScrollRef.current = readScroll(entryKeyRef.current)
  }, [write])

  // Back/Forward, including mouse side-buttons and Alt+Left/Right. Never pushes:
  // StrictMode double-invokes effects in dev, and this must not invent entries.
  useEffect(() => {
    const onPopState = (e: PopStateEvent) => {
      const state = e.state as NoteHistoryState | null
      entryKeyRef.current = state?.key ?? null
      const note = state?.note ?? noteFromLocation()
      pendingScrollRef.current = note ? readScroll(entryKeyRef.current) : null
      onNavigateRef.current(note)
    }
    window.addEventListener('popstate', onPopState)
    return () => window.removeEventListener('popstate', onPopState)
  }, [])

  // Record the offset continuously against the current entry, so it is already
  // saved however the user leaves — a click, Back, Forward, or a reload. Saving
  // on the way out would miss Back, which fires after the entry has changed.
  useEffect(() => {
    const el = scrollRef.current
    if (!el) return
    let raf = 0
    const onScroll = () => {
      if (raf) return
      raf = requestAnimationFrame(() => {
        raf = 0
        // Skip the frames while a restore is still settling, or we would record
        // the offset we are on our way *from*.
        if (pendingScrollRef.current !== null) return
        if (entryKeyRef.current) storeScroll(entryKeyRef.current, el.scrollTop)
      })
    }
    el.addEventListener('scroll', onScroll, { passive: true })
    return () => {
      el.removeEventListener('scroll', onScroll)
      if (raf) cancelAnimationFrame(raf)
    }
  }, [content])

  // Apply the offset the entry carries, once its content is up.
  useEffect(() => {
    const target = pendingScrollRef.current
    if (content == null || target === null) return
    if (target === 0) {
      pendingScrollRef.current = null
      return
    }
    let frames = 0
    let raf = requestAnimationFrame(function step() {
      const el = scrollRef.current
      if (el) {
        el.scrollTop = target
        if (Math.abs(el.scrollTop - target) < 1) {
          pendingScrollRef.current = null
          return
        }
      }
      if (++frames < RESTORE_FRAMES) raf = requestAnimationFrame(step)
      else pendingScrollRef.current = null
    })
    return () => cancelAnimationFrame(raf)
  }, [content])

  return { scrollRef, pushNote, replaceNote, clearNote }
}
