declare global {
    interface Window {
        twttr?: {
            _e: Array<(t: Window['twttr']) => void>;
            ready: (f: (t: Window['twttr']) => void) => void;
            widgets: {
                load: (element?: HTMLElement) => void;
            };
        };
    }
}

/** Resolves with Twitter's widgets API, loading its script on first use. */
export function loadTwitterWidgets(): Promise<NonNullable<Window['twttr']>> {
    return new Promise((resolve) => {
        // widgets.js drains `_e` when it loads, so callbacks queued on this stub
        // run even when it is requested before the script arrives.
        if (!window.twttr) {
            window.twttr = {
                _e: [],
                ready(f) { this._e.push(f); },
                widgets: { load: () => {} },
            };
        }

        window.twttr.ready((twttr) => resolve(twttr!));

        if (!document.getElementById('twitter-wjs')) {
            const script = document.createElement('script');
            script.id = 'twitter-wjs';
            script.src = 'https://platform.twitter.com/widgets.js';
            script.async = true;
            script.charset = 'utf-8';
            document.head.appendChild(script);
        }
    });
}
