<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { socket } from '$lib/api/socket';
	import { fetchStudios } from '$lib/api/api';
	import { getBackendUrl, getLocalIp } from '$lib/bridge';
	import type { Studio, Preset } from '$lib/types';
	import { IS_TAURI } from '$lib/bridge';
	import TitleBarWeb from '$lib/components/TitleBarWeb.svelte';

	let studio = $state<Studio | null>(null);
	let loading = $state(true);
	let localIp = $state<string | null>(null);

	const presets = $derived(studio?.presets ?? []);

	onMount(() => {
		fetchStudios().then((data) => {
			studio = data[0] ?? null;
			loading = false;
		});

		const ipInterval = setInterval(() => {
			const ip = getLocalIp();
			if (ip) {
				localIp = ip;
				clearInterval(ipInterval);
			}
		}, 200);
		setTimeout(() => clearInterval(ipInterval), 5000);

		socket.on('studio-updated', () => {
			fetchStudios().then((data) => {
				studio = data[0] ?? null;
			});
		});

		socket.on('update-studios', () => {
			fetchStudios().then((data) => {
				studio = data[0] ?? null;
			});
		});

		return () => {
			socket.off('studio-updated');
			socket.off('update-studios');
		};
	});

	function selectPreset(preset: Preset) {
		if (!studio) return;
		goto(`/control?studio=${studio.id}&preset=${preset.id}`);
	}
</script>

<div class="page">
	{#if !IS_TAURI}
		<TitleBarWeb back={{ href: '/', label: 'Presets' }} />
	{/if}

	<main class="content">
		<div class="page-title">
			{#if studio && !loading}
				<h2>Select a Preset</h2>
				<span class="studio-label">{studio.name}</span>
			{:else if !loading}
				<h2>No Studio Found</h2>
			{/if}
		</div>

		{#if loading}
			<div class="empty-message">Loading…</div>
		{:else if !studio}
			<div class="empty-message">
				No studio found. <a href="/studio-editor">Configure one →</a>
			</div>
		{:else if presets.length === 0}
			<div class="empty-message">
				No presets configured. <a href="/studio-editor">Add presets →</a>
			</div>
		{:else}
			<div class="preset-grid">
				{#each presets as preset (preset.id)}
					<button class="preset-card" onclick={() => selectPreset(preset)}>
						<div class="preset-number">
							{(preset.id ?? 0).toString().padStart(2, '0')}
						</div>
						<div class="preset-body">
							<div class="preset-name">{preset.name}</div>
							<div class="preset-meta">
								{preset.commands.length} command{preset.commands.length !== 1 ? 's' : ''}
							</div>
						</div>
						{#if preset.commands.length > 0}
							<div class="color-dots">
								{#each preset.commands.slice(0, 8) as cmd}
									<span class="dot" style="background: {cmd.obs_command_color}"></span>
								{/each}
							</div>
						{/if}
						<div class="preset-arrow">
							<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
								<path d="M5 12h14M12 5l7 7-7 7" />
							</svg>
						</div>
					</button>
				{/each}
			</div>
		{/if}
	</main>
</div>

<style>
	.page {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
		background: var(--bg);
	}

	.content {
		flex: 1;
		padding: 40px 32px;
		max-width: 900px;
		margin: 0 auto;
		width: 100%;
	}

	.page-title {
		margin-bottom: 32px;
		display: flex;
		align-items: baseline;
		gap: 14px;
	}

	.page-title h2 {
		font-size: 20px;
		font-weight: 400;
		color: var(--text-3);
	}

	.studio-label {
		font-size: 13px;
		color: var(--text-3);
		background: var(--surface-2);
		border: 1px solid var(--border-1);
		border-radius: var(--r-sm);
		padding: 2px 8px;
	}

	.preset-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
		gap: 12px;
	}

	.preset-card {
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 16px 18px;
		background: var(--surface-1);
		border: 1px solid var(--border-1);
		border-radius: var(--r-lg);
		cursor: pointer;
		text-align: left;
		transition: all 0.15s;
		font-family: inherit;
	}

	.preset-card:hover {
		border-color: var(--accent);
		background: var(--surface-2);
		transform: translateY(-1px);
		box-shadow: 0 4px 20px rgba(56, 189, 248, 0.1);
	}

	.preset-number {
		font-size: 22px;
		font-weight: 800;
		color: var(--text-3);
		font-variant-numeric: tabular-nums;
		flex-shrink: 0;
		width: 36px;
	}

	.preset-body {
		flex: 1;
		min-width: 0;
	}

	.preset-name {
		font-size: clamp(0.8rem, 4vw, 1.25rem);
		font-weight: 700;
		color: var(--text-1);
		margin-bottom: 4px;
		line-height: 1.2;
		overflow-wrap: break-word;
	}

	.preset-meta {
		font-size: clamp(0.65rem, 3.5vw, 0.8rem);
		color: var(--text-3);
	}

	.color-dots {
		display: flex;
		gap: 4px;
		flex-shrink: 0;
		flex-wrap: wrap;
		max-width: 72px;
		align-content: center;
	}

	.dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex-shrink: 0;
		opacity: 0.85;
	}

	.preset-arrow {
		color: var(--text-3);
		flex-shrink: 0;
		transition: transform 0.15s, color 0.15s;
	}

	.preset-card:hover .preset-arrow {
		transform: translateX(3px);
		color: var(--accent);
	}

	.empty-message {
		padding: 48px 0;
		text-align: center;
		font-size: 13px;
		color: var(--text-3);
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
	}

</style>
