<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import StatusDot from '$lib/components/StatusDot.svelte';
	import { socket, connected } from '$lib/api/socket';
	import { fetchPrograms, fetchStudios, imgUrl } from '$lib/api/api';
	import { addToast } from '$lib/toasts';
	import type { Program } from '$lib/types';
	import MediaPreview from '$lib/components/MediaPreview.svelte';

	let programs = $state<Program[]>([]);
	let studioName = $state<string>('');
	let activeId = $state<number | null>(null);
	let loading = $state(true);
	let selectingId = $state<number | null>(null);

	onMount(() => {
		fetchPrograms().then((data) => {
			programs = data;
			loading = false;
		});

		fetchStudios().then((studios) => {
			const s = studios[0];
			if (s) studioName = s.name;
		});

		socket.emit('join-studio-room', {});
		socket.emit('get-studio-state', {});

		// Named handlers so we only remove our own listeners on cleanup,
		// not every listener registered globally for these events.
		function onStudioState(data: any) {
			activeId = data.programId ?? null;
		}

		function onProgramSelected(_data: any) {
			selectingId = null;
			goto('/control');
		}

		function onUpdatePrograms() {
			fetchPrograms().then((data) => { programs = data; });
		}

		socket.on('studio-state', onStudioState);
		socket.on('program-selected', onProgramSelected);
		socket.on('update-programs', onUpdatePrograms);

		return () => {
			socket.off('studio-state', onStudioState);
			socket.off('program-selected', onProgramSelected);
			socket.off('update-programs', onUpdatePrograms);
			socket.emit('leave-studio-room', {});
		};
	});

	function selectProgram(program: Program) {
		if (selectingId !== null) return;
		selectingId = program.id;
		socket.emit('select-program', { programId: program.id });

		// Fallback: if server confirmation never arrives, reset after 5s
		setTimeout(() => {
			if (selectingId === program.id) {
				selectingId = null;
				addToast('error', 'Failed to select program. Please try again.');
			}
		}, 5000);	
	}
</script>

<div class="page">
	<header class="topbar">
		<span class="topbar-title">{studioName || 'Studio'}</span>
		<div class="topbar-right">
			<StatusDot connected={$connected} />
		</div>
	</header>

	<main class="content">
		<div class="page-title">
			<h1>Select Program</h1>
			<p>Choose a program to broadcast on {studioName || 'Studio'}.</p>
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
					{@const isSelecting = program.id === selectingId}
					<button
						class="program-card"
						class:active={isActive}
						class:selecting={isSelecting}
						disabled={selectingId !== null}
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
							{#if isSelecting}
								<div class="active-badge selecting-badge">Selecting…</div>
							{:else if isActive}
								<div class="active-badge">Active</div>
							{/if}
						</div>
						<div class="card-foot">
							<span class="card-name">{program.name}</span>
							<span class="card-meta">
								{program.graphics.length} graphics · {program.program_popups.length} pop-ups
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
		height: 100vh;
		overflow-y: auto;
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
		font-size: 1.125rem;
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
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--text-1);
		margin-bottom: 4px;
	}

	.page-title p {
		font-size: 0.875rem;
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

	.program-card:hover:not(:disabled) {
		border-color: var(--border-2);
		transform: translateY(-2px);
		box-shadow: 0 6px 20px rgba(0, 0, 0, 0.3);
	}

	.program-card:disabled {
		cursor: not-allowed;
		opacity: 0.6;
	}

	.program-card.active {
		border-color: var(--accent);
		box-shadow: 0 0 0 1px var(--accent), 0 6px 20px rgba(56, 189, 248, 0.15);
	}

	.program-card.selecting {
		border-color: var(--accent);
		opacity: 1;
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
		font-size: 0.6875rem;
		font-weight: 700;
		padding: 2px 8px;
		border-radius: 999px;
		letter-spacing: 0.05em;
	}

	.selecting-badge {
		background: var(--border-2);
		color: var(--text-1);
	}

	.card-foot {
		padding: 10px 12px;
		display: flex;
		flex-direction: column;
		gap: 3px;
	}

	.card-name {
		font-size: 0.875rem;
		font-weight: 600;
		color: var(--text-1);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.card-meta {
		font-size: 0.75rem;
		color: var(--text-3);
	}

	.empty-msg {
		padding: 48px 0;
		text-align: center;
		font-size: 0.875rem;
		color: var(--text-3);
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
	}
</style>
