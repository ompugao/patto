import type { FileEntry, FileMetadata } from './protocol'

export const byNewest = (entries: FileEntry[]) => [...entries].sort((a, b) => b.modified - a.modified)

export function fileEntries(paths: string[], metadata: Record<string, FileMetadata>): FileEntry[] {
  return byNewest(paths.map(path => ({ path, modified: metadata[path]?.modified ?? 0 })))
}

export function withModified(files: FileEntry[], path: string, modified: number): FileEntry[] {
  return byNewest(files.map(f => (f.path === path ? { ...f, modified } : f)))
}

export function matchingFiles(files: FileEntry[], query: string): FileEntry[] {
  const lowerQuery = query.toLowerCase()
  return files.filter(f => f.path.toLowerCase().includes(lowerQuery))
}

export function pinnedFirst(files: FileEntry[], pinnedPaths: string[]): FileEntry[] {
  const pinnedSet = new Set(pinnedPaths)
  const pinned = files.filter(f => pinnedSet.has(f.path))
  const rest = files.filter(f => !pinnedSet.has(f.path))
  return [...pinned, ...rest]
}

/** The note a wiki link names, with or without its `.pn` extension. */
export function wikiLinkTarget(files: FileEntry[], link: string): FileEntry | undefined {
  return files.find(f => f.path.replace(/\.pn$/, '') === link || f.path === link || f.path === `${link}.pn`)
}

export const fileName = (path: string) => path.split('/').pop()
