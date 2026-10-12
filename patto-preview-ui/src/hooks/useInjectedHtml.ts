import { useEffect, useRef } from 'react'

/**
 * Puts provider HTML into the returned container. Scripts are recreated
 * because innerHTML never executes them, and oEmbed snippets rely on theirs.
 */
export function useInjectedHtml(html: string | null) {
  const containerRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const container = containerRef.current
    if (!html || !container) return
    container.innerHTML = html
    container.querySelectorAll('script').forEach(script => {
      const runnable = document.createElement('script')
      if (script.src) {
        runnable.src = script.src
        runnable.async = script.async
      } else {
        runnable.textContent = script.textContent
      }
      Array.from(script.attributes).forEach(attr => {
        if (attr.name !== 'src') {
          runnable.setAttribute(attr.name, attr.value)
        }
      })
      script.parentNode?.replaceChild(runnable, script)
    })
  }, [html])

  return containerRef
}
