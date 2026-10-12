import { useCallback, useState } from 'react'
import type { FileEntry, ServerMessage } from '../protocol'
import { byNewest, fileEntries, withModified } from '../fileList'

/** The sidebar's view of the workspace, kept current from server messages. */
export function useFileList() {
  const [files, setFiles] = useState<FileEntry[]>([])
  const [pinnedFiles, setPinnedFiles] = useState<string[]>([])

  const applyFileMessage = useCallback((msg: ServerMessage) => {
    switch (msg.type) {
      case 'FileList':
        setFiles(fileEntries(msg.data.files ?? [], msg.data.metadata ?? {}))
        break

      case 'FileChanged': {
        const { path, metadata } = msg.data
        if (metadata) setFiles(prev => withModified(prev, path, metadata.modified))
        break
      }

      case 'FileAdded':
        if (msg.data.path && msg.data.metadata) {
          setFiles(prev => byNewest([...prev, { path: msg.data.path, modified: msg.data.metadata.modified }]))
        }
        break

      case 'FileRemoved':
        if (msg.data.path) setFiles(prev => prev.filter(f => f.path !== msg.data.path))
        break

      case 'PinnedFiles':
        setPinnedFiles(msg.data.pinned ?? [])
        break
    }
  }, [])

  return { files, pinnedFiles, applyFileMessage }
}
