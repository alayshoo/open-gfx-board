<script lang="ts">
	import { onMount } from "svelte";
	import { page } from "$app/stores";
	import { socket } from "$lib/api/socket";
	import { imgUrl } from "$lib/api/api";
	import type { Program, StudioState } from "$lib/types";

	const studioId = $derived(Number($page.url.searchParams.get("studio")));

	// ── Displayed state ────────────────────────────────────────────────────────
	let displayedPath = $state<string | null>(null);
	let displayedType = $state<"image" | "video">("image");
	let visible = $state(false);

	// Must stay in sync with the CSS transition duration below.
	const FADE_MS = 500;

	// ── Media cache ────────────────────────────────────────────────────────────
	// Warms the browser's HTTP cache so subsequent renders reuse the already-
	// downloaded bytes without a network round-trip.  We keep a Set of URLs we
	// have already requested so we don't fire duplicate fetches.
	const preloaded = new Set<string>();

	function getType(rawPath: string | null): "image" | "video" {
		const ext = rawPath?.split(".").pop()?.toLowerCase() ?? "";
		return ["mp4", "webm"].includes(ext) ? "video" : "image";
	}

	function preload(url: string, type: "image" | "video") {
		if (preloaded.has(url)) return;
		preloaded.add(url);
		if (type === "image") {
			const img = new Image();
			img.src = url;
		} else {
			const vid = document.createElement("video");
			vid.preload = "auto";
			vid.src = url;
			vid.load();
		}
	}

	/** Preload every graphic that belongs to the given program. */
	function preloadProgram(program: Program | null) {
		if (!program) return;
		for (const screen of program.graphics ?? []) {
			const url = imgUrl(screen.graphics_path);
			if (url) preload(url, getType(screen.graphics_path));
		}
		// Also preload the program background if present
		const bg = imgUrl(program.background_graphics_path);
		if (bg) preload(bg, getType(program.background_graphics_path));
	}

	// ── Transition engine ──────────────────────────────────────────────────────
	// Rapid events are coalesced: only the latest pending target is kept while
	// a fade is in progress.
	let pending: { path: string | null; type: "image" | "video" } | null = null;
	let busy = false;

	async function show(path: string | null, type: "image" | "video") {
		pending = { path, type };
		if (busy) return; // current fade will pick up the latest pending on finish
		busy = true;

		while (pending) {
			const target = pending;
			pending = null;

			// Fade out only when something is currently on screen
			if (visible) {
				visible = false;
				await new Promise<void>((r) => setTimeout(r, FADE_MS));
			}

			displayedPath = target.path;
			displayedType = target.type;

			if (target.path !== null) {
				// Two rAF ticks ensure the DOM has committed the new src before
				// the CSS opacity transition starts (avoids a single-frame flash).
				await new Promise<void>((r) =>
					requestAnimationFrame(() =>
						requestAnimationFrame(() => r()),
					),
				);
				// Only fade in if no newer target arrived while we were waiting.
				// If one did, skip the intermediate fade-in to avoid a visible
				// flash of the old content before the loop fades it out again.
				if (!pending) {
					visible = true;
				}
			}
		}

		busy = false;
	}

	// ── Socket ─────────────────────────────────────────────────────────────────
	onMount(() => {
		if (!studioId) return;

		socket.emit("join-studio-room", { studioId });
		// Request the current studio state so we can warm the cache immediately.
		socket.emit("get-studio-state", { studioId });

		function onStudioState(data: StudioState) {
			if (Number(data.studioId) !== studioId) return;
			preloadProgram(data.program);
			// Restore whatever is currently live so a page load / reconnect is not blank
			if (data.activeOverlay) {
				const rawPath = data.activeOverlay.graphicPath ?? null;
				const url = rawPath ? imgUrl(rawPath) : null;
				const type = getType(rawPath);
				if (url) preload(url, type);
				show(url, type);
			} else if (data.activeAd) {
				const rawPath = data.activeAd.imagePath ?? null;
				const url = rawPath ? imgUrl(rawPath) : null;
				const type = getType(rawPath);
				if (url) preload(url, type);
				show(url, type);
			} else {
				show(null, "image");
			}
		}

		function onProgramSelected(data: any) {
			if (Number(data.studioId) !== studioId) return;
			preloadProgram(data.program as Program);
		}

		function onProgramCleared(data: any) {
			if (Number(data.studioId) !== studioId) return;
			show(null, "image");
		}

		function onOverlayActivated(data: any) {
			if (Number(data.studioId) !== studioId) return;
			const rawPath: string | null = data.graphicPath ?? null;
			const url = rawPath ? imgUrl(rawPath) : null;
			const type = getType(rawPath);
			if (url) preload(url, type);
			show(url, type);
		}

		function onOverlayDeactivated(data: any) {
			if (data && Number(data.studioId) !== studioId) return;
			show(null, "image");
		}

		socket.on("studio-state", onStudioState);
		socket.on("program-selected", onProgramSelected);
		socket.on("program-cleared", onProgramCleared);
		socket.on("overlay-activated", onOverlayActivated);
		socket.on("overlay-deactivated", onOverlayDeactivated);

		return () => {
			socket.emit("leave-studio-room", { studioId });
			socket.off("studio-state", onStudioState);
			socket.off("program-selected", onProgramSelected);
			socket.off("program-cleared", onProgramCleared);
			socket.off("overlay-activated", onOverlayActivated);
			socket.off("overlay-deactivated", onOverlayDeactivated);
		};
	});
</script>

<svelte:head>
	<style>
		html,
		body {
			margin: 0;
			padding: 0;
			background: transparent !important;
			overflow: hidden;
		}
	</style>
</svelte:head>

<div class="overlay-root" class:visible>
	{#if displayedPath}
		{#if displayedType === "video"}
			<!-- svelte-ignore a11y_media_has_caption -->
			<video class="overlay-media" src={displayedPath} autoplay loop muted
			></video>
		{:else}
			<img class="overlay-media" src={displayedPath} alt="overlay" />
		{/if}
	{/if}
</div>

<style>
	.overlay-root {
		position: fixed;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: transparent;
		opacity: 0;
		/* Must match FADE_MS constant in the script block above */
		transition: opacity 0.5s ease-in-out;
		pointer-events: none;
	}

	.overlay-root.visible {
		opacity: 1;
	}

	.overlay-media {
		max-width: 100vw;
		max-height: 100vh;
		width: 100%;
		height: 100%;
		object-fit: contain;
	}
</style>
