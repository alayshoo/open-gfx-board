import { initBackendUrl } from '$lib/bridge';

/** Runs once before routes load — must finish before Socket.IO and HTTP API use getBackendUrl(). */
export async function init() {
	await initBackendUrl();
}
