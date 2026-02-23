import { useEffect, useRef, useState } from 'react';

interface SpeakerDeckBlockProps {
    url: string;
}

export default function SpeakerDeckBlock({ url }: SpeakerDeckBlockProps) {
    const [embedHtml, setEmbedHtml] = useState<string | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const containerRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        // Fetch from our API proxy
        const fetchEmbed = async () => {
            try {
                setLoading(true);
                setError(null);

                const response = await fetch(`/api/speakerdeck-embed?url=${encodeURIComponent(url)}`);

                if (!response.ok) {
                    throw new Error('Failed to fetch SpeakerDeck embed');
                }

                const data = await response.json();

                if (data.html) {
                    setEmbedHtml(data.html);
                } else {
                    throw new Error('No embed HTML received');
                }
            } catch (err: any) {
                console.error('SpeakerDeck embed error:', err);
                setError(err.message);
            } finally {
                setLoading(false);
            }
        };

        if (url) {
            fetchEmbed();
        }
    }, [url]);

    useEffect(() => {
        if (embedHtml && containerRef.current) {
            // Clear previous content
            containerRef.current.innerHTML = '';

            // Insert the HTML directly (won't execute script tags via React)
            containerRef.current.innerHTML = embedHtml;

            // Extract and manually execute scripts since dangerouslySetInnerHTML drops execution
            const scripts = containerRef.current.querySelectorAll('script');
            scripts.forEach(script => {
                const newScript = document.createElement('script');
                if (script.src) {
                    newScript.src = script.src;
                    newScript.async = script.async;
                } else {
                    newScript.textContent = script.textContent;
                }
                // Copy remaining script attributes
                Array.from(script.attributes).forEach(attr => {
                    if (attr.name !== 'src') {
                        newScript.setAttribute(attr.name, attr.value);
                    }
                });
                script.parentNode?.replaceChild(newScript, script);
            });
        }
    }, [embedHtml]);

    if (loading) {
        return (
            <div className="w-full max-w-2xl bg-slate-50 flex items-center justify-center border border-slate-200 rounded-lg p-8 my-4 text-slate-500 shadow-sm animate-pulse aspect-video">
                Loading SpeakerDeck presentation...
            </div>
        );
    }

    if (error) {
        return (
            <div className="w-full max-w-2xl bg-red-50 text-red-700 border border-red-200 rounded-lg p-6 my-4 text-center aspect-video flex flex-col justify-center items-center shadow-sm">
                <span className="font-semibold mb-2">Error loading SpeakerDeck</span>
                <span className="text-sm opacity-80 mb-4">{error}</span>
                <a href={url} target="_blank" rel="noopener noreferrer" className="text-blue-600 hover:underline text-sm font-medium">
                    View on SpeakerDeck
                </a>
            </div>
        );
    }

    return (
        <div
            className="w-full max-w-2xl my-4 rounded-lg overflow-hidden shadow-lg border border-slate-200 bg-white relative"
            ref={containerRef}
        />
    );
}
