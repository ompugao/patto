'use client';

import { useEffect, useRef, useState } from 'react';
import parse from 'html-react-parser';

export default function LazyCode({ code, language }) {
  const ref = useRef(null);
  const [highlighted, setHighlighted] = useState(false);
  const [highlightedHtml, setHighlightedHtml] = useState('');

  useEffect(() => {
    // Reset highlighted state when code changes
    setHighlighted(false);
    setHighlightedHtml('');
  }, [code, language]);

  useEffect(() => {
    if (!ref.current) return;

    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting && !highlighted) {
          // Lazy load highlight.js
          import('highlight.js').then((hljs) => {
            const result = hljs.default.highlightAuto(code, language ? [language] : undefined);
            setHighlightedHtml(result.value);
            setHighlighted(true);
          });
        }
      },
      {
        rootMargin: '50px', // Start loading when code block is 50px away from viewport
        threshold: 0
      }
    );

    observer.observe(ref.current);

    return () => {
      if (ref.current) {
        observer.unobserve(ref.current);
      }
    };
  }, [code, language, highlighted]);

  return (
    <code ref={ref} className="hljs">
      {highlighted ? parse(highlightedHtml) : code}
    </code>
  );
}