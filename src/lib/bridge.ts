/* $lib/bridge.ts */

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
	const portsToProbe: number[] = [];

	// Always try the user's preferred port first so the frontend finds the
	// backend even when it's on a non-default port.
	if (IS_TAURI) {
		try {
			const { invoke } = await import('@tauri-apps/api/core');
			const preferred: number | null = await invoke('get_preferred_port');
			if (preferred != null) portsToProbe.push(preferred);
		} catch { /* ignore — fall through to defaults */ }
	} else {
		// Dev / web build: preferred port may have been saved to localStorage.
		const stored = typeof localStorage !== 'undefined'
			? localStorage.getItem('preferred_port')
			: null;
		if (stored) {
			const p = parseInt(stored, 10);
			if (Number.isFinite(p) && p > 0) portsToProbe.push(p);
		}
	}

	// Append the standard fallback ports (skip any already in the list).
	for (const p of [5000, 5174, 3000, 8080, 8000]) {
		if (!portsToProbe.includes(p)) portsToProbe.push(p);
	}

	for (const port of portsToProbe) {
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

	// After finding the port, fetch the LAN IP.
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

/** Returns the port the frontend is currently connected to (Tauri / dev only). */
export function getCurrentPort(): number {
	return _backendPort;
}
