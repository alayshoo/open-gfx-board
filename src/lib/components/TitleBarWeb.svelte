<script lang="ts">
	import StatusDot from './StatusDot.svelte';
	import { connected } from '$lib/api/socket';

	let {
		back,
	}: {
		back?: { href: string; label: string };
	} = $props();
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
	</div>
	<div class="topnav-right">
		<div class="nav-links">
			<a href="/program-editor">Programs</a>
			<a href="/ad-editor">Ads</a>
			<a href="/studio-editor">Studios</a>
			<a href="/screen-editor">Screens</a>
			<a href="/import-export">Settings</a>
		</div>
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
		flex: 1;
	}

	.topnav-right {
		justify-content: flex-end;
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

	.nav-links {
		display: flex;
		align-items: center;
		gap: 2px;
	}

	.nav-links a {
		font-size: 16px;
		color: var(--text-3);
		padding: 4px 8px;
		border-radius: var(--r-sm);
		text-decoration: none;
		transition: all 0.15s;
	}

	.nav-links a:hover {
		color: var(--text-1);
		background: var(--surface-2);
	}
</style>
