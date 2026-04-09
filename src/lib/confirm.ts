import { writable } from 'svelte/store';

export type ConfirmOptions = {
	title?: string;
	message: string;
	confirmLabel?: string;
	cancelLabel?: string;
	danger?: boolean;
};

type ConfirmState = {
	options: ConfirmOptions;
	resolve: (value: boolean) => void;
} | null;

export const _confirmState = writable<ConfirmState>(null);

export function showConfirm(options: ConfirmOptions): Promise<boolean> {
	return new Promise((resolve) => {
		_confirmState.set({ options, resolve });
	});
}
