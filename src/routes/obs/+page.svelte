<script lang="ts">
	import { onMount } from "svelte";
	import { page } from "$app/stores";
	import { socket } from "$lib/api/socket";
	import { imgUrl } from "$lib/api/api";
	import type { Program, StudioState } from "$lib/types";

	type Direction = "top" | "bottom" | "left" | "right";

	const studioId = $derived(Number($page.url.searchParams.get("studio")));

	// ── Displayed state ────────────────────────────────────────────────────────
	let displayedPath = $state<string | null>(null);
	let displayedType = $state<"image" | "video">("image");
	let visible = $state(false);

	// Must stay in sync with the CSS transition duration below.
	const FADE_MS = 500;
	const AD_SLIDE_MS = 1000;

	// ── Media cache ─────────────────────────────────────────────────────────────
	// Warms the browser's HTTP cache so subsequent renders reuse already-
	// downloaded bytes without a network round-trip.
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
		const bg = imgUrl(program.background_graphics_path);
		if (bg) preload(bg, getType(program.background_graphics_path));
	}

	// ── Main overlay transition engine ──────────────────────────────────────────
	// Rapid events are coalesced: only the latest pending target is kept while
	// a fade is in progress.
	let pending: { path: string | null; type: "image" | "video" } | null = null;
	let busy = false;

	function sleep(ms: number): Promise<void> {
		return new Promise((r) => setTimeout(r, ms));
	}

	function waitForMedia(src: string, type: "image" | "video"): Promise<void> {
		return new Promise((resolve) => {
			if (type === "image") {
				const img =
					document.querySelector<HTMLImageElement>(".overlay-media-img");
				if (!img || !img.src) {
					resolve();
					return;
				}
				// .decode() resolves when the image is decoded and ready
				// to composite without jank.
				img.decode().then(resolve, resolve);
			} else {
				const vid =
					document.querySelector<HTMLVideoElement>(".overlay-media-vid");
				if (!vid) {
					resolve();
					return;
				}
				if (vid.readyState >= 3) {
					resolve();
				} else {
					vid.addEventListener("canplay", () => resolve(), { once: true });
				}
			}
		});
	}

	async function show(path: string | null, type: "image" | "video") {
		pending = { path, type };
		if (busy) return;
		busy = true;

		while (pending) {
			const target = pending;
			pending = null;

			if (visible) {
				visible = false;
				await sleep(FADE_MS);
			}

			displayedPath = target.path;
			displayedType = target.type;

			if (target.path !== null) {
				await waitForMedia(target.path, target.type);
				if (!pending) {
					visible = true;
				}
			}
		}

		busy = false;
	}

	// ── Ad overlay – direct DOM management ─────────────────────────────────────
	// All ad animation is handled imperatively to avoid reactive overhead and to
	// guarantee correct sequencing (hide current → load next → slide in) with no
	// visual glitches regardless of how rapidly events arrive.
	let adContainer: HTMLDivElement;
	let adImg: HTMLImageElement;
	let adVideo: HTMLVideoElement;

	type AdPayload = {
		src: string;
		type: "image" | "video";
		direction: Direction;
		position: number;
	} | null;

	// Coalescing queue – only the most-recent pending request is kept so rapid
	// ad changes never queue up a backlog of animations.
	let adPending: { payload: AdPayload } | null = null;
	let adBusy = false;
	let adCurrentDirection: Direction = "bottom";
	let adIsVisible = false;

	// ── Auto-ad & filler scheduling state ───────────────────────────────────────
	let currentProgram: Program | null = null;
	let overlayActive = false;
	let fillerIsActive = false;
	let fillerSuppressed = false;
	let autoAdTimers = new Map<number, ReturnType<typeof setTimeout>>();
	let fillerRotationTimer: ReturnType<typeof setInterval> | null = null;
	let fillerIndex = 0;

	function adSlideOutTransform(dir: Direction): string {
		if (dir === "bottom") return "translateY(118%)";
		if (dir === "top") return "translateY(-118%)";
		if (dir === "left") return "translateX(-118%)";
		return "translateX(118%)";
	}

	function waitForAdMedia(type: "image" | "video"): Promise<void> {
		return new Promise((resolve) => {
			if (type === "image") {
				if (adImg.complete && adImg.naturalWidth > 0) {
					resolve();
					return;
				}
				adImg.addEventListener("load", () => resolve(), { once: true });
				adImg.addEventListener("error", () => resolve(), { once: true });
			} else {
				if (adVideo.readyState >= 3) {
					resolve();
					return;
				}
				adVideo.addEventListener("canplay", () => resolve(), { once: true });
				adVideo.addEventListener("error", () => resolve(), { once: true });
			}
		});
	}

	async function triggerAd(payload: AdPayload) {
		adPending = { payload };
		if (adBusy) return;
		adBusy = true;

		while (adPending) {
			const { payload: target } = adPending;
			adPending = null;

			// ── Step 1: slide out the current ad if one is visible ───────────
			if (adIsVisible) {
				adIsVisible = false;
				adContainer.style.transition = `transform ${AD_SLIDE_MS}ms cubic-bezier(0.4,0,0.2,1)`;
				adContainer.style.transform = adSlideOutTransform(adCurrentDirection);
				await sleep(AD_SLIDE_MS);
				// Clear media after slide-out completes
				adImg.src = "";
				adImg.style.display = "none";
				adVideo.src = "";
				adVideo.load();
				adVideo.style.display = "none";
				adContainer.style.display = "none";
			}

			if (!target) continue; // Pure hide request – nothing more to do

			if (adPending) continue; // A newer request arrived while hiding – restart

			// ── Step 2: set up the new ad media (hidden, off-screen) ─────────
			adCurrentDirection = target.direction;

			if (target.type === "video") {
				adImg.style.display = "none";
				adVideo.style.display = "block";
				adVideo.src = target.src;
				adVideo.load();
			} else {
				adVideo.style.display = "none";
				adImg.style.display = "block";
				adImg.src = target.src;
			}

			// Place the container off-screen in the incoming direction with no
			// transition so there is no visible flash while the media loads.
			adContainer.style.transition = "none";
			adContainer.style.transform = adSlideOutTransform(target.direction);
			adContainer.style.display = "block";

			await waitForAdMedia(target.type);

			if (adPending) {
				// Superseded while loading – tear down and restart
				adImg.src = "";
				adImg.style.display = "none";
				adVideo.src = "";
				adVideo.load();
				adVideo.style.display = "none";
				adContainer.style.display = "none";
				continue;
			}

			// ── Step 3: size and position the container ──────────────────────
			const naturalW =
				target.type === "image" ? adImg.naturalWidth : adVideo.videoWidth;
			const naturalH =
				target.type === "image" ? adImg.naturalHeight : adVideo.videoHeight;
			const halfW = naturalW / 2;
			const halfH = naturalH / 2;
			const pos = target.position;

			adContainer.style.width = `${naturalW}px`;
			adContainer.style.height = `${naturalH}px`;
			adContainer.style.top = "";
			adContainer.style.bottom = "";
			adContainer.style.left = "";
			adContainer.style.right = "";

			if (target.direction === "bottom") {
				adContainer.style.bottom = "30px";
				adContainer.style.left = `calc(${pos}% - ${halfW}px)`;
			} else if (target.direction === "top") {
				adContainer.style.top = "30px";
				adContainer.style.left = `calc(${pos}% - ${halfW}px)`;
			} else if (target.direction === "left") {
				adContainer.style.left = "30px";
				adContainer.style.top = `calc(${pos}% - ${halfH}px)`;
			} else {
				adContainer.style.right = "30px";
				adContainer.style.top = `calc(${pos}% - ${halfH}px)`;
			}

			// ── Step 4: snap to off-screen start position, then slide in ─────
			// transition is already "none" and transform is already the slide-out
			// value from step 2, so we just need a reflow to commit that state
			// before re-enabling the transition and moving to translate(0,0).
			void adContainer.offsetHeight; // force reflow

			adContainer.style.transition = `transform ${AD_SLIDE_MS}ms cubic-bezier(0.4,0,0.2,1)`;
			adContainer.style.transform = "translate(0,0)";
			adIsVisible = true;

			await sleep(AD_SLIDE_MS); // wait for slide-in to finish
		}

		adBusy = false;
	}

	// ── Ad payload builder ──────────────────────────────────────────────────────
	function buildAdPayload(pa: import("$lib/types").ProgramAd): AdPayload | null {
		const rawPath = pa.ad?.image_path ?? null;
		const url = rawPath ? imgUrl(rawPath) : null;
		if (!url) return null;
		return {
			src: url,
			type: getType(rawPath),
			direction: (pa.ad?.direction ?? "bottom") as Direction,
			position: pa.ad?.position ?? 50,
		};
	}

	// ── Auto-ad scheduler ────────────────────────────────────────────────────────
	// Each ProgramAd with type 'automatic' or 'both' gets its own timer that
	// re-schedules itself after each fire. Jitter of ±25 % prevents clustering
	// while keeping the average rate equal to the configured frequency/hr.
	function scheduleAutoAd(pa: import("$lib/types").ProgramAd): void {
		const base = 3_600_000 / Math.max(pa.frequency, 1);
		const delay = base * (0.75 + Math.random() * 0.5);
		const handle = setTimeout(async () => {
			autoAdTimers.delete(pa.id);
			if (!currentProgram) return;

			if (overlayActive) {
				// Overlay is live – skip this occurrence but reschedule
				scheduleAutoAd(pa);
				return;
			}

			if (adIsVisible && !fillerIsActive) {
				// A real/manual ad is playing – retry in 5 s
				const h2 = setTimeout(() => {
					autoAdTimers.delete(pa.id);
					scheduleAutoAd(pa);
				}, 5_000);
				autoAdTimers.set(pa.id, h2);
				return;
			}

			// Override any active filler
			if (fillerIsActive) stopFillers();

			fillerSuppressed = true;
			const payload = buildAdPayload(pa);
			if (payload) {
				triggerAd(payload);
				await sleep(pa.duration * 1_000);
				triggerAd(null);
				await sleep(AD_SLIDE_MS + 100);
			}
			fillerSuppressed = false;

			startFillersIfNeeded();
			if (currentProgram) scheduleAutoAd(pa); // schedule next occurrence
		}, delay);
		autoAdTimers.set(pa.id, handle);
	}

	function startAutoAds(program: Program): void {
		stopAutoAds();
		for (const pa of program.program_ads) {
			if (
				(pa.ad_launch_type === "automatic" || pa.ad_launch_type === "both") &&
				pa.frequency > 0 &&
				pa.ad?.image_path
			) {
				scheduleAutoAd(pa);
			}
		}
	}

	function stopAutoAds(): void {
		for (const h of autoAdTimers.values()) clearTimeout(h);
		autoAdTimers.clear();
	}

	// ── Filler rotator ───────────────────────────────────────────────────────────
	// Filler ads play whenever nothing else is on screen, rotating every 30 s
	// when multiple fillers are configured.
	function playFillerAt(ads: import("$lib/types").ProgramAd[], index: number): void {
		const payload = buildAdPayload(ads[index]);
		if (payload) triggerAd(payload);
	}

	function startFillersIfNeeded(): void {
		if (fillerSuppressed || overlayActive || adIsVisible || fillerIsActive) return;
		const ads = (currentProgram?.program_ads ?? []).filter(
			(pa) => pa.ad_launch_type === "filler" && pa.ad?.image_path,
		);
		if (ads.length === 0) return;

		fillerIndex = 0;
		playFillerAt(ads, 0);
		fillerIsActive = true;

		if (ads.length > 1) {
			fillerRotationTimer = setInterval(() => {
				if (fillerSuppressed || overlayActive) {
					stopFillers();
					triggerAd(null);
					return;
				}
				fillerIndex = (fillerIndex + 1) % ads.length;
				playFillerAt(ads, fillerIndex);
			}, 30_000);
		}
	}

	function stopFillers(): void {
		if (fillerRotationTimer !== null) {
			clearInterval(fillerRotationTimer);
			fillerRotationTimer = null;
		}
		fillerIsActive = false;
		fillerIndex = 0;
	}

	function stopAllScheduling(): void {
		stopAutoAds();
		stopFillers();
		fillerSuppressed = false;
		overlayActive = false;
		fillerIsActive = false;
		// Dismiss any locally-managed ad (filler/auto) that may still be
		// visible.  triggerAd's coalescing queue ensures that if a new ad
		// is triggered right after, the latest request wins.
		triggerAd(null);
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
			stopAllScheduling();
			currentProgram = data.program;

			// Restore whatever is currently live so a page load / reconnect is not blank
			if (data.activeOverlay) {
				const allowAds = data.activeOverlay.allowAds ?? false;
				overlayActive = !allowAds;
				fillerSuppressed = !allowAds;
				const rawPath = data.activeOverlay.graphicPath ?? null;
				const url = rawPath ? imgUrl(rawPath) : null;
				const type = getType(rawPath);
				if (url) preload(url, type);
				show(url, type);
			} else if (data.activeAd) {
				fillerSuppressed = true;
				const rawPath = data.activeAd.imagePath ?? null;
				const url = rawPath ? imgUrl(rawPath) : null;
				const type = getType(rawPath);
				if (url) preload(url, type);
				show(url, type);
				if (url) {
					triggerAd({
						src: url,
						type,
						direction: (data.activeAd.direction ?? "bottom") as Direction,
						position: data.activeAd.position ?? 50,
					});
				}
			} else {
				show(null, "image");
			}

			// Always start auto-ad schedulers; fillers only when nothing else is active
			if (currentProgram) startAutoAds(currentProgram);
			startFillersIfNeeded();
		}

		function onProgramSelected(data: any) {
			if (Number(data.studioId) !== studioId) return;
			preloadProgram(data.program as Program);
			stopAllScheduling();
			currentProgram = data.program as Program;
			if (currentProgram) {
				startAutoAds(currentProgram);
				startFillersIfNeeded();
			}
		}

		function onProgramCleared(data: any) {
			if (Number(data.studioId) !== studioId) return;
			show(null, "image");
			stopAllScheduling();
			currentProgram = null;
			triggerAd(null);
		}

		function onOverlayActivated(data: any) {
			if (Number(data.studioId) !== studioId) return;
			const rawPath: string | null = data.graphicPath ?? null;
			const url = rawPath ? imgUrl(rawPath) : null;
			const type = getType(rawPath);
			if (url) preload(url, type);
			show(url, type);

			const allowAds = data.allowAds ?? false;
			overlayActive = !allowAds;

			if (!allowAds) {
				// Overlay does not allow ads – suppress everything
				fillerSuppressed = true;
				stopFillers();
				triggerAd(null);
			} else {
				// Overlay allows ads – let fillers / auto ads keep running
				fillerSuppressed = false;
				startFillersIfNeeded();
			}
		}

		function onOverlayDeactivated(data: any) {
			if (data && Number(data.studioId) !== studioId) return;
			show(null, "image");
			overlayActive = false;
			fillerSuppressed = false;
			startFillersIfNeeded();
		}

		function onAdStarted(data: any) {
			if (Number(data.studioId) !== studioId) return;
			const rawPath: string | null = data.imagePath ?? null;
			const url = rawPath ? imgUrl(rawPath) : null;
			if (!url) return;
			const type = getType(rawPath);
			fillerSuppressed = true;
			stopFillers();
			triggerAd({
				src: url,
				type,
				direction: (data.direction ?? "bottom") as Direction,
				position: data.position ?? 50,
			});
		}

		function onAdEnded(data: any) {
			if (data && Number(data.studioId) !== studioId) return;
			triggerAd(null);
			fillerSuppressed = false;
			if (!overlayActive) startFillersIfNeeded();
		}

		// ── Data-change listeners ────────────────────────────────────────────
		// When ads, screens, or programs are edited the server broadcasts
		// update-* events to all clients.  Re-requesting studio state causes
		// onStudioState to run again, which re-preloads any assets whose URLs
		// may have changed, keeping the local HTTP cache warm and correct.
		function onUpdateData() {
			socket.emit("get-studio-state", { studioId });
		}

		socket.on("studio-state", onStudioState);
		socket.on("program-selected", onProgramSelected);
		socket.on("program-cleared", onProgramCleared);
		socket.on("overlay-activated", onOverlayActivated);
		socket.on("overlay-deactivated", onOverlayDeactivated);
		socket.on("ad-started", onAdStarted);
		socket.on("ad-ended", onAdEnded);
		socket.on("update-ads", onUpdateData);
		socket.on("update-programs", onUpdateData);
		socket.on("update-screens", onUpdateData);

		return () => {
			stopAllScheduling();
			socket.emit("leave-studio-room", { studioId });
			socket.off("studio-state", onStudioState);
			socket.off("program-selected", onProgramSelected);
			socket.off("program-cleared", onProgramCleared);
			socket.off("overlay-activated", onOverlayActivated);
			socket.off("overlay-deactivated", onOverlayDeactivated);
			socket.off("ad-started", onAdStarted);
			socket.off("ad-ended", onAdEnded);
			socket.off("update-ads", onUpdateData);
			socket.off("update-programs", onUpdateData);
			socket.off("update-screens", onUpdateData);
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
	<img
		class="overlay-media overlay-media-img"
		class:active={displayedType === "image"}
		src={displayedType === "image" ? (displayedPath ?? "") : ""}
		alt="overlay"
	/>
	<video
		class="overlay-media overlay-media-vid"
		class:active={displayedType === "video"}
		src={displayedType === "video" ? (displayedPath ?? "") : ""}
		autoplay
		loop
		muted
	></video>
</div>

<!-- Ad overlay: sized, positioned, and animated entirely via direct DOM manipulation. -->
<!-- svelte-ignore a11y_media_has_caption -->
<div bind:this={adContainer} class="ad-overlay-root" style="display:none">
	<img bind:this={adImg} alt="ad" style="display:none;width:100%;height:100%" />
	<video
		bind:this={adVideo}
		autoplay
		loop
		muted
		style="display:none;width:100%;height:100%"
	></video>
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
		display: none;
	}

	.overlay-media.active {
		display: block;
	}

	.ad-overlay-root {
		position: fixed;
		pointer-events: none;
		z-index: 10;
	}
</style>
