import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react'
import type { ServerMessage } from '../protocol'

interface Options {
  /**
   * `isOwnReply` marks the FileChanged that answers our own selectNote. The
   * server sends the same message when any file changes on disk, so only the
   * queue of requests we sent can tell the two apart.
   */
  onMessage: (msg: ServerMessage, isOwnReply: boolean) => void;
}

/** The WebSocket to patto-preview, reconnecting on its own when it drops. */
export function usePreviewSocket({ onMessage }: Options) {
  const [isConnected, setIsConnected] = useState(false)
  const socketRef = useRef<WebSocket | null>(null)
  // Paths we asked for and have not seen answered yet
  const pendingRef = useRef<string[]>([])

  // The socket is built once, so the handler reaches the caller through a ref
  const onMessageRef = useRef(onMessage)
  useLayoutEffect(() => { onMessageRef.current = onMessage })

  // Splicing off everything up to the match also drops requests that were never
  // answered (an unreadable file), so the queue cannot drift out of step.
  const claimReply = useCallback((msg: ServerMessage) => {
    if (msg.type !== 'FileChanged') return false
    const at = pendingRef.current.indexOf(msg.data.path)
    if (at === -1) return false
    pendingRef.current.splice(0, at + 1)
    return true
  }, [])

  const send = useCallback((message: unknown) => {
    const socket = socketRef.current
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      console.warn('[patto] WebSocket not open, state:', socket?.readyState)
      return false
    }
    socket.send(JSON.stringify(message))
    return true
  }, [])

  /** Ask the server to render a note; the answer arrives as a FileChanged. */
  const selectNote = useCallback((path: string) => {
    if (send({ type: 'SelectFile', data: { path } })) {
      pendingRef.current.push(path)
      console.log('[patto] SelectFile sent:', path)
    }
  }, [send])

  const setPinned = useCallback((path: string, pinned: boolean) => {
    send({ type: pinned ? 'PinFile' : 'UnpinFile', data: { path } })
  }, [send])

  useEffect(() => {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    // Use the port we were served from, so `--port` works; in `vite dev` the
    // dev-server proxy (see vite.config.ts) forwards /ws to the backend.
    const wsUrl = `${protocol}//${window.location.host}/ws`

    // A superseded socket — StrictMode's double-mount, or a reconnect that has
    // already landed — must not touch the live one.
    const isCurrent = (socket: WebSocket) => socketRef.current === socket

    const connect = () => {
      const socket = new WebSocket(wsUrl)
      socketRef.current = socket

      socket.onopen = () => {
        if (!isCurrent(socket)) return
        console.log('[patto] WebSocket connected to', wsUrl)
        setIsConnected(true)
      }

      socket.onmessage = event => {
        if (!isCurrent(socket)) return
        let msg: ServerMessage
        try {
          msg = JSON.parse(event.data)
        } catch (e) {
          console.error('[patto] Failed to parse websocket message', e, event.data)
          return
        }
        console.log('[patto] msg:', msg.type)
        onMessageRef.current(msg, claimReply(msg))
      }

      socket.onclose = () => {
        if (!isCurrent(socket)) return
        console.log('[patto] WebSocket closed, reconnecting in 2s...')
        setIsConnected(false)
        pendingRef.current = []
        socketRef.current = null
        setTimeout(connect, 2000)
      }

      socket.onerror = err => console.error('[patto] WebSocket error:', err)
    }

    connect()
    return () => { socketRef.current?.close() }
  }, [claimReply])

  return { isConnected, selectNote, setPinned }
}
