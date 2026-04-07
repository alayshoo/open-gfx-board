<script lang="ts">
	import type { ProgramPopUp } from "$lib/types";
	import { onMount } from "svelte";

	let {
		programPopUps = [],
		activePopUpId = null,
		allowPopUpsMode = false,
		onTrigger,
	}: {
		programPopUps?: ProgramPopUp[];
		activePopUpId?: number | null;
		allowPopUpsMode?: boolean;
		onTrigger?: (popup: ProgramPopUp) => void;
	} = $props();

	const manualPopUps = $derived(
		programPopUps.filter((pa) => pa.popup_launch_type === "manual" || pa.popup_launch_type === "both"),
	);

	let container: HTMLDivElement;
	let cols = $state(1);

	function computeGrid() {
		if (!container) return;
		const N = manualPopUps.length;
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
		// recompute when manualPopUps change
		manualPopUps;
		computeGrid();
	});
</script>

<div class="popup-launcher">
	<div class="panel-header">
		<span class="panel-label">PopUps</span>
		<span class="count">{manualPopUps.length}</span>
	</div>
	<div
		class="grid"
		bind:this={container}
		style="grid-template-columns: repeat({cols}, 1fr)"
	>
		{#each manualPopUps as pa (pa.id)}
			<button
				class="popup-btn"
				class:active={activePopUpId === pa.popup_id}
				disabled={!allowPopUpsMode}
				onclick={() => onTrigger?.(pa)}
				title={pa.popup?.name}
			>
				<div class="popup-sec-info">
					<span class="popup-dur">{pa.duration}s</span>
					{#if pa.popup?.sponsor_name}
						<span class="popup-sponsor">{pa.popup.sponsor_name}</span>
					{/if}
				</div>
				<div style="height: 10px;"></div>
				<div class="popup-info">
					<span class="popup-name">{pa.popup?.name}</span>
				</div>
				{#if activePopUpId === pa.popup_id}
					<span class="live-pip"></span>
				{/if}
			</button>
		{:else}
			<div class="empty-state">No manual pop-ups</div>
		{/each}
	</div>
</div>

<style>
	.popup-launcher {
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

	.popup-btn {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		justify-content: space-between;
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
		.popup-btn {
			min-height: 90px;
		}
	}

	.popup-btn:not(:disabled):hover {
		background: var(--surface-3);
		border-color: var(--border-2);
	}

	.popup-btn.active {
		background: var(--warn-dim);
		border-color: var(--warn);
		box-shadow: inset 0 0 0 1px var(--warn);
	}

	.popup-btn:disabled {
		opacity: 0.5;
		filter: grayscale(1);
		cursor: not-allowed;
		background: var(--surface-1);
		border-color: var(--border-1);
	}

	.popup-info {
		display: flex;
		flex-direction: column;
	}

	.popup-name {
		font-size: clamp(1rem, 12cqi, 3rem);
		font-weight: 500;
		color: var(--text-1);
		line-height: 1.2;
		word-break: break-word;
	}

	.active .popup-name {
		color: var(--warn);
	}

	.popup-sec-info {
		display: flex;
		flex-direction: row;
		width: calc(100% - 16px);
		gap: 6px;
		align-items: center;
		font-size: 11px;
	}

	.popup-sponsor {
		color: var(--text-3);
		font-weight: 500;
		text-transform: uppercase;
		margin-bottom: 0.1rem;
		letter-spacing: 0.05em;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.popup-dur {
		color: var(--warn);
		background: var(--warn-dim);
		padding: 2px 6px;
		border-radius: 4px;
		font-weight: 700;
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
