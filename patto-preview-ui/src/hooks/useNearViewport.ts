import { useEffect, useRef, useState } from 'react'

/**
 * Whether the returned ref's element has come within `rootMargin` of the
 * viewport. Stays true once seen; a detached ref (hidden print copy) never
 * intersects.
 */
export function useNearViewport<T extends Element>(rootMargin = '800px 0px') {
  const ref = useRef<T>(null)
  const [near, setNear] = useState(false)

  useEffect(() => {
    const el = ref.current
    if (!el || near) return
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setNear(true)
        }
      },
      { rootMargin },
    )
    observer.observe(el)
    return () => observer.disconnect()
  }, [near, rootMargin])

  return [ref, near] as const
}
