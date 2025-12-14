'use client';

import { useEffect, useRef, useCallback, useState } from 'react';

/**
 * Connection states for WebSocket
 */
export const ConnectionState = {
  CONNECTING: 'connecting',
  CONNECTED: 'connected',
  DISCONNECTED: 'disconnected',
  RECONNECTING: 'reconnecting',
};

/**
 * Custom hook for WebSocket connection to patto-preview server.
 * Includes automatic reconnection with exponential backoff.
 * 
 * @param {Function} onMessage - Callback for incoming messages
 * @param {Object} options - Configuration options
 * @param {boolean} options.autoReconnect - Whether to auto-reconnect (default: true)
 * @param {number} options.maxRetries - Maximum reconnection attempts (default: 5)
 * @returns {{
 *   sendMessage: Function,
 *   connectionState: string,
 *   reconnect: Function
 * }}
 */
export function usePattoWebSocket(onMessage, options = {}) {
  const { autoReconnect = true, maxRetries = 5 } = options;

  const wsRef = useRef(null);
  const onMessageRef = useRef(onMessage);
  const retryCountRef = useRef(0);
  const retryTimeoutRef = useRef(null);

  const [connectionState, setConnectionState] = useState(ConnectionState.DISCONNECTED);

  // Update message handler ref
  useEffect(() => {
    onMessageRef.current = onMessage;
  }, [onMessage]);

  /**
   * Calculate exponential backoff delay
   * @param {number} attempt - Current retry attempt
   * @returns {number} Delay in milliseconds
   */
  const getBackoffDelay = useCallback((attempt) => {
    // Exponential backoff: 1s, 2s, 4s, 8s, 16s (capped)
    return Math.min(1000 * Math.pow(2, attempt), 16000);
  }, []);

  /**
   * Create and connect WebSocket
   */
  const connect = useCallback(() => {
    if (typeof window === 'undefined') return;

    // Clean up existing connection
    if (wsRef.current) {
      wsRef.current.close();
    }

    setConnectionState(
      retryCountRef.current > 0
        ? ConnectionState.RECONNECTING
        : ConnectionState.CONNECTING
    );

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/ws`;

    const ws = new WebSocket(wsUrl);
    wsRef.current = ws;

    ws.onopen = () => {
      console.log('WebSocket connected');
      setConnectionState(ConnectionState.CONNECTED);
      retryCountRef.current = 0;
    };

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        onMessageRef.current?.(data);
      } catch (error) {
        console.error('Error parsing WebSocket message:', error);
      }
    };

    ws.onerror = (error) => {
      console.error('WebSocket error:', error);
    };

    ws.onclose = (event) => {
      console.log('WebSocket disconnected', event.code, event.reason);
      setConnectionState(ConnectionState.DISCONNECTED);

      // Attempt reconnection if enabled and not a clean close
      if (autoReconnect && event.code !== 1000 && retryCountRef.current < maxRetries) {
        const delay = getBackoffDelay(retryCountRef.current);
        console.log(`Reconnecting in ${delay}ms (attempt ${retryCountRef.current + 1}/${maxRetries})`);

        retryTimeoutRef.current = setTimeout(() => {
          retryCountRef.current++;
          connect();
        }, delay);
      }
    };
  }, [autoReconnect, maxRetries, getBackoffDelay]);

  // Initial connection
  useEffect(() => {
    connect();

    return () => {
      if (retryTimeoutRef.current) {
        clearTimeout(retryTimeoutRef.current);
      }
      if (wsRef.current) {
        wsRef.current.close(1000, 'Component unmounting');
      }
    };
  }, [connect]);

  /**
   * Send a message through the WebSocket
   * @param {Object} message - Message to send
   */
  const sendMessage = useCallback((message) => {
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(message));
    } else {
      console.warn('Cannot send message: WebSocket not connected');
    }
  }, []);

  /**
   * Manually trigger reconnection
   */
  const reconnect = useCallback(() => {
    retryCountRef.current = 0;
    connect();
  }, [connect]);

  return { sendMessage, connectionState, reconnect };
}
