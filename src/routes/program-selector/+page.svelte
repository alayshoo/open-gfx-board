<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import StatusDot from '$lib/components/StatusDot.svelte';
	import { socket, connected } from '$lib/socket';
	import { fetchPrograms, fetchStudios, imgUrl } from '$lib/api';
	import { addToast } from '$lib/stores/toasts';
	import type { Program, Studio } from '$lib/types';
	import MediaPreview from '$lib/components/MediaPreview.svelte';

	const studioId = $derived(Number($page.url.searchParams.get('studio')));

	let programs = $state<Program[]>([]);
	let studioName = $state<string>('');
	let activeId = $state<number | null>(null);
	let loading = $state(true);

	onMount(() => {
		if (!studioId) {
			goto('/studio-selector');
			return;
		}

		fetchPrograms().then((data) => {
			programs = data;
			loading = false;
		});

		fetchStudios().then((studios) => {
			const s = studios.find((s: Studio) => s.id === studioId);
			if (s) studioName = s.name;
		});

		socket.emit('join-studio-room', { studioId });
		socket.emit('get-studio-state', { studioId });

		socket.on('studio-state', (data: any) => {
			if (data.studioId === studioId) activeId = data.programId ?? null;
		});

		socket.on('program-selected', (data: any) => {
			if (data.studioId === studioId) {
				goto(`/control?studio=${studioId}`);
			}
		});

		socket.on('update-programs', () => {
			fetchPrograms().then((data) => { programs = data; });
		});

		return () => {
			socket.off('studio-state');
			socket.off('program-selected');
			socket.off('update-programs');
			socket.emit('leave-studio-room', { studioId });
		};
	});

	function selectProgram(program: Program) {
		socket.emit('select-program', { studioId, programId: program.id });
	}
</script>

<div class="page">
	<header class="topbar">
		<span class="topbar-title">{studioName || `Studio ${studioId}`}</span>
		<div class="topbar-right">
			<StatusDot connected={$connected} />
		</div>
	</header>

	<main class="content">
		<div class="page-title">
			<h1>Select Program</h1>
			<p>Choose a program to broadcast on {studioName || `Studio ${studioId}`}.</p>
		</div>

		{#if loading}
			<div class="empty-msg">Loading programs…</div>
		{:else if programs.length === 0}
			<div class="empty-msg">
				No programs found.
				<a href="/program-editor">Create one →</a>
			</div>
		{:else}
			<div class="program-grid">
				{#each programs as program (program.id)}
					{@const logo = imgUrl(program.logo_path)}
					{@const isActive = program.id === activeId}
					<button
						class="program-card"
						class:active={isActive}
						onclick={() => selectProgram(program)}
					>
						<div class="card-media">
							{#if logo}
								<MediaPreview src={logo} alt={program.name} class="card-img" />
							{:else}
								<div class="card-placeholder">
									<svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.2">
										<rect x="2" y="3" width="20" height="14" rx="2"/>
										<path d="M8 21h8M12 17v4"/>
									</svg>
								</div>
							{/if}
							{#if isActive}
								<div class="active-badge">Active</div>
							{/if}
						</div>
						<div class="card-foot">
							<span class="card-name">{program.name}</span>
							<span class="card-meta">
								{program.graphics.length} graphics · {program.program_ads.length} ads
							</span>
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

	.topbar {
		height: 50px;
		padding: 0 24px;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		border-bottom: 1px solid var(--border-1);
		background: var(--surface-1);
		flex-shrink: 0;
	}

	.topbar-title {
		font-size: 16px;
		font-weight: 600;
		color: var(--text-2);
	}

	.topbar-right {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.content {
		flex: 1;
		padding: 40px 32px;
		max-width: 900px;
		margin: 0 auto;
		width: 100%;
	}

	.page-title {
		margin-bottom: 28px;
	}

	.page-title h1 {
		font-size: 22px;
		font-weight: 700;
		color: var(--text-1);
		margin-bottom: 4px;
	}

	.page-title p {
		font-size: 13px;
		color: var(--text-3);
	}

	/* Grid */
	.program-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
		gap: 12px;
	}

	.program-card {
		display: flex;
		flex-direction: column;
		background: var(--surface-1);
		border: 1px solid var(--border-1);
		border-radius: var(--r-lg);
		overflow: hidden;
		cursor: pointer;
		font-family: inherit;
		text-align: left;
		transition: all 0.15s;
	}

	.program-card:hover {
		border-color: var(--border-2);
		transform: translateY(-2px);
		box-shadow: 0 6px 20px rgba(0, 0, 0, 0.3);
	}

	.program-card.active {
		border-color: var(--accent);
		box-shadow: 0 0 0 1px var(--accent), 0 6px 20px rgba(56, 189, 248, 0.15);
	}

	.card-media {
		position: relative;
		aspect-ratio: 16 / 9;
		background: var(--surface-2);
	}

	:global(.card-img) {
		width: 100%;
		height: 100%;
		object-fit: contain;
	}

	.card-placeholder {
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-3);
	}

	.active-badge {
		position: absolute;
		top: 8px;
		right: 8px;
		background: var(--accent);
		color: #000;
		font-size: 10px;
		font-weight: 700;
		padding: 2px 8px;
		border-radius: 999px;
		letter-spacing: 0.05em;
	}

	.card-foot {
		padding: 10px 12px;
		display: flex;
		flex-direction: column;
		gap: 3px;
	}

	.card-name {
		font-size: 13px;
		font-weight: 600;
		color: var(--text-1);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.card-meta {
		font-size: 11px;
		color: var(--text-3);
	}

	.empty-msg {
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
