import { writable } from 'svelte/store';
import type { Toast } from '$lib/types';

let _id = 0;
export const toasts = writable<Toast[]>([]);

export function addToast(type: Toast['type'], message: string, duration = 4000) {
	const id = ++_id;
	toasts.update((t) => [...t, { id, type, message }]);
	setTimeout(() => {
		toasts.update((t) => t.filter((x) => x.id !== id));
	}, duration);
}
