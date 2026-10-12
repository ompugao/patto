import { useEffect, useState } from 'react'

interface Settled<T> {
  url: string;
  data: T | null;
  error: string | null;
}

function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err)
}

/**
 * Loads whatever an embed needs for `url` through `load`, which must be a
 * stable function. Reports loading until the result for the current url is
 * in, so a url change does not briefly show the previous embed.
 */
export function useEmbedData<T>(url: string, load: (url: string) => Promise<T>, enabled = true) {
  const [settled, setSettled] = useState<Settled<T> | null>(null)

  useEffect(() => {
    if (!url || !enabled) return
    let cancelled = false
    load(url)
      .then(data => {
        if (!cancelled) setSettled({ url, data, error: null })
      })
      .catch((err: unknown) => {
        console.error('[patto] embed error:', url, err)
        if (!cancelled) setSettled({ url, data: null, error: errorMessage(err) })
      })
    return () => {
      cancelled = true
    }
  }, [url, load, enabled])

  const current = settled?.url === url ? settled : null
  return {
    loading: current === null,
    data: current?.data ?? null,
    error: current?.error ?? null,
  }
}
