import { useEffect, useRef } from 'react';
import { loadTweetHtml } from '../embeds';
import { useEmbedData } from '../hooks/useEmbedData';
import { loadTwitterWidgets } from '../twitterWidgets';

interface TwitterBlockProps {
    url: string;
    title: string | null;
}

export default function TwitterBlock({ url, title }: TwitterBlockProps) {
    const { loading, data: embedHtml, error } = useEmbedData(url, loadTweetHtml);
    const containerRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        const el = containerRef.current;
        if (!embedHtml || !el) return;
        loadTwitterWidgets().then((twttr) => {
            twttr.widgets.load(el);
        });
    }, [embedHtml]);

    if (error) {
        return <a href={url} className="text-blue-500 underline">{title || url}</a>;
    }

    if (loading || !embedHtml) {
        return <div className="text-slate-400 text-sm my-4">Loading tweet…</div>;
    }

    return (
        <div ref={containerRef} dangerouslySetInnerHTML={{ __html: embedHtml }} className="my-4" />
    );
}
