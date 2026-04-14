import { getBaseUrl } from '$lib/api/api';

/**
 * Returns the plugin IDs enabled for a program, fetched from the server.
 * Returns an empty array when no preference has been saved yet.
 *
 * Preferences are stored server-side so they are shared across all clients
 * (Tauri window, browser, remote devices) rather than being per-device.
 */
export async function fetchProgramPluginIds(programId: number): Promise<string[]> {
	try {
		const res = await fetch(`${getBaseUrl()}/programs/${programId}/plugin-prefs`);
		if (!res.ok) return [];
		const data = await res.json();
		return Array.isArray(data.plugin_ids) ? (data.plugin_ids as string[]) : [];
	} catch {
		return [];
	}
}

/**
 * Persists the plugin IDs enabled for a program, server-side.
 */
export async function setProgramPluginIds(programId: number, ids: string[]): Promise<void> {
	try {
		await fetch(`${getBaseUrl()}/programs/${programId}/plugin-prefs`, {
			method: 'PUT',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ plugin_ids: ids }),
		});
	} catch {
		// best-effort — silently ignore network errors
	}
}
