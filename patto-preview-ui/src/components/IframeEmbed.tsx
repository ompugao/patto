import { useState } from 'react';
import { Play } from 'lucide-react';
import { iframeEmbedUrl, isYoutubeLink } from '../embeds';

interface IframeEmbedProps {
    link: string;
    title: string | null;
}

/** A generic iframe embed that loads only when clicked, with a YouTube poster when it is one. */
export default function IframeEmbed({ link, title }: IframeEmbedProps) {
    const [isLoaded, setIsLoaded] = useState(false);
    const iframeSrc = iframeEmbedUrl(link);
    const isYoutube = isYoutubeLink(link);

    if (!isLoaded) {
        return (
            <div
                className="w-full max-w-2xl aspect-video bg-slate-800 rounded-lg flex flex-col items-center justify-center cursor-pointer hover:bg-slate-700 transition my-4 shadow-md group relative overflow-hidden"
                onClick={() => setIsLoaded(true)}
            >
                {isYoutube && (
                    <div className="absolute inset-0 opacity-20 bg-cover bg-center" style={{ backgroundImage: `url(https://img.youtube.com/vi/${iframeSrc.split('/').pop()}/maxresdefault.jpg)` }}></div>
                )}
                <div className="relative z-10 flex flex-col items-center">
                    <div className="bg-red-600 text-white rounded-full p-4 mb-2 group-hover:bg-red-500 transition shadow-lg">
                        <Play size={32} className="ml-1" />
                    </div>
                    <span className="text-slate-200 font-medium">
                        Click to load {title || (isYoutube ? 'YouTube Video' : 'Embed')}
                    </span>
                    <span className="text-slate-400 text-xs mt-1 max-w-sm truncate px-4">{link}</span>
                </div>
            </div>
        );
    }

    return (
        <div className="w-full max-w-2xl my-4 rounded-lg overflow-hidden shadow-lg border border-slate-200 bg-slate-50 relative aspect-video">
            <iframe
                src={iframeSrc}
                className="absolute inset-0 w-full h-full"
                allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
                allowFullScreen
                title={title || "Embedded content"}
            ></iframe>
        </div>
    );
}
