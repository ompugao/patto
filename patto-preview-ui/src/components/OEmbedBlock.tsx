import type { OEmbedService } from '../embeds';
import { useEmbedData } from '../hooks/useEmbedData';
import { useInjectedHtml } from '../hooks/useInjectedHtml';

interface OEmbedBlockProps {
    url: string;
    service: OEmbedService;
}

export default function OEmbedBlock({ url, service }: OEmbedBlockProps) {
    const { loading, data: html, error } = useEmbedData(url, service.load);
    const containerRef = useInjectedHtml(html);

    if (loading) {
        return (
            <div className="w-full max-w-2xl bg-slate-50 flex items-center justify-center border border-slate-200 rounded-lg p-8 my-4 text-slate-500 shadow-sm animate-pulse aspect-video">
                Loading {service.name} presentation...
            </div>
        );
    }

    if (error) {
        return (
            <div className="w-full max-w-2xl bg-red-50 text-red-700 border border-red-200 rounded-lg p-6 my-4 text-center aspect-video flex flex-col justify-center items-center shadow-sm">
                <span className="font-semibold mb-2">Error loading {service.name}</span>
                <span className="text-sm opacity-80 mb-4">{error}</span>
                <a href={url} target="_blank" rel="noopener noreferrer" className="text-blue-600 hover:underline text-sm font-medium">
                    View on {service.name}
                </a>
            </div>
        );
    }

    return <div className={service.containerClass} ref={containerRef} />;
}
