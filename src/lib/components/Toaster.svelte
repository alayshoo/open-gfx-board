<script lang="ts">
	import { toasts } from '$lib/toasts';
</script>

<div class="toaster" aria-live="polite">
	{#each $toasts as toast (toast.id)}
		<div class="toast toast--{toast.type}">
			<span class="icon">
				{#if toast.type === 'success'}✓{:else if toast.type === 'error'}✕{:else}i{/if}
			</span>
			<span class="msg">{toast.message}</span>
		</div>
	{/each}
</div>

<style>
	.toaster {
		position: fixed;
		bottom: 24px;
		left: 24px;
		display: flex;
		flex-direction: column;
		gap: 8px;
		z-index: 9999;
		pointer-events: none;
	}

	.toast {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 11px 16px;
		border-radius: var(--r);
		border: 1px solid transparent;
		font-size: 0.875rem;
		font-weight: 500;
		backdrop-filter: blur(8px);
		animation: slidein 0.2s ease;
		pointer-events: auto;
	}

	.toast--success {
		background: rgba(34, 197, 94, 0.15);
		border-color: rgba(34, 197, 94, 0.3);
		color: #4ade80;
	}

	.toast--error {
		background: rgba(239, 68, 68, 0.15);
		border-color: rgba(239, 68, 68, 0.3);
		color: #f87171;
	}

	.toast--info {
		background: rgba(56, 189, 248, 0.15);
		border-color: rgba(56, 189, 248, 0.3);
		color: #7dd3fc;
	}

	.icon {
		font-size: 0.8125rem;
		width: 18px;
		height: 18px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 700;
		background: currentColor;
		color: var(--bg);
		flex-shrink: 0;
	}

	@keyframes slidein {
		from { transform: translateY(12px); opacity: 0; }
		to { transform: translateY(0); opacity: 1; }
	}
</style>
