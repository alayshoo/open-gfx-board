<script lang="ts">
	import type { Graphic } from '$lib/types';
	import { onMount } from 'svelte';

	let {
		screens = [],
		activeScreenId = null,
		onTrigger,
	}: {
		screens?: Graphic[];
		activeScreenId?: number | null;
		onTrigger?: (screen: Graphic) => void;
	} = $props();

	let container: HTMLDivElement;
	let cols = $state(1);

	function computeGrid() {
		if (!container) return;
		const N = screens.length;
		if (N === 0) return;
		const W = container.clientWidth;
		const targetH = window.innerWidth < 768 ? 90 : 140; // Target height for balanced buttons
		let bestCols = 1;
		let bestScore = Infinity;
		const maxCols = window.innerWidth < 768 ? 1 : 3;
		for (let c = 1; c <= Math.min(N, maxCols); c++) {
			const btnW = W / c;
			const score = Math.abs(Math.log(btnW / targetH));
			if (score < bestScore) {
				bestScore = score;
				bestCols = c;
			}
		}
		cols = bestCols;
	}

	onMount(() => {
		const observer = new ResizeObserver(computeGrid);
		observer.observe(container);
		computeGrid();
		return () => observer.disconnect();
	});

	$effect(() => {
		// recompute when screens change
		screens;
		computeGrid();
	});
</script>

<div class="screen-selector">
	<div class="panel-header">
		<span class="panel-label">
			Screens
		</span>
		<span class="count">{screens.length}</span>
	</div>
	<div class="grid" bind:this={container} style="grid-template-columns: repeat({cols}, 1fr)">
		{#each screens as g (g.id)}
			<button
				class="graphic-btn"
				class:active={activeScreenId === g.id}
				onclick={() => onTrigger?.(g)}
				title={g.graphics_name}
			>
				{#if g.allow_popups}
					<span class="popup-badge">POPUPS</span>
				{/if}
				<span class="g-name">{g.graphics_name}</span>
				{#if activeScreenId === g.id}
					<span class="live-pip"></span>
				{/if}
			</button>
		{:else}
			<div class="empty-state">
				<span>No screens for this program</span>
			</div>
		{/each}
	</div>
</div>

<style>
	.screen-selector {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.panel-header {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.panel-label {
		display: flex;
		align-items: center;
		gap: 5px;
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--text-3);
	}

	.count {
		font-size: 11px;
		font-weight: 700;
		background: var(--surface-3);
		color: var(--text-2);
		border-radius: 999px;
		padding: 1px 7px;
	}

	.grid {
		display: grid;
		gap: 8px;
		align-content: start;
		padding-right: 10px;
	}

	.graphic-btn {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		justify-content: flex-end;
		padding: 10px 12px;
		min-height: 140px;
		background: var(--surface-2);
		border: 1px solid var(--border-1);
		border-radius: var(--r);
		cursor: pointer;
		text-align: left;
		transition: all 0.15s;
		container-type: inline-size;
	}

	@media (max-width: 767px) {
		.graphic-btn {
			min-height: 90px;
		}
	}

	.graphic-btn:hover {
		background: var(--surface-3);
		border-color: var(--border-2);
	}

	.graphic-btn.active {
		background: var(--accent-dim);
		border-color: var(--accent);
		box-shadow: inset 0 0 0 1px var(--accent);
	}

	.g-name {
		font-size: clamp(1rem, 12cqi, 3rem);
		font-weight: 500;
		color: var(--text-1);
		line-height: 1.2;
		word-break: break-word;
	}

	.active .g-name {
		color: var(--accent);
	}

	.popup-badge {
		position: absolute;
		top: 8px;
		left: 8px;
		font-size: 11px;
		font-weight: 700;
		letter-spacing: 0.08em;
		background: var(--warn-dim);
		color: var(--warn);
		border-radius: 3px;
		padding: 1px 3px;
	}

	.live-pip {
		position: absolute;
		top: 8px;
		right: 8px;
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--accent);
		box-shadow: 0 0 8px var(--accent);
		animation: pulse 2s ease-in-out infinite;
	}

	@keyframes pulse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.4; }
	}

	.empty-state {
		display: flex;
		align-items: center;
		justify-content: center;
		grid-column: 1 / -1;
		padding: 24px;
		text-align: center;
		font-size: 24px;
		color: var(--text-3);
	}
</style>
