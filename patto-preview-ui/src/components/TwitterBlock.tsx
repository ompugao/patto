import { useState, useEffect, useRef } from 'react';

interface TwitterBlockProps {
    url: string;
    title: string | null;
}

declare global {
    interface Window {
        twttr?: {
            _e: Array<(t: Window['twttr']) => void>;
            ready: (f: (t: Window['twttr']) => void) => void;
            widgets: {
                load: (element?: HTMLElement) => void;
            };
        };
    }
}

function loadTwitterScript(): Promise<NonNullable<Window['twttr']>> {
    return new Promise((resolve) => {
        // Set up the twttr stub with a ready queue if not already present
        if (!window.twttr) {
            window.twttr = {
                _e: [],
                ready(f) { this._e.push(f); },
                widgets: { load: () => {} },
            };
        }

        window.twttr.ready((twttr) => resolve(twttr!));

        if (!document.getElementById('twitter-wjs')) {
            const script = document.createElement('script');
            script.id = 'twitter-wjs';
            script.src = 'https://platform.twitter.com/widgets.js';
            script.async = true;
            script.charset = 'utf-8';
            document.head.appendChild(script);
        }
    });
}

export default function TwitterBlock({ url, title }: TwitterBlockProps) {
    const [embedHtml, setEmbedHtml] = useState<string | null>(null);
    const [error, setError] = useState(false);
    const containerRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        const apiUrl = `/api/twitter-embed?url=${encodeURIComponent(url)}`;
        fetch(apiUrl)
            .then(r => r.json())
            .then(data => {
                if (data.html) {
                    setEmbedHtml(data.html);
                } else {
                    setError(true);
                }
            })
            .catch(() => setError(true));
    }, [url]);

    useEffect(() => {
        if (embedHtml && containerRef.current) {
            const el = containerRef.current;
            loadTwitterScript().then((twttr) => {
                twttr.widgets.load(el);
            });
        }
    }, [embedHtml]);

    if (error) {
        return <a href={url} className="text-blue-500 underline">{title || url}</a>;
    }

    if (!embedHtml) {
        return <div className="text-slate-400 text-sm my-4">Loading tweet…</div>;
    }

    return (
        <div ref={containerRef} dangerouslySetInnerHTML={{ __html: embedHtml }} className="my-4" />
    );
}
