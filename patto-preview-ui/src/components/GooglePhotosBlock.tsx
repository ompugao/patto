import { useEffect, useState } from 'react';
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

export default function GooglePhotosBlock({ url, title }: GooglePhotosBlockProps) {
    const [media, setMedia] = useState<GooglePhotosMedia | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        // Google Photos refuses to be framed, so ask our server to scrape the
        // share page for a thumbnail; clicking opens the share page itself.
        const fetchMedia = async () => {
            try {
                setLoading(true);
                setError(null);

                const response = await fetch(`/api/google-photos-embed?url=${encodeURIComponent(url)}`);
                const data = await response.json();
                if (!response.ok) {
                    throw new Error(data.error || 'Failed to fetch Google Photos preview');
                }
                setMedia(data);
            } catch (err: any) {
                console.error('Google Photos embed error:', err);
                setError(err.message);
            } finally {
                setLoading(false);
            }
        };

        if (url) {
            fetchMedia();
        }
    }, [url]);

    if (loading) {
        return (
            <div className="w-full max-w-2xl bg-slate-50 flex items-center justify-center border border-slate-200 rounded-lg p-8 my-4 text-slate-500 shadow-sm animate-pulse aspect-video">
                Loading Google Photos preview...
            </div>
        );
    }

    if (error || !media) {
        return (
            <div className="w-full max-w-2xl bg-red-50 text-red-700 border border-red-200 rounded-lg p-6 my-4 text-center flex flex-col justify-center items-center shadow-sm">
                <span className="font-semibold mb-2">Error loading Google Photos preview</span>
                <span className="text-sm opacity-80 mb-4">{error}</span>
                <a href={url} target="_blank" rel="noopener noreferrer" className="text-blue-600 hover:underline text-sm font-medium">
                    View on Google Photos
                </a>
            </div>
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
