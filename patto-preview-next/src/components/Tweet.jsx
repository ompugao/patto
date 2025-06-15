import React, { useEffect, useRef } from 'react';

let twitterScriptLoaded = false;

export default function Tweet({ id }) {
  const ref = useRef(null);

  useEffect(() => {
    const loadTwitterScript = () => {
      if (!twitterScriptLoaded && typeof window !== 'undefined') {
        const script = document.createElement('script');
        script.src = 'https://platform.twitter.com/widgets.js';
        script.async = true;
        script.onload = () => {
          twitterScriptLoaded = true;
          loadWidget();
        };
        document.head.appendChild(script);
      } else {
        loadWidget();
      }
    };

    const loadWidget = () => {
      if (window.twttr && window.twttr.widgets && ref.current) {
        // Don't clear innerHTML, just load widgets on existing content
        // @ts-expect-error
        window.twttr.widgets.load(ref.current);
      } else if (twitterScriptLoaded) {
        // Retry if Twitter script is loaded but widgets not ready
        setTimeout(loadWidget, 100);
      }
    };

    loadTwitterScript();
  }, [id]);

  return (
    <div ref={ref}>
      <blockquote className="twitter-tweet">
        <a href={`https://twitter.com/x/status/${id}`}></a>
      </blockquote>
    </div>
  );
};

export function extractTwitterId(url) {
  const parts = url.split('/status/');
  if (parts.length > 1) {
    let potentialId = parts[1];
    // Remove any trailing characters like '?' or '/'
    const tweetId = potentialId.split('?')[0].split('/')[0];
    return tweetId;
  }
  return null;
}
