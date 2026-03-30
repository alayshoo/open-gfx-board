<!-- $lib/components/Modal.svelte -->

<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		open = $bindable(false),
		title = '',
		width = '540px',
		onclose,
		children,
		footer,
	}: {
		open?: boolean;
		title?: string;
		width?: string;
		onclose?: () => void;
		children?: Snippet;
		footer?: Snippet;
	} = $props();

	function close() {
		open = false;
		onclose?.();
	}

	function onBackdrop(e: MouseEvent) {
		if (e.target === e.currentTarget) close();
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') close();
	}
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="backdrop" onclick={onBackdrop}>
		<div class="modal" style="max-width:{width}">
			<div class="modal-head">
				<span class="modal-title">{title}</span>
				<button class="close-btn" onclick={close} aria-label="Close">
					<svg width="14" height="14" viewBox="0 0 14 14" fill="none">
						<path d="M1 1l12 12M13 1L1 13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
					</svg>
				</button>
			</div>
			<div class="modal-body">
				{@render children?.()}
			</div>
			{#if footer}
				<div class="modal-foot">
					{@render footer()}
				</div>
			{/if}
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
		z-index: 1000;
		padding: 16px;
	}

	.modal {
		background: var(--surface-1);
		border: 1px solid var(--border-1);
		border-radius: var(--r-xl);
		width: 100%;
		max-height: 90vh;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		box-shadow: 0 24px 64px rgba(0, 0, 0, 0.6);
	}

	.modal-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 16px 20px;
		border-bottom: 1px solid var(--border-1);
		flex-shrink: 0;
	}

	.modal-title {
		font-size: 14px;
		font-weight: 600;
		color: var(--text-1);
	}

	.close-btn {
		background: var(--surface-2);
		border: 1px solid var(--border-1);
		color: var(--text-2);
		border-radius: var(--r-sm);
		width: 28px;
		height: 28px;
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		transition: all 0.15s;
	}

	.close-btn:hover {
		background: var(--surface-3);
		color: var(--text-1);
	}

	.modal-body {
		padding: 20px;
		overflow-y: auto;
		flex: 1;
	}

	.modal-foot {
		padding: 14px 20px;
		border-top: 1px solid var(--border-1);
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		flex-shrink: 0;
		background: var(--surface-2);
	}
</style>
