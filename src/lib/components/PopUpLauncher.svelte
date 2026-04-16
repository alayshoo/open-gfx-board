<script lang="ts">
	import type { ProgramPopUp } from "$lib/types";

	const LAYER_COLORS: Record<number, { solid: string; dim: string }> = {
		1: { solid: '#f59e0b', dim: 'rgba(245,158,11,0.15)' },
		2: { solid: '#84cc16', dim: 'rgba(132,204,22,0.15)' },
		3: { solid: '#14b8a6', dim: 'rgba(20,184,166,0.15)' },
	};

	function layerColor(layer: number): { solid: string; dim: string } {
		return LAYER_COLORS[layer] ?? LAYER_COLORS[1];
	}

	let {
		programPopUps = [],
		activePopUpIds = { 1: null, 2: null, 3: null },
		allowPopUpsPerLayer = { 1: false, 2: false, 3: false },
		onTrigger,
	}: {
		programPopUps?: ProgramPopUp[];
		activePopUpIds?: Record<number, number | null>;
		allowPopUpsPerLayer?: Record<number, boolean>;
		onTrigger?: (popup: ProgramPopUp) => void;
	} = $props();

	const manualPopUps = $derived(
		programPopUps.filter((pa) => pa.popup_launch_type === "manual" || pa.popup_launch_type === "both"),
	);

	// Reactive container width — updated by the ResizeObserver attachment below
	let containerWidth = $state(0);

	const cols = $derived.by(() => {
		const N = manualPopUps.length;
		if (N === 0 || containerWidth === 0) return 1;
		const targetH = window.innerWidth < 768 ? 90 : 140;
		const maxCols = window.innerWidth < 768 ? 1 : 3;
		let bestCols = 1;
		let bestScore = Infinity;
		for (let c = 1; c <= Math.min(N, maxCols); c++) {
			const btnW = containerWidth / c;
			const score = Math.abs(Math.log(btnW / targetH));
			if (score < bestScore) {
				bestScore = score;
				bestCols = c;
			}
		}
		return bestCols;
	});

	function observeWidth(el: HTMLElement) {
		const observer = new ResizeObserver(([entry]) => {
			containerWidth = entry.contentRect.width;
		});
		observer.observe(el);
		// Read initial width synchronously
		containerWidth = el.clientWidth;
		return () => observer.disconnect();
	}
</script>

<div class="popup-launcher">
	<div class="panel-header">
		<span class="panel-label">PopUps</span>
		<span class="count">{manualPopUps.length}</span>
	</div>
	<div
		class="grid"
		style="grid-template-columns: repeat({cols}, 1fr)"
		{@attach observeWidth}
	>
		{#each manualPopUps as pa (pa.id)}
			{@const layer = pa.layer ?? 1}
			{@const isActive = activePopUpIds[layer] === pa.popup_id}
			{@const isDisabled = !allowPopUpsPerLayer[layer]}
			{@const lc = layerColor(layer)}
			<button
				class="popup-btn"
				class:active={isActive}
				disabled={isDisabled}
				onclick={() => onTrigger?.(pa)}
				title={pa.popup?.name}
				style={isActive
					? `background:${lc.dim};border-color:${lc.solid};box-shadow:inset 0 0 0 1px ${lc.solid};`
					: ''}
			>
				<div class="popup-sec-info">
					<span class="popup-dur">{pa.duration}s</span>
					{#if pa.popup?.sponsor_name}
						<span class="popup-sponsor">{pa.popup.sponsor_name}</span>
					{/if}
				</div>
				<div style="height: 10px;"></div>
				<div class="popup-info">
					<span
						class="popup-name"
						style={isActive ? `color:${lc.solid};` : ''}
					>{pa.popup?.name}</span>
				</div>
				<!-- Layer badge — always visible in top-right -->
				<span
					class="layer-badge"
					style="color:{lc.solid};background:{lc.dim};border-color:{lc.solid};"
				>{layer}</span>
				<!-- Live-pip — overlaid on top of layer badge when active -->
				{#if isActive}
					<span
						class="live-pip"
						style="background:{lc.solid};box-shadow:0 0 8px {lc.solid};"
					></span>
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

	/* Layer badge — top-right corner, always visible */
	.layer-badge {
		position: absolute;
		top: 7px;
		right: 7px;
		width: 18px;
		height: 18px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 50%;
		border: 1px solid;
		font-size: 10px;
		font-weight: 700;
		line-height: 1;
		letter-spacing: 0;
		pointer-events: none;
		z-index: 1;
	}

	/* Live-pip — sits on top of the layer badge when active */
	.live-pip {
		position: absolute;
		top: 7px;
		right: 7px;
		width: 18px;
		height: 18px;
		border-radius: 50%;
		animation: pulse 2s ease-in-out infinite;
		pointer-events: none;
		z-index: 2;
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
