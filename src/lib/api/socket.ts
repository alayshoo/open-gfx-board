import { io } from 'socket.io-client';
import { writable } from 'svelte/store';
import { browser } from '$app/environment';
import { getBackendUrl } from '$lib/bridge';

export const connected = writable(false);
export let BACKEND_URL = getBackendUrl();

// Only create a real socket in the browser — the module is also evaluated
// server-side during SvelteKit's build/SSR pass, where io() must not run.
// Backend port is resolved in src/hooks.client.ts (before any route loads) so we
// connect to the same host:port as the Axum server (not a stale default :5000).
export const socket = browser
	? io(getBackendUrl(), { transports: ['websocket'], autoConnect: true })
	: (null as unknown as ReturnType<typeof io>);

if (browser && socket) {
	socket.on('connect', () => connected.set(true));
	socket.on('disconnect', () => connected.set(false));
}