<script lang="ts">
	import { _confirmState, type ConfirmOptions } from '$lib/confirm';

	let options = $state<ConfirmOptions | null>(null);
	let resolveFn = $state<((v: boolean) => void) | null>(null);
	let open = $state(false);

	_confirmState.subscribe((state) => {
		if (state) {
			options = state.options;
			resolveFn = state.resolve;
			open = true;
		}
	});

	function answer(value: boolean) {
		open = false;
		const fn = resolveFn;
		// Reset after a tick so the closing animation isn't cut short
		setTimeout(() => {
			options = null;
			resolveFn = null;
			_confirmState.set(null);
		}, 150);
		fn?.(value);
	}

	function onBackdrop(e: MouseEvent) {
		if (e.target === e.currentTarget) answer(false);
	}

	function onKeydown(e: KeyboardEvent) {
		if (!open) return;
		if (e.key === 'Escape') answer(false);
		if (e.key === 'Enter') answer(true);
	}
</script>

<svelte:window onkeydown={onKeydown} />

{#if open && options}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="backdrop" onclick={onBackdrop}>
		<div class="modal" role="alertdialog" aria-modal="true" aria-labelledby="confirm-title">
			<div class="modal-head">
				<span class="modal-title" id="confirm-title">
					{options.title ?? 'Confirm'}
				</span>
			</div>
			<div class="modal-body">
				<p>{options.message}</p>
			</div>
			<div class="modal-foot">
				<button class="btn btn-secondary btn-sm" onclick={() => answer(false)}>
					{options.cancelLabel ?? 'Cancel'}
				</button>
				<button
					class="btn btn-sm"
					class:btn-danger={options.danger !== false}
					class:btn-primary={options.danger === false}
					onclick={() => answer(true)}
				>
					{options.confirmLabel ?? 'Confirm'}
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.72);
		backdrop-filter: blur(4px);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 2000;
		padding: 16px;
	}

	.modal {
		background: var(--surface-1);
		border: 1px solid var(--border-1);
		border-radius: var(--r-xl);
		width: 100%;
		max-width: 400px;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		box-shadow: 0 24px 64px rgba(0, 0, 0, 0.6);
	}

	.modal-head {
		padding: 16px 20px 0;
	}

	.modal-title {
		font-size: 14px;
		font-weight: 600;
		color: var(--text-1);
	}

	.modal-body {
		padding: 12px 20px 20px;
	}

	.modal-body p {
		font-size: 0.875rem;
		color: var(--text-2);
		line-height: 1.6;
	}

	.modal-foot {
		padding: 14px 20px;
		border-top: 1px solid var(--border-1);
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		background: var(--surface-2);
	}
</style>
