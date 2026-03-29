export const IS_TAURI = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

function readStoredBackendPort(): number {
	if (typeof window === 'undefined') return 5000;
	try {
		const s = localStorage.getItem('backend_port');
		if (!s) return 5000;
		const p = parseInt(s, 10);
		return Number.isFinite(p) && p > 0 ? p : 5000;
	} catch {
		return 5000;
	}
}

let _backendPort = readStoredBackendPort();
let _localIp: string | null = null;

export function getBackendUrl(): string {
	if (IS_TAURI || import.meta.env.DEV) {
		return `http://localhost:${_backendPort}`;
	}
	return window.location.origin;
}

export function getLocalIp(): string | null {
	return _localIp;
}

export async function initBackendUrl(): Promise<void> {
	for (const port of [5000, 5174, 3000, 8080, 8000]) {
		try {
			const controller = new AbortController();
			const timeout = setTimeout(() => controller.abort(), 500);
			const res = await fetch(`http://localhost:${port}/health`, { signal: controller.signal });
			clearTimeout(timeout);
			if (res.ok) {
				_backendPort = port;
				if (typeof localStorage !== 'undefined') {
					localStorage.setItem('backend_port', String(port));
				}
				break;
			}
		} catch {
			// try next port
		}
	}
	// After finding the port, fetch the LAN IP
	try {
		const res = await fetch(`${getBackendUrl()}/local-ip`);
		if (res.ok) {
			const data = await res.json();
			_localIp = data.ip ?? null;
		}
	} catch {
		// local IP detection unavailable
	}
}

export const BACKEND_URL = {
	get value() { return getBackendUrl(); }
};
