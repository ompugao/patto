import { useEffect, useRef, useState } from 'react';
import { Play, Image as ImageIcon } from 'lucide-react';

interface GooglePhotosBlockProps {
    url: string;
    title: string | null;
}

interface GooglePhotosMedia {
    thumbnail_url: string;
    video_url: string | null;
    title: string | null;
}

// Lookups cost the server a round trip to Google, so share them across every
// mount (virtual scrolling, the hidden print copy, AST updates).
const mediaCache = new Map<string, Promise<GooglePhotosMedia>>();

function fetchMedia(url: string): Promise<GooglePhotosMedia> {
    let pending = mediaCache.get(url);
    if (!pending) {
        pending = fetch(`/api/google-photos-embed?url=${encodeURIComponent(url)}`).then(async (response) => {
            const data = await response.json();
            if (!response.ok) {
                throw new Error(data.error || 'Failed to fetch Google Photos preview');
            }
            return data as GooglePhotosMedia;
        });
        // Let a failed lookup be retried the next time the block mounts.
        pending.catch(() => mediaCache.delete(url));
        mediaCache.set(url, pending);
    }
    return pending;
}

export default function GooglePhotosBlock({ url, title }: GooglePhotosBlockProps) {
    const [media, setMedia] = useState<GooglePhotosMedia | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [nearViewport, setNearViewport] = useState(false);
    const [thumbnailFailed, setThumbnailFailed] = useState(false);
    const containerRef = useRef<HTMLDivElement>(null);

    // Only look up embeds about to scroll into view: a long note can hold dozens,
    // and the browser's six connections per host would queue everything else
    // (images, other embeds) behind them. The hidden print copy never intersects.
    useEffect(() => {
        const el = containerRef.current;
        if (!el || nearViewport) return;
        const observer = new IntersectionObserver(
            (entries) => {
                if (entries.some((entry) => entry.isIntersecting)) {
                    setNearViewport(true);
                }
            },
            { rootMargin: '800px 0px' },
        );
        observer.observe(el);
        return () => observer.disconnect();
    }, [nearViewport]);

    useEffect(() => {
        // Google Photos refuses to be framed, so ask our server to scrape the
        // share page for a thumbnail; clicking opens the share page itself.
        if (!url || !nearViewport) return;
        let cancelled = false;
        setLoading(true);
        setError(null);
        fetchMedia(url)
            .then((data) => {
                if (!cancelled) setMedia(data);
            })
            .catch((err: any) => {
                console.error('Google Photos embed error:', err);
                if (!cancelled) setError(err.message);
            })
            .finally(() => {
                if (!cancelled) setLoading(false);
            });
        return () => {
            cancelled = true;
        };
    }, [url, nearViewport]);

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
