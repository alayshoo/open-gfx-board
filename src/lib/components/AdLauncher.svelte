<script lang="ts">
	import type { ProgramAd } from "$lib/types";
	import { onMount } from "svelte";

	let {
		programAds = [],
		activeAdId = null,
		allowAdsMode = false,
		onTrigger,
	}: {
		programAds?: ProgramAd[];
		activeAdId?: number | null;
		allowAdsMode?: boolean;
		onTrigger?: (ad: ProgramAd) => void;
	} = $props();

	const manualAds = $derived(
		programAds.filter((pa) => pa.ad_launch_type === "manual"),
	);

	let container: HTMLDivElement;
	let cols = $state(1);

	function computeGrid() {
		if (!container) return;
		const N = manualAds.length;
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
		// recompute when manualAds change
		manualAds;
		computeGrid();
	});
</script>

<div class="ad-launcher">
	<div class="panel-header">
		<span class="panel-label">Ads</span>
		<span class="count">{manualAds.length}</span>
	</div>
	<div
		class="grid"
		bind:this={container}
		style="grid-template-columns: repeat({cols}, 1fr)"
	>
		{#each manualAds as pa (pa.id)}
			<button
				class="ad-btn"
				class:active={activeAdId === pa.ad_id}
				disabled={!allowAdsMode}
				onclick={() => onTrigger?.(pa)}
				title={pa.ad?.name}
			>
				<div class="ad-info">
					<span class="ad-name">{pa.ad?.name}</span>
				</div>
				<div class="ad-sec-info">
					<span class="ad-dur">{pa.duration}s</span>
					{#if pa.ad?.sponsor_name}
						<span class="ad-sponsor">{pa.ad.sponsor_name}</span>
					{/if}
				</div>
				{#if activeAdId === pa.ad_id}
					<span class="live-pip"></span>
				{/if}
			</button>
		{:else}
			<div class="empty-state">No manual ads</div>
		{/each}
	</div>
</div>

<style>
	.ad-launcher {
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

	.ad-btn {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		justify-content: flex-end;
		padding: 10px 12px;
		background: var(--surface-2);
		border: 1px solid var(--border-1);
		border-radius: var(--r);
		cursor: pointer;
		text-align: left;
		transition: all 0.15s;
		min-height: 140px;
		container-type: inline-size;
	}

	@media (max-width: 767px) {
		.ad-btn {
			min-height: 90px;
		}
	}

	.ad-btn:not(:disabled):hover {
		background: var(--surface-3);
		border-color: var(--border-2);
	}

	.ad-btn.active {
		background: var(--warn-dim);
		border-color: var(--warn);
		box-shadow: inset 0 0 0 1px var(--warn);
	}

	.ad-btn:disabled {
		opacity: 0.5;
		filter: grayscale(1);
		cursor: not-allowed;
		background: var(--surface-1);
		border-color: var(--border-1);
	}

	.ad-info {
		display: flex;
		flex-direction: column;
	}

	.ad-name {
		font-size: clamp(1rem, 12cqi, 3rem);
		font-weight: 500;
		color: var(--text-1);
		line-height: 1.2;
		word-break: break-word;
	}

	.active .ad-name {
		color: var(--warn);
	}

	.ad-sec-info {
		display: flex;
		flex-direction: row;
		gap: 6px;
		align-items: center;

		position: absolute;
		top: 8px;
		left: 8px;
		font-size: 14px;
	}

	.ad-sponsor {
		color: var(--text-3);
		font-weight: 500;
		text-transform: uppercase;
		margin-bottom: 0.1rem;
		letter-spacing: 0.05em;
	}

	.ad-dur {
		color: var(--warn);
		background: var(--warn-dim);
		padding: 2px 6px;
		border-radius: 4px;
		font-weight: 800;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.live-pip {
		position: absolute;
		top: 8px;
		right: 8px;
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--warn);
		box-shadow: 0 0 8px var(--warn);
		animation: pulse 2s ease-in-out infinite;
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.4;
		}
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
