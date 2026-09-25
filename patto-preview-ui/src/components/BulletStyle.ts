import { createContext, useCallback, useContext, useState } from 'react';

/**
 * How nested lines are marked in the preview.
 * - guide:  a vertical guide line down the left of each nesting level (no markers)
 * - bullet: •, ◦, ▪ cycling with depth, like the HTML renderer's nested lists
 * - dash:   an en dash before each nested line
 * - none:   indentation only
 */
export type BulletStyle = 'guide' | 'bullet' | 'dash' | 'none';

export const BULLET_STYLES: { value: BulletStyle; label: string }[] = [
    { value: 'guide', label: 'Guide line' },
    { value: 'bullet', label: 'Bullets' },
    { value: 'dash', label: 'Dashes' },
    { value: 'none', label: 'None' },
];

const STORAGE_KEY = 'patto-preview.bulletStyle';
const isBulletStyle = (v: unknown): v is BulletStyle => BULLET_STYLES.some(s => s.value === v);

function loadBulletStyle(): BulletStyle {
    try {
        const stored = localStorage.getItem(STORAGE_KEY);
        if (isBulletStyle(stored)) return stored;
    } catch {
        // Storage can be unavailable (private windows, blocked site data)
    }
    return 'guide';
}

/** The chosen style, remembered per browser. */
export function usePersistedBulletStyle(): [BulletStyle, (s: BulletStyle) => void] {
    const [style, setStyle] = useState<BulletStyle>(loadBulletStyle);
    const update = useCallback((s: BulletStyle) => {
        setStyle(s);
        try {
            localStorage.setItem(STORAGE_KEY, s);
        } catch {
            // Still applies for this session
        }
    }, []);
    return [style, update];
}

export const BulletStyleContext = createContext<BulletStyle>('guide');
export const useBulletStyle = () => useContext(BulletStyleContext);

const BULLETS = ['•', '◦', '▪'];

/** The marker shown before a line at `depth` (1 = first nested level), or null for none. */
export function bulletMarker(style: BulletStyle, depth: number): string | null {
    if (depth < 1) return null;
    switch (style) {
        case 'bullet': return BULLETS[(depth - 1) % BULLETS.length];
        case 'dash': return '–';
        default: return null;
    }
}
