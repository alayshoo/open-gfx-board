<script lang="ts">
	let {
		connected = false,
		label = true,
	}: { connected?: boolean; label?: boolean } = $props();
</script>

<span class="dot-wrap" class:connected>
	<span class="dot"></span>
	{#if label}
		<span class="label">{connected ? 'Connected' : 'Offline'}</span>
	{/if}
</span>

<style>
	.dot-wrap {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		font-size: 11px;
		font-weight: 500;
		color: var(--text-3);
	}

	.dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: #e40204;
		flex-shrink: 0;
	}

	.connected .dot {
		background: var(--go);
		box-shadow: 0 0 6px var(--go);
		animation: pulse 2.5s ease-in-out infinite;
	}

	.connected .label {
		color: var(--go);
	}

	@keyframes pulse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.5; }
	}
</style>
