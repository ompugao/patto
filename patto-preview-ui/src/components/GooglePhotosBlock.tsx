import { useState } from 'react';
import { Play, Image as ImageIcon } from 'lucide-react';
import { loadGooglePhotosMedia } from '../embeds';
import { useEmbedData } from '../hooks/useEmbedData';
import { useNearViewport } from '../hooks/useNearViewport';

interface GooglePhotosBlockProps {
    url: string;
    title: string | null;
}

export default function GooglePhotosBlock({ url, title }: GooglePhotosBlockProps) {
    // Only look up embeds about to scroll into view: a long note can hold dozens,
    // and the browser's six connections per host would queue everything else
    // (images, other embeds) behind them. The hidden print copy never intersects.
    const [containerRef, nearViewport] = useNearViewport<HTMLDivElement>();
    // Google Photos refuses to be framed, so ask our server to scrape the
    // share page for a thumbnail; clicking opens the share page itself.
    const { loading, data: media, error } = useEmbedData(url, loadGooglePhotosMedia, nearViewport);
    const [thumbnailFailed, setThumbnailFailed] = useState(false);

    if (loading) {
        return (
            <div ref={containerRef} className="w-full max-w-2xl bg-slate-50 flex items-center justify-center border border-slate-200 rounded-lg p-8 my-4 text-slate-500 shadow-sm animate-pulse aspect-video">
                <a href={url} target="_blank" rel="noopener noreferrer" className="hover:underline">
                    {title || 'Google Photos'}
                </a>
            </div>
        );
    }

    // Offline, unshared or rate-limited: fall back to the plain link the note
    // already holds rather than an alarming error box.
    if (error || !media || thumbnailFailed) {
        return (
            <a
                href={url}
                target="_blank"
                rel="noopener noreferrer"
                title={error ?? undefined}
                className="w-full max-w-2xl my-4 flex items-center gap-3 rounded-lg border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-blue-600 hover:underline shadow-sm"
            >
                <ImageIcon size={18} className="shrink-0 text-slate-400" />
                <span className="truncate">{title || media?.title || url}</span>
            </a>
        );
    }

    const label = title || media.title || 'Google Photos';
    const isVideo = media.video_url !== null;
    return (
        <div
            className="w-full max-w-2xl aspect-video bg-slate-800 rounded-lg flex flex-col items-center justify-center cursor-pointer hover:bg-slate-700 transition my-4 shadow-md group relative overflow-hidden"
            onClick={() => window.open(url, '_blank', 'noopener,noreferrer')}
        >
            <img
                src={media.thumbnail_url}
                referrerPolicy="no-referrer"
                onError={() => setThumbnailFailed(true)}
                alt={label}
                className="absolute inset-0 w-full h-full object-contain"
            />
            <div className="relative z-10 flex flex-col items-center">
                <div className="bg-black/60 text-white rounded-full p-4 mb-2 group-hover:bg-black/80 transition shadow-lg">
                    {isVideo ? <Play size={32} className="ml-1" /> : <ImageIcon size={32} />}
                </div>
                <span className="text-white font-medium bg-black/50 rounded px-2">{label}</span>
            </div>
        </div>
    );
}
