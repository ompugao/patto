import { useCallback, useEffect, useRef, useState } from 'react'
import type { AstNode } from '../ast'
import type { ServerMessage } from '../protocol'
import { wikiLinkTarget } from '../fileList'
import { noteFromLocation, useNoteHistory } from './useNoteHistory'
import { usePreviewSocket } from './usePreviewSocket'
import { useFileList } from './useFileList'

/** The note being previewed: which one, its rendered AST, and how to move to another. */
export function useNotePreview() {
  const [ast, setAst] = useState<AstNode | null>(null)
  // Seeded from `?note=` so a deep link shows "Loading..." rather than the empty state
  const [selectedFile, setSelectedFile] = useState<string | null>(() => noteFromLocation())
  const { files, pinnedFiles, applyFileMessage } = useFileList()

  // Socket callbacks can run before React has re-rendered, so they read the
  // selection from a ref that moves in the same tick as the state does.
  const selectedFileRef = useRef<string | null>(selectedFile)
  const setSelected = useCallback((path: string | null) => {
    selectedFileRef.current = path
    setSelectedFile(path)
  }, [])

  // Show a note without touching history — for Back/Forward, which has already
  // moved it, and for the empty state when `note` is null.
  const showNote = useCallback((note: string | null) => {
    setSelected(note)
    setAst(null) // Clear while loading
  }, [setSelected])

  const { scrollRef, pushNote, replaceNote, clearNote } = useNoteHistory({
    content: ast,
    onNavigate: showNote,
  })

  const handleMessage = useCallback((msg: ServerMessage, isOwnReply: boolean) => {
    applyFileMessage(msg)
    if (msg.type === 'FileList') {
      // A `?note=` naming a file that is not in the workspace would otherwise
      // sit on "Loading..." forever, since the server answers it with nothing.
      const current = selectedFileRef.current
      if (current && !(msg.data.files ?? []).includes(current)) {
        console.warn('[patto] note from URL not found in workspace:', current)
        showNote(null)
        clearNote()
      }
    } else if (msg.type === 'FileChanged') {
      const { path, ast: changed } = msg.data
      if (isOwnReply) {
        // Show it only if it is still what we are on: pressing Back before it
        // arrived means we have moved on since asking.
        if (path === selectedFileRef.current) setAst(changed ?? null)
      } else {
        // A broadcast — some file changed on disk, including Neovim buffers
        // arriving over the LSP bridge — and it takes over the view. Keep the
        // URL honest about that, but replace rather than push: a `git pull`
        // touching many files would otherwise shred the back stack.
        if (path !== selectedFileRef.current) {
          setSelected(path)
          replaceNote(path)
        }
        setAst(changed ?? null)
      }
    }
  }, [applyFileMessage, clearNote, replaceNote, setSelected, showNote])

  const { isConnected, selectNote, setPinned } = usePreviewSocket({ onMessage: handleMessage })

  // Ask the server for whatever we are meant to be showing. Running again on
  // reconnect is what brings the view back when the server restarts, and it also
  // covers the `?note=` we were opened with, which is selected before we connect.
  useEffect(() => {
    if (isConnected && selectedFile) selectNote(selectedFile)
  }, [isConnected, selectedFile, selectNote])

  // User-intent navigation: a sidebar click, fuzzy-find Enter, or a wiki link.
  const selectFile = useCallback((path: string) => {
    // Re-selecting the open note only makes sure the URL agrees; re-showing it
    // would blank the view with nothing on the way to replace it.
    if (path === selectedFileRef.current) {
      replaceNote(path)
      return
    }
    pushNote(path)
    showNote(path)
  }, [pushNote, replaceNote, showNote])

  const openWikiLink = useCallback((link: string) => {
    const target = wikiLinkTarget(files, link)
    if (target) selectFile(target.path)
  }, [files, selectFile])

  const togglePin = useCallback((path: string) => {
    setPinned(path, !pinnedFiles.includes(path))
  }, [pinnedFiles, setPinned])

  return { ast, files, pinnedFiles, selectedFile, isConnected, scrollRef, selectFile, openWikiLink, togglePin }
}
