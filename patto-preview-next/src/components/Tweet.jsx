import React, { useEffect, useRef } from 'react';

export default function Tweet({ id }) {
  const ref = useRef(null);

  useEffect(() => {
    // @ts-expect-error
    window.twttr?.widgets.load(ref.current);
  }, [id]);

  return (
    <div
      dangerouslySetInnerHTML={{ __html: generateEmbedHtml(id) }}
      ref={ref}
    />
  );
};

function generateEmbedHtml(id) {
  if (!/^\d+$/u.test(id)) {
    throw new Error(`Invalid tweet ID: ${id}`);
  }

  return `<blockquote class="twitter-tweet"><a href="https://twitter.com/x/status/${id}"></a></blockquote>`;
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
