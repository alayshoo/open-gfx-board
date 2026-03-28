<script lang="ts">
	import OBSCmdBtn from '$lib/components/Cmd-Panel-Btn.svelte';
	import { onMount } from 'svelte';
	import type { ObsCommand } from '$lib/types';

	let {
		commands = [],
		onCommand,
	}: {
		commands?: ObsCommand[];
		onCommand?: (cmd: ObsCommand) => void;
	} = $props();

	let container: HTMLDivElement;
	let cols = $state(1);

	function computeGrid() {
		if (!container) return;
		const N = commands.length;
		if (N === 0) return;
		const W = container.clientWidth;
		const H = container.clientHeight;
		let bestCols = 1;
		let bestScore = Infinity;
		for (let c = 1; c <= N; c++) {
			const r = Math.ceil(N / c);
			const btnW = W / c;
			const btnH = H / r;
			const score = Math.abs(Math.log(btnW / btnH));
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
		// recompute when commands change
		commands;
		computeGrid();
	});
</script>

<div class="container" bind:this={container} style="grid-template-columns: repeat({cols}, 1fr)">
	{#each commands as cmd (cmd.id ?? cmd.obs_command_name)}
		<OBSCmdBtn
			title={cmd.obs_command_name}
			color={cmd.obs_command_color}
			key={cmd.obs_command_shortcut}
			onclick={() => onCommand?.(cmd)}
		/>
	{:else}
		<div class="empty">No commands configured</div>
	{/each}
</div>

<style>
	.container {
		display: grid;
		grid-auto-rows: 1fr;
		gap: 8px;
		width: 100%;
		height: 100%;
	}

	.empty {
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-3);
		font-size: 12px;
		grid-column: 1 / -1;
	}
</style>
