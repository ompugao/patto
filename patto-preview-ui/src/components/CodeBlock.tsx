import { useMemo } from 'react';
import 'highlight.js/styles/github.css';
import hljs from 'highlight.js';

interface CodeBlockProps {
    code: string;
    language?: string;
    inline?: boolean;
}

export default function CodeBlock({ code, language, inline = false }: CodeBlockProps) {
    const highlightedHtml = useMemo(() => {
        if (inline) return '';
        try {
            return hljs.highlightAuto(code, language ? [language] : undefined).value;
        } catch (e) {
            console.error('Highlight.js error:', e);
            return '';
        }
    }, [code, language, inline]);

    if (inline) {
        return <code className="bg-slate-100 text-rose-600 px-1 rounded text-sm font-mono">{code}</code>;
    }

    return (
        <pre className="hljs p-4 rounded-lg overflow-x-auto text-sm my-3 font-mono border border-slate-200">
            {highlightedHtml ? (
                <code dangerouslySetInnerHTML={{ __html: highlightedHtml }} />
            ) : (
                <code>{code}</code>
            )}
        </pre>
    );
}
