<script lang="ts">
	import '@fontsource/figtree/400.css';
	import '@fontsource/figtree/500.css';
	import '@fontsource/figtree/600.css';
	import '@fontsource/figtree/700.css';
	import '@fontsource/figtree/800.css';
	import Toaster from '$lib/components/Toaster.svelte';
	import TitleBar from '$lib/components/TitleBarTauri.svelte';
	import { IS_TAURI } from '$lib/bridge';
	import { onMount } from 'svelte';

	let { children } = $props();

	const SPLASH_MIN_MS = 2000;

	onMount(async () => {
		if (import.meta.env.PROD) {
			document.addEventListener('contextmenu', (e) => e.preventDefault());
			document.addEventListener('keydown', (e) => {
				if ((e.ctrlKey && e.shiftKey && e.key === 'I') || e.key === 'F12') {
					e.preventDefault();
				}
			});
		}

		if (!IS_TAURI) return;
		const { invoke } = await import('@tauri-apps/api/core');
		const elapsed = performance.now();
		const remaining = SPLASH_MIN_MS - elapsed;
		if (remaining > 0) await new Promise((r) => setTimeout(r, remaining));
		await invoke('close_splashscreen');
	});
</script>

{#if IS_TAURI}
	<TitleBar />
{/if}

<div class="app-body" class:tauri={IS_TAURI}>
	{@render children()}
</div>

<Toaster />

<style>
	:global(html, body) {
		font-family: 'Figtree', system-ui, -apple-system, 'Segoe UI', sans-serif;
		font-size: 16px;
	}

	:global(*) {
		box-sizing: border-box;
		margin: 0;
		padding: 0;
	}

	:global(:root) {
		--bg: #09090b;
		--surface-1: #111114;
		--surface-2: #18181c;
		--surface-3: #222228;
		--border-1: #27272f;
		--border-2: #3f3f4a;
		--text-1: #f4f4f5;
		--text-2: #a1a1aa;
		--text-3: #52525b;
		--accent: #38bdf8;
		--accent-dim: rgba(56, 189, 248, 0.1);
		--accent-hover: #7dd3fc;
		--live: #ef4444;
		--live-dim: rgba(239, 68, 68, 0.12);
		--go: #22c55e;
		--go-dim: rgba(34, 197, 94, 0.1);
		--warn: #f59e0b;
		--warn-dim: rgba(245, 158, 11, 0.1);
		--r-sm: 4px;
		--r: 8px;
		--r-lg: 12px;
		--r-xl: 16px;
	}

	:global(html) {
		/* Kill rubber-band / bounce scroll — native apps don't do that */
		overscroll-behavior: none;
	}

	:global(body) {
		background: var(--bg);
		color: var(--text-1);
		font-family: 'Figtree', sans-serif;
		min-height: 100vh;
		-webkit-font-smoothing: antialiased;
		-moz-osx-font-smoothing: grayscale;
		/* Arrow cursor everywhere by default, just like a native app */
		cursor: default;
		/* No rubber-band on body either */
		overscroll-behavior: none;
	}

	/* ─── Native feel: no accidental text selection on UI chrome ─── */
	:global(*) {
		-webkit-user-select: none;
		user-select: none;
		/* Kill the mobile tap-highlight flash that WebView inherits */
		-webkit-tap-highlight-color: transparent;
	}

	/* Re-enable selection where it actually makes sense */
	:global(input, textarea, [contenteditable]) {
		-webkit-user-select: text;
		user-select: text;
		cursor: text;
	}

	/* ─── No ghost image when accidentally dragging UI elements ─── */
	:global(img, svg, video, canvas) {
		-webkit-user-drag: none;
		pointer-events: none; /* images are never click targets unless you opt in */
	}
	/* Opt images back in when they're meant to be interactive */
	:global(button img, a img, [role='button'] img, label img) {
		pointer-events: auto;
	}

	/* ─── Pointer cursor only on things that are actually clickable ─── */
	:global(button, [role='button'], label, select, summary, a) {
		cursor: pointer;
	}
	:global(button:disabled, [aria-disabled='true']) {
		cursor: not-allowed;
		opacity: 0.45;
	}

	/* ─── Focus: no browser ring, but keep keyboard accessibility ─── */
	:global(:focus) {
		outline: none;
	}
	:global(:focus-visible) {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	/* ─── Smooth interactive states (avoids the "web-jank" feeling) ─── */
	:global(button, .btn, a, [role='button']) {
		transition:
			background 0.12s ease,
			border-color 0.12s ease,
			color 0.12s ease,
			opacity 0.12s ease,
			box-shadow 0.12s ease;
	}

	:global(a) {
		color: var(--accent);
		text-decoration: none;
	}

	:global(button) {
		font-family: inherit;
		cursor: pointer;
		border: none;
		outline: none;
	}

	:global(input, textarea, select) {
		font-family: inherit;
		background: var(--surface-3);
		border: 1px solid var(--border-1);
		color: var(--text-1);
		border-radius: var(--r-sm);
		padding: 6px 10px;
		font-size: 13px;
		outline: none;
		transition: border-color 0.15s;
	}

	:global(input:focus, textarea:focus, select:focus) {
		border-color: var(--accent);
	}

	:global(input[type='color']) {
		padding: 2px 4px;
		height: 30px;
		cursor: pointer;
	}

	/* Scrollbars */
	:global(::-webkit-scrollbar) {
		width: 6px;
		height: 6px;
	}
	:global(::-webkit-scrollbar-track) {
		background: transparent;
	}
	:global(::-webkit-scrollbar-thumb) {
		background: var(--border-2);
		border-radius: 3px;
	}
	:global(::-webkit-scrollbar-thumb:hover) {
		background: var(--text-3);
	}

	/* Global table style for editor pages */
	:global(.data-table) {
		width: 100%;
		border-collapse: collapse;
		font-size: 16px;
	}
	:global(.data-table th) {
		text-align: left;
		padding: 12px 16px;
		font-size: 14px;
		font-weight: 600;
		letter-spacing: 0.07em;
		text-transform: uppercase;
		color: var(--text-3);
		border-bottom: 1px solid var(--border-1);
		white-space: nowrap;
	}
	:global(.data-table td) {
		padding: 12px 16px;
		border-bottom: 1px solid var(--border-1);
		color: var(--text-2);
		vertical-align: middle;
	}
	:global(.data-table tr:last-child td) {
		border-bottom: none;
	}
	:global(.data-table tr:hover td) {
		background: var(--surface-2);
		color: var(--text-1);
	}

	/* Global btn styles */
	:global(.btn) {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 10px 18px;
		border-radius: var(--r-sm);
		font-size: 13px;
		font-weight: 500;
		font-family: inherit;
		cursor: pointer;
		border: 1px solid transparent;
		transition:
			background 0.15s,
			border-color 0.15s,
			color 0.15s,
			box-shadow 0.15s;
		line-height: 1;
	}
	:global(.btn-primary) {
		background: var(--accent);
		color: #000;
		border-color: var(--accent);
	}
	:global(.btn-primary:hover) {
		background: var(--accent-hover);
		border-color: var(--accent-hover);
	}
	:global(.btn-secondary) {
		background: var(--surface-2);
		color: var(--text-2);
		border-color: var(--border-1);
	}
	:global(.btn-secondary:hover) {
		background: var(--surface-3);
		color: var(--text-1);
		border-color: var(--border-2);
	}
	:global(.btn-danger) {
		background: var(--live-dim);
		color: var(--live);
		border-color: rgba(239, 68, 68, 0.25);
	}
	:global(.btn-danger:hover) {
		background: var(--live);
		color: #fff;
	}
	:global(.btn-ghost) {
		background: transparent;
		color: var(--text-2);
		border-color: var(--border-1);
	}
	:global(.btn-ghost:hover) {
		background: var(--surface-2);
		color: var(--text-1);
	}
	:global(.btn-sm) {
		padding: 7px 14px;
		font-size: 13px;
	}
	:global(.btn-icon) {
		padding: 10px;
		aspect-ratio: 1;
		justify-content: center;
	}

	/* ── App body wrapper ──────────────────────────────────── */
	.app-body {
		display: contents; /* pass-through by default */
	}

	.app-body.tauri {
		display: flex;
		flex-direction: column;
		height: calc(100vh - 36px);
		overflow: hidden;
	}

	/* When running in Tauri, page roots must fill the flex container
	   (viewport minus titlebar) rather than 100vh on their own. */
	:global(.app-body.tauri .page-wrap),
	:global(.app-body.tauri .page) {
		min-height: 0;
		flex: 1;
		overflow-y: auto;
	}

	/* Page chrome shared */
	:global(.page-wrap) {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
	}
	:global(.page-content) {
		flex: 1;
		padding: 24px;
		max-width: 1100px;
		margin: 0 auto;
		width: 100%;
	}
	:global(.section-header) {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 16px;
	}
	:global(.section-title) {
		font-size: 14px;
		font-weight: 600;
		color: var(--text-2);
		display: flex;
		align-items: center;
		gap: 8px;
	}
	:global(.card) {
		background: var(--surface-1);
		border: 1px solid var(--border-1);
		border-radius: var(--r-lg);
		overflow: hidden;
	}
</style>
