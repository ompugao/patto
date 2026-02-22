import { useState, useCallback, useEffect } from 'react';
import { createPortal } from 'react-dom';

interface ImageLightboxProps {
    src: string;
    alt?: string;
}

export default function ImageLightbox({ src, alt }: ImageLightboxProps) {
    const [isOpen, setIsOpen] = useState(false);

    const handleOpen = useCallback(() => setIsOpen(true), []);
    const handleClose = useCallback(() => setIsOpen(false), []);

    useEffect(() => {
        if (!isOpen) return;
        const handleKey = (e: KeyboardEvent) => {
            if (e.key === 'Escape') setIsOpen(false);
        };
        window.addEventListener('keydown', handleKey);
        return () => window.removeEventListener('keydown', handleKey);
    }, [isOpen]);

    return (
        <>
            <figure className="my-3">
                <img
                    src={src}
                    alt={alt || ''}
                    className="max-w-full rounded-lg shadow-sm cursor-pointer hover:opacity-90 transition-opacity"
                    style={{ maxHeight: '30em', objectFit: 'contain' }}
                    loading="lazy"
                    onClick={handleOpen}
                />
                {alt && <figcaption className="text-xs text-slate-500 mt-1 text-center">{alt}</figcaption>}
            </figure>

            {isOpen && createPortal(
                <div
                    style={{ position: 'fixed', inset: 0, zIndex: 9999, display: 'flex', alignItems: 'center', justifyContent: 'center', backgroundColor: 'rgba(0,0,0,0.7)', backdropFilter: 'blur(4px)' }}
                    onClick={handleClose}
                >
                    <div style={{ position: 'relative', maxWidth: '90vw', maxHeight: '90vh' }} onClick={e => e.stopPropagation()}>
                        <button
                            onClick={handleClose}
                            style={{ position: 'absolute', top: '-12px', right: '-12px', width: '32px', height: '32px', background: 'white', borderRadius: '50%', border: 'none', boxShadow: '0 2px 8px rgba(0,0,0,0.3)', display: 'flex', alignItems: 'center', justifyContent: 'center', cursor: 'pointer', fontSize: '18px', fontWeight: 'bold', color: '#475569' }}
                        >
                            ×
                        </button>
                        <img
                            src={src}
                            alt={alt || ''}
                            style={{ maxWidth: '90vw', maxHeight: '90vh', objectFit: 'contain', borderRadius: '8px', boxShadow: '0 25px 50px rgba(0,0,0,0.25)' }}
                        />
                    </div>
                </div>,
                document.body
            )}
        </>
    );
}
