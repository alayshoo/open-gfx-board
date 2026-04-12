<script lang="ts">
	import { page } from '$app/stores';
	import StatusDot from './StatusDot.svelte';
	import { connected } from '$lib/api/socket';

	let {
		back,
	}: {
		back?: { href: string; label: string };
	} = $props();

	const editLinks = [
		{ href: '/studio-editor', label: 'Presets' },
		{ href: '/program-editor', label: 'Programs' },
		{ href: '/screen-editor', label: 'Screens' },
		{ href: '/popup-editor', label: 'PopUps' },
		{ href: '/plugin-editor', label: 'Plugins' },
	];

	let editMenuOpen = $state(false);
	let editButtonEl: HTMLButtonElement | null = null;

	const isEditActive = $derived(
		editLinks.some((l) => l.href === $page.url.pathname),
	);

	$effect(() => {
		if (!editMenuOpen) return;

		function handleClickOutside(e: MouseEvent) {
			if (
				editButtonEl &&
				!editButtonEl.parentElement?.contains(e.target as Node)
			) {
				editMenuOpen = false;
			}
		}

		document.addEventListener('mousedown', handleClickOutside);
		return () => document.removeEventListener('mousedown', handleClickOutside);
	});
</script>

<nav class="topnav">
	<div class="topnav-left">
		{#if back}
			<a class="back-link" href={back.href} aria-label={back.label}>
				<svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
					<path d="M9 1L3 7l6 6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
				</svg>
			</a>
		{/if}

		<div class="nav-links">
			<a
				href="/control"
				class="nav-link"
				class:active={$page.url.pathname === '/control'}>Control</a
			>

			<!-- Edit dropdown -->
			<div class="dropdown-wrap">
				<button
					bind:this={editButtonEl}
					class="nav-link dropdown-trigger"
					class:active={isEditActive || editMenuOpen}
					onclick={() => (editMenuOpen = !editMenuOpen)}
					aria-haspopup="true"
					aria-expanded={editMenuOpen}
				>
					Edit
					<svg
						class="chevron"
						class:open={editMenuOpen}
						width="8"
						height="8"
						viewBox="0 0 8 8"
						fill="none"
						stroke="currentColor"
						stroke-width="1.5"
						aria-hidden="true"
					>
						<polyline points="1,2 4,6 7,2" />
					</svg>
				</button>

				{#if editMenuOpen}
					<div class="dropdown-menu" role="menu">
						{#each editLinks as link}
							<a
								href={link.href}
								class="dropdown-item"
								class:active={$page.url.pathname === link.href}
								role="menuitem"
								onclick={() => (editMenuOpen = false)}
							>
								{link.label}
							</a>
						{/each}
					</div>
				{/if}
			</div>

			<a
				href="/settings"
				class="nav-link"
				class:active={$page.url.pathname === '/settings'}>Settings</a
			>
		</div>
	</div>

	<div class="topnav-right">
		<StatusDot connected={$connected} />
	</div>
</nav>

<style>
	.topnav {
		height: 48px;
		padding: 0 20px;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		background: var(--surface-1);
		border-bottom: 1px solid var(--border-1);
		flex-shrink: 0;
	}

	.topnav-left,
	.topnav-right {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.back-link {
		display: inline-flex;
		align-items: center;
		padding: 6px;
		margin: -6px;
		color: var(--text-3);
		transition: color 0.15s;
		text-decoration: none;
	}

	.back-link:hover {
		color: var(--text-2);
	}

	/* ── Nav links ───────────────────────────────────────── */
	.nav-links {
		display: flex;
		align-items: center;
		gap: 2px;
	}

	.nav-link {
		font-size: 1.125rem;
		font-weight: 500;
		color: var(--text-3);
		padding: 4px 8px;
		border-radius: var(--r-sm);
		text-decoration: none;
		line-height: 1;
		background: transparent;
		transition:
			background 0.12s ease,
			color 0.12s ease;
	}

	.nav-link:hover {
		color: var(--text-1);
		background: var(--surface-2);
	}

	.nav-link.active {
		color: var(--accent);
		background: var(--accent-dim);
	}

	/* ── Edit dropdown ───────────────────────────────────── */
	.dropdown-wrap {
		position: relative;
		display: flex;
		align-items: center;
	}

	.dropdown-trigger {
		display: flex;
		align-items: center;
		gap: 4px;
		cursor: default;
		border: none;
	}

	.chevron {
		transition: transform 0.15s ease;
		flex-shrink: 0;
	}

	.chevron.open {
		transform: rotate(180deg);
	}

	.dropdown-menu {
		position: absolute;
		top: calc(100% + 6px);
		left: 0;
		min-width: 130px;
		background: var(--surface-2);
		border: 1px solid var(--border-1);
		border-radius: var(--r-sm);
		padding: 4px;
		display: flex;
		flex-direction: column;
		gap: 1px;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
		z-index: 200;
	}

	.dropdown-item {
		font-size: 1rem;
		font-weight: 500;
		color: var(--text-3);
		padding: 6px 10px;
		border-radius: var(--r-sm);
		text-decoration: none;
		transition:
			background 0.1s ease,
			color 0.1s ease;
	}

	.dropdown-item:hover {
		color: var(--text-1);
		background: var(--surface-3);
	}

	.dropdown-item.active {
		color: var(--accent);
		background: var(--accent-dim);
	}
</style>
