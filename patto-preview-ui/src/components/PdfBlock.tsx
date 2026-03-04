interface PdfBlockProps {
    src: string; // fully resolved URL (e.g. /api/files/path/to/file.pdf or https://...)
    title: string | null;
}

export default function PdfBlock({ src, title }: PdfBlockProps) {
    return (
        <div className="w-full my-4 rounded-lg overflow-hidden shadow-lg border border-slate-200 bg-slate-50">
            <div className="px-3 py-1 bg-slate-100 border-b border-slate-200 text-slate-500 text-xs truncate">
                📄 {title || src}
            </div>
            <iframe
                src={src}
                className="w-full"
                style={{ height: '600px' }}
                title={title || 'PDF'}
            />
        </div>
    );
}
