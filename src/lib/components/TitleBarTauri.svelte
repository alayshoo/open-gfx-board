<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { page } from "$app/stores";

	let isMaximized = $state(false);
	let appWindow: import("@tauri-apps/api/window").Window | null = null;
	let editMenuOpen = $state(false);
	let editButtonEl: HTMLButtonElement | null = null;

	let unlisten: (() => void) | undefined;

	onMount(async () => {
		const { getCurrentWindow } = await import("@tauri-apps/api/window");
		appWindow = getCurrentWindow();
		isMaximized = await appWindow.isMaximized();

		unlisten = await appWindow.onResized(async () => {
			isMaximized = await appWindow!.isMaximized();
		});
	});

	onDestroy(() => unlisten?.());

	async function minimize() {
		await appWindow?.minimize();
	}

	async function toggleMaximize() {
		await appWindow?.toggleMaximize();
	}

	async function close() {
		await appWindow?.close();
	}

	const editLinks = [
		{ href: "/studio-editor", label: "Studios" },
		{ href: "/program-editor", label: "Programs" },
		{ href: "/screen-editor", label: "Screens" },
		{ href: "/ad-editor", label: "Ads" },
	] as const;

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

		document.addEventListener("mousedown", handleClickOutside);
		return () =>
			document.removeEventListener("mousedown", handleClickOutside);
	});

	const isEditActive = $derived(
		editLinks.some((l) => l.href === $page.url.pathname),
	);
</script>

<header class="titlebar" data-tauri-drag-region>
	<!-- Left: navigation -->
	<nav class="nav-links">
		<a
			href="/control"
			class="nav-link"
			class:active={$page.url.pathname === "/control"}>Control</a
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
			href="/import-export"
			class="nav-link"
			class:active={$page.url.pathname === "/import-export"}>Settings</a
		>
	</nav>

	<!-- Centre: drag zone (no content) -->
	<div class="drag-spacer" data-tauri-drag-region></div>

	<!-- Right: window controls (Windows-style order: min, max, close) -->
	<div class="window-controls">
		<button class="wc-btn" onclick={minimize} aria-label="Minimize">
			<svg
				width="10"
				height="1"
				viewBox="0 0 10 1"
				fill="currentColor"
				aria-hidden="true"
			>
				<rect width="10" height="1" />
			</svg>
		</button>

		<button
			class="wc-btn"
			onclick={toggleMaximize}
			aria-label={isMaximized ? "Restore" : "Maximize"}
		>
			{#if isMaximized}
				<svg
					width="10"
					height="10"
					viewBox="0 0 10 10"
					fill="none"
					stroke="currentColor"
					stroke-width="1"
					aria-hidden="true"
				>
					<rect x="2" y="0" width="8" height="8" />
					<polyline points="0,2 0,10 8,10" />
				</svg>
			{:else}
				<svg
					width="10"
					height="10"
					viewBox="0 0 10 10"
					fill="none"
					stroke="currentColor"
					stroke-width="1"
					aria-hidden="true"
				>
					<rect x="0.5" y="0.5" width="9" height="9" />
				</svg>
			{/if}
		</button>

		<button class="wc-btn wc-close" onclick={close} aria-label="Close">
			<svg
				width="10"
				height="10"
				viewBox="0 0 10 10"
				stroke="currentColor"
				stroke-width="1.4"
				aria-hidden="true"
			>
				<line x1="0" y1="0" x2="10" y2="10" />
				<line x1="10" y1="0" x2="0" y2="10" />
			</svg>
		</button>
	</div>
</header>

<style>
	.titlebar {
		height: 36px;
		min-height: 36px;
		display: flex;
		align-items: center;
		background: var(--surface-1);
		border-bottom: 1px solid var(--border-1);
		flex-shrink: 0;
		position: relative;
		z-index: 100;
	}

	/* ── Nav links ───────────────────────────────────────── */
	.nav-links {
		display: flex;
		align-items: center;
		gap: 2px;
		padding: 0 10px;
		height: 100%;
	}

	.nav-link {
		font-size: 12px;
		font-weight: 500;
		color: var(--text-3);
		padding: 3px 9px;
		border-radius: var(--r-sm);
		text-decoration: none;
		line-height: 1;
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
		height: 100%;
		display: flex;
		align-items: center;
	}

	.dropdown-trigger {
		display: flex;
		align-items: center;
		gap: 4px;
		cursor: default;
		background: transparent;
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
		top: calc(100% + 4px);
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
		font-size: 12px;
		font-weight: 500;
		color: var(--text-3);
		padding: 5px 9px;
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

	/* ── Central drag spacer ─────────────────────────────── */
	.drag-spacer {
		flex: 1;
		height: 100%;
	}

	/* ── Window controls ─────────────────────────────────── */
	.window-controls {
		display: flex;
		align-items: stretch;
		height: 100%;
	}

	.wc-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 46px;
		height: 100%;
		background: transparent;
		border: none;
		color: var(--text-3);
		cursor: default;
		transition:
			background 0.1s ease,
			color 0.1s ease;
	}

	.wc-btn:hover {
		background: var(--surface-3);
		color: var(--text-1);
	}

	.wc-close:hover {
		background: #c42b1c;
		color: #fff;
	}
</style>
