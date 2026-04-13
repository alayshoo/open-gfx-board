const KEY_PREFIX = 'pgfx-program-plugins-';

/**
 * Returns the plugin IDs explicitly enabled for a program.
 * Returns `null` when no preference has been saved, which means
 * "no plugins enabled" (the default — all plugins are off until the user
 * explicitly enables them for this program).
 */
export function getProgramPluginIds(programId: number): string[] | null {
	if (typeof localStorage === 'undefined') return null;
	try {
		const raw = localStorage.getItem(`${KEY_PREFIX}${programId}`);
		return raw !== null ? (JSON.parse(raw) as string[]) : null;
	} catch {
		return null;
	}
}

/**
 * Persists the plugin IDs enabled for a program.
 * Pass `null` to remove the preference and revert to "show none" (the default).
 */
export function setProgramPluginIds(programId: number, ids: string[] | null): void {
	if (typeof localStorage === 'undefined') return;
	try {
		if (ids === null) {
			localStorage.removeItem(`${KEY_PREFIX}${programId}`);
		} else {
			localStorage.setItem(`${KEY_PREFIX}${programId}`, JSON.stringify(ids));
		}
	} catch {}
}
