import { io } from 'socket.io-client';
import { writable } from 'svelte/store';

// In dev (Vite), the frontend runs on a different port than Flask, so we need
// to explicitly target the Flask server. In production (served by Flask), we
// must use the same origin the browser loaded the page from — which also makes
// the app work correctly when accessed from other devices on the network.
export const BACKEND_URL = import.meta.env.DEV ? 'http://localhost:5000' : window.location.origin;

export const socket = io(BACKEND_URL, {
	transports: ['websocket', 'polling'],
	autoConnect: true,
});

export const connected = writable(false);

socket.on('connect', () => connected.set(true));
socket.on('disconnect', () => connected.set(false));
