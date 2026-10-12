import { useCallback, useMemo, useState, type ChangeEvent, type KeyboardEvent } from 'react'
import type { FileEntry } from '../protocol'
import { matchingFiles, pinnedFirst } from '../fileList'

/** The fuzzy-find box over the sidebar list, with keyboard highlighting. */
export function useFileSearch(files: FileEntry[], pinnedFiles: string[], onPick: (path: string) => void) {
  const [query, setQuery] = useState('')
  const [highlightedIndex, setHighlightedIndex] = useState(-1)

  // When no filter is active, show pinned files at the top
  const displayFiles = useMemo(() => {
    if (query.trim()) return matchingFiles(files, query)
    return pinnedFirst(files, pinnedFiles)
  }, [files, pinnedFiles, query])

  // Reset highlight when query changes
  const onQueryChange = useCallback((e: ChangeEvent<HTMLInputElement>) => {
    setQuery(e.target.value)
    setHighlightedIndex(-1)
  }, [])

  const onKeyDown = useCallback((e: KeyboardEvent<HTMLInputElement>) => {
    const len = displayFiles.length
    if (len === 0) return
    if ((e.key === 'Tab' && e.shiftKey) || e.key === 'ArrowUp') {
      e.preventDefault()
      setHighlightedIndex(i => (i - 1 + len) % len)
    } else if (e.key === 'Tab' || e.key === 'ArrowDown') {
      e.preventDefault()
      setHighlightedIndex(i => (i + 1) % len)
    } else if (e.key === 'Enter') {
      const idx = highlightedIndex >= 0 ? highlightedIndex : 0
      onPick(displayFiles[idx].path)
    }
  }, [displayFiles, highlightedIndex, onPick])

  return { query, highlightedIndex, displayFiles, onQueryChange, onKeyDown }
}
