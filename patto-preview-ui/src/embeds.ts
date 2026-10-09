// How each kind of embedded link is resolved. The /api/* routes are served by
// patto-preview, which proxies the providers' oEmbed endpoints.

export interface OEmbedService {
    name: string;
    load: (url: string) => Promise<string>;
    containerClass: string;
}

function oEmbedLoader(endpoint: string, name: string) {
    return async (url: string): Promise<string> => {
        const response = await fetch(`${endpoint}?url=${encodeURIComponent(url)}`);
        if (!response.ok) {
            throw new Error(`Failed to fetch ${name} embed`);
        }
        const data = await response.json();
        if (!data.html) {
            throw new Error('No embed HTML received');
        }
        return data.html as string;
    };
}

export const SPEAKER_DECK: OEmbedService = {
    name: 'SpeakerDeck',
    load: oEmbedLoader('/api/speakerdeck-embed', 'SpeakerDeck'),
    containerClass: 'w-full max-w-2xl my-4 rounded-lg overflow-hidden shadow-lg border border-slate-200 bg-white relative',
};

export const SLIDE_SHARE: OEmbedService = {
    name: 'SlideShare',
    load: oEmbedLoader('/api/slideshare-embed', 'SlideShare'),
    containerClass: 'w-full max-w-2xl my-4 rounded-lg overflow-hidden relative',
};

export const loadTweetHtml = oEmbedLoader('/api/twitter-embed', 'Twitter');

export interface GooglePhotosMedia {
    thumbnail_url: string;
    video_url: string | null;
    title: string | null;
}

// Lookups cost the server a round trip to Google, so share them across every
// mount (virtual scrolling, the hidden print copy, AST updates).
const mediaCache = new Map<string, Promise<GooglePhotosMedia>>();

export function loadGooglePhotosMedia(url: string): Promise<GooglePhotosMedia> {
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

export function isGooglePhotosLink(link: string): boolean {
    return link.includes('photos.app.goo.gl') || link.includes('photos.google.com') || link.includes('goo.gl/photos');
}

export function isYoutubeLink(link: string): boolean {
    return link.includes('youtube.com') || link.includes('youtu.be');
}

/** The iframe URL for a YouTube watch/share link; other links pass through. */
export function iframeEmbedUrl(link: string): string {
    if (!isYoutubeLink(link) || link.includes('embed')) return link;
    try {
        const url = new URL(link);
        const videoId = url.searchParams.get('v') || url.pathname.split('/').pop();
        return videoId ? `https://www.youtube.com/embed/${videoId}` : link;
    } catch {
        return link;
    }
}

/** Local PDFs are served by the preview server; absolute URLs are framed as-is. */
export function pdfSrc(link: string): string {
    return link.includes('://') ? link : `/api/files/${link}`;
}
