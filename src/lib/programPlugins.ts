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

// ── Plugin popup overrides ────────────────────────────────────────────────────

export interface PluginPopupOverride {
	plugin_id: string;
	template_id: string;
	/** null means "no override — use the plugin's default popup". */
	popup_id: number | null;
	/** Duration in seconds for the overriding popup (ignored when popup_id is null). */
	duration: number;
	/** Display layer (1–3) for this plugin popup slot. */
	layer: number;
}

/**
 * Returns the popup overrides configured for each plugin template on a program.
 * Returns an empty array when none have been set yet.
 */
export async function fetchPluginPopupOverrides(programId: number): Promise<PluginPopupOverride[]> {
	try {
		const res = await fetch(`${getBaseUrl()}/programs/${programId}/plugin-popup-overrides`);
		if (!res.ok) return [];
		const data = await res.json();
		return Array.isArray(data.overrides) ? (data.overrides as PluginPopupOverride[]) : [];
	} catch {
		return [];
	}
}

/**
 * Persists the popup overrides for a program's plugins, server-side.
 */
export async function setPluginPopupOverrides(
	programId: number,
	overrides: PluginPopupOverride[],
): Promise<void> {
	try {
		await fetch(`${getBaseUrl()}/programs/${programId}/plugin-popup-overrides`, {
			method: 'PUT',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ overrides }),
		});
	} catch {
		// best-effort — silently ignore network errors
	}
}
