<script lang="ts">
	import { onMount } from "svelte";
	import { socket } from "$lib/api/socket";
	import { imgUrl } from "$lib/api/api";
	import type { Program, StudioState } from "$lib/types";

	type Direction = "top" | "bottom" | "left" | "right";

	// ── Displayed state ────────────────────────────────────────────────────────
	let displayedPath = $state<string | null>(null);
	let displayedType = $state<"image" | "video">("image");
	let visible = $state(false);

	// Must stay in sync with the CSS transition duration below.
	const FADE_MS = 500;
	const POPUP_SLIDE_MS = 1000;

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

	// ── PopUp overlay – direct DOM management ───────────────────────────────────
	// All pop-up animation is handled imperatively to avoid reactive overhead and
	// to guarantee correct sequencing (hide current → load next → slide in) with
	// no visual glitches regardless of how rapidly events arrive.
	let popupContainer: HTMLDivElement;
	let popupImg: HTMLImageElement;
	let popupVideo: HTMLVideoElement;

	type PopUpPayload = {
		src: string;
		type: "image" | "video";
		direction: Direction;
		position: number;
	} | null;

	// Coalescing queue – only the most-recent pending request is kept so rapid
	// pop-up changes never queue up a backlog of animations.
	let popupPending: { payload: PopUpPayload } | null = null;
	let popupBusy = false;
	let popupCurrentDirection: Direction = "bottom";
	let popupIsVisible = false;

	// ── Auto pop-up & filler scheduling state ────────────────────────────────────
	let currentProgram: Program | null = null;
	let overlayActive = false;
	let fillerIsActive = false;
	let fillerSuppressed = false;
	let autoPopUpTimers = new Map<number, ReturnType<typeof setTimeout>>();
	let fillerRotationTimer: ReturnType<typeof setInterval> | null = null;
	let fillerIndex = 0;

	function popupSlideOutTransform(dir: Direction): string {
		if (dir === "bottom") return "translateY(118%)";
		if (dir === "top") return "translateY(-118%)";
		if (dir === "left") return "translateX(-118%)";
		return "translateX(118%)";
	}

	function waitForPopUpMedia(type: "image" | "video"): Promise<void> {
		return new Promise((resolve) => {
			if (type === "image") {
				if (popupImg.complete && popupImg.naturalWidth > 0) {
					resolve();
					return;
				}
				popupImg.addEventListener("load", () => resolve(), { once: true });
				popupImg.addEventListener("error", () => resolve(), { once: true });
			} else {
				if (popupVideo.readyState >= 3) {
					resolve();
					return;
				}
				popupVideo.addEventListener("canplay", () => resolve(), { once: true });
				popupVideo.addEventListener("error", () => resolve(), { once: true });
			}
		});
	}

	async function triggerPopUp(payload: PopUpPayload) {
		popupPending = { payload };
		if (popupBusy) return;
		popupBusy = true;

		while (popupPending) {
			const { payload: target } = popupPending;
			popupPending = null;

			// ── Step 1: slide out the current pop-up if one is visible ──────────
			if (popupIsVisible) {
				popupIsVisible = false;
				popupContainer.style.transition = `transform ${POPUP_SLIDE_MS}ms cubic-bezier(0.4,0,0.2,1)`;
				popupContainer.style.transform = popupSlideOutTransform(popupCurrentDirection);
				await sleep(POPUP_SLIDE_MS);
				// Clear media after slide-out completes
				popupImg.src = "";
				popupImg.style.display = "none";
				popupVideo.src = "";
				popupVideo.load();
				popupVideo.style.display = "none";
				popupContainer.style.display = "none";
			}

			if (!target) continue; // Pure hide request – nothing more to do

			if (popupPending) continue; // A newer request arrived while hiding – restart

			// ── Step 2: set up the new pop-up media (hidden, off-screen) ────────
			popupCurrentDirection = target.direction;

			if (target.type === "video") {
				popupImg.style.display = "none";
				popupVideo.style.display = "block";
				popupVideo.src = target.src;
				popupVideo.load();
			} else {
				popupVideo.style.display = "none";
				popupImg.style.display = "block";
				popupImg.src = target.src;
			}

			// Place the container off-screen in the incoming direction with no
			// transition so there is no visible flash while the media loads.
			popupContainer.style.transition = "none";
			popupContainer.style.transform = popupSlideOutTransform(target.direction);
			popupContainer.style.display = "block";

			await waitForPopUpMedia(target.type);

			if (popupPending) {
				// Superseded while loading – tear down and restart
				popupImg.src = "";
				popupImg.style.display = "none";
				popupVideo.src = "";
				popupVideo.load();
				popupVideo.style.display = "none";
				popupContainer.style.display = "none";
				continue;
			}

			// ── Step 3: size and position the container ──────────────────────
			const naturalW =
				target.type === "image" ? popupImg.naturalWidth : popupVideo.videoWidth;
			const naturalH =
				target.type === "image" ? popupImg.naturalHeight : popupVideo.videoHeight;
			const halfW = naturalW / 2;
			const halfH = naturalH / 2;
			const pos = target.position;

			popupContainer.style.width = `${naturalW}px`;
			popupContainer.style.height = `${naturalH}px`;
			popupContainer.style.top = "";
			popupContainer.style.bottom = "";
			popupContainer.style.left = "";
			popupContainer.style.right = "";

			if (target.direction === "bottom") {
				popupContainer.style.bottom = "30px";
				popupContainer.style.left = `calc(${pos}% - ${halfW}px)`;
			} else if (target.direction === "top") {
				popupContainer.style.top = "30px";
				popupContainer.style.left = `calc(${pos}% - ${halfW}px)`;
			} else if (target.direction === "left") {
				popupContainer.style.left = "30px";
				popupContainer.style.top = `calc(${pos}% - ${halfH}px)`;
			} else {
				popupContainer.style.right = "30px";
				popupContainer.style.top = `calc(${pos}% - ${halfH}px)`;
			}

			// ── Step 4: snap to off-screen start position, then slide in ────
			// transition is already "none" and transform is already the slide-out
			// value from step 2, so we just need a reflow to commit that state
			// before re-enabling the transition and moving to translate(0,0).
			void popupContainer.offsetHeight; // force reflow

			popupContainer.style.transition = `transform ${POPUP_SLIDE_MS}ms cubic-bezier(0.4,0,0.2,1)`;
			popupContainer.style.transform = "translate(0,0)";
			popupIsVisible = true;

			await sleep(POPUP_SLIDE_MS); // wait for slide-in to finish
		}

		popupBusy = false;
	}

	// ── PopUp payload builder ────────────────────────────────────────────────────
	function buildPopUpPayload(pa: import("$lib/types").ProgramPopUp): PopUpPayload | null {
		const rawPath = pa.popup?.image_path ?? null;
		const url = rawPath ? imgUrl(rawPath) : null;
		if (!url) return null;
		return {
			src: url,
			type: getType(rawPath),
			direction: (pa.popup?.direction ?? "bottom") as Direction,
			position: pa.popup?.position ?? 50,
		};
	}

	// ── Auto pop-up scheduler ────────────────────────────────────────────────────
	// Each ProgramPopUp with type 'automatic' or 'both' gets its own timer that
	// re-schedules itself after each fire. Jitter of ±25 % prevents clustering
	// while keeping the average rate equal to the configured frequency/hr.
	function scheduleAutoPopUp(pa: import("$lib/types").ProgramPopUp): void {
		const base = 3_600_000 / Math.max(pa.frequency, 1);
		const delay = base * (0.75 + Math.random() * 0.5);
		const handle = setTimeout(async () => {
			autoPopUpTimers.delete(pa.id);
			if (!currentProgram) return;

			if (overlayActive) {
				// Overlay is live – skip this occurrence but reschedule
				scheduleAutoPopUp(pa);
				return;
			}

			if (popupIsVisible && !fillerIsActive) {
				// A real/manual pop-up is playing – retry in 5 s
				const h2 = setTimeout(() => {
					autoPopUpTimers.delete(pa.id);
					scheduleAutoPopUp(pa);
				}, 5_000);
				autoPopUpTimers.set(pa.id, h2);
				return;
			}

			// Override any active filler
			if (fillerIsActive) stopFillers();

			fillerSuppressed = true;
			const payload = buildPopUpPayload(pa);
			if (payload) {
				await triggerPopUp(payload); // wait for slide-in to complete before counting duration
				await sleep(pa.duration * 1_000);
				triggerPopUp(null);
				await sleep(POPUP_SLIDE_MS + 100);
			}
			fillerSuppressed = false;

			startFillersIfNeeded();
			if (currentProgram) scheduleAutoPopUp(pa); // schedule next occurrence
		}, delay);
		autoPopUpTimers.set(pa.id, handle);
	}

	function startAutoPopUps(program: Program): void {
		stopAutoPopUps();
		for (const pa of program.program_popups) {
			if (
				(pa.popup_launch_type === "automatic" || pa.popup_launch_type === "both") &&
				pa.frequency > 0 &&
				pa.popup?.image_path
			) {
				scheduleAutoPopUp(pa);
			}
		}
	}

	function stopAutoPopUps(): void {
		for (const h of autoPopUpTimers.values()) clearTimeout(h);
		autoPopUpTimers.clear();
	}

	// ── Filler rotator ───────────────────────────────────────────────────────────
	// Filler pop-ups play whenever nothing else is on screen, rotating every 30 s
	// when multiple fillers are configured.
	function playFillerAt(popups: import("$lib/types").ProgramPopUp[], index: number): void {
		const payload = buildPopUpPayload(popups[index]);
		if (payload) triggerPopUp(payload);
	}

	function startFillersIfNeeded(): void {
		if (fillerSuppressed || overlayActive || popupIsVisible || fillerIsActive) return;
		const popups = (currentProgram?.program_popups ?? []).filter(
			(pa) => pa.popup_launch_type === "filler" && pa.popup?.image_path,
		);
		if (popups.length === 0) return;

		fillerIndex = 0;
		playFillerAt(popups, 0);
		fillerIsActive = true;

		if (popups.length > 1) {
			fillerRotationTimer = setInterval(() => {
				if (fillerSuppressed || overlayActive) {
					stopFillers();
					triggerPopUp(null);
					return;
				}
				fillerIndex = (fillerIndex + 1) % popups.length;
				playFillerAt(popups, fillerIndex);
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
		stopAutoPopUps();
		stopFillers();
		fillerSuppressed = false;
		overlayActive = false;
		fillerIsActive = false;
		// Dismiss any locally-managed pop-up (filler/auto) that may still be
		// visible. triggerPopUp's coalescing queue ensures that if a new pop-up
		// is triggered right after, the latest request wins.
		triggerPopUp(null);
	}

	// ── Socket ─────────────────────────────────────────────────────────────────
	onMount(() => {
		socket.emit("join-studio-room", {});
		// Request the current studio state so we can warm the cache immediately.
		socket.emit("get-studio-state", {});

		function onStudioState(data: StudioState) {
			preloadProgram(data.program);
			stopAllScheduling();
			currentProgram = data.program;

			// Restore whatever is currently live so a page load / reconnect is not blank
			if (data.activeOverlay) {
				const allowPopUps = data.activeOverlay.allowPopUps ?? false;
				overlayActive = !allowPopUps;
				fillerSuppressed = !allowPopUps;
				const rawPath = data.activeOverlay.graphicPath ?? null;
				const url = rawPath ? imgUrl(rawPath) : null;
				const type = getType(rawPath);
				if (url) preload(url, type);
				show(url, type);
			} else if (data.activePopUp) {
				fillerSuppressed = true;
				const rawPath = data.activePopUp.imagePath ?? null;
				const url = rawPath ? imgUrl(rawPath) : null;
				const type = getType(rawPath);
				if (url) preload(url, type);
				if (url) {
					triggerPopUp({
						src: url,
						type,
						direction: (data.activePopUp.direction ?? "bottom") as Direction,
						position: data.activePopUp.position ?? 50,
					});
				}
			} else {
				show(null, "image");
			}

			// Always start auto pop-up schedulers; fillers only when nothing else is active.
			// Delay startFillersIfNeeded so any ongoing popup slide-out animation from
			// stopAllScheduling() has time to complete before we check popupIsVisible.
			if (currentProgram) startAutoPopUps(currentProgram);
			setTimeout(() => startFillersIfNeeded(), POPUP_SLIDE_MS + 100);
		}

		function onProgramSelected(data: any) {
			preloadProgram(data.program as Program);
			stopAllScheduling();
			currentProgram = data.program as Program;
			if (currentProgram) {
				startAutoPopUps(currentProgram);
				// Delay until any slide-out animation from stopAllScheduling finishes.
				setTimeout(() => startFillersIfNeeded(), POPUP_SLIDE_MS + 100);
			}
		}

		function onProgramCleared(_data: any) {
			show(null, "image");
			stopAllScheduling();
			currentProgram = null;
			triggerPopUp(null);
		}

		function onOverlayActivated(data: any) {
			const rawPath: string | null = data.graphicPath ?? null;
			const url = rawPath ? imgUrl(rawPath) : null;
			const type = getType(rawPath);
			if (url) preload(url, type);
			show(url, type);

			const allowPopUps = data.allowPopUps ?? false;
			overlayActive = !allowPopUps;

			if (!allowPopUps) {
				// Overlay does not allow pop-ups – suppress everything
				fillerSuppressed = true;
				stopFillers();
				triggerPopUp(null);
			} else {
				// Overlay allows pop-ups – let fillers / auto pop-ups keep running
				fillerSuppressed = false;
				startFillersIfNeeded();
			}
		}

		function onOverlayDeactivated(_data: any) {
			show(null, "image");
			overlayActive = false;
			fillerSuppressed = false;
			startFillersIfNeeded();
		}

		function onPopUpStarted(data: any) {
			const rawPath: string | null = data.imagePath ?? null;
			const url = rawPath ? imgUrl(rawPath) : null;
			if (!url) return;
			const type = getType(rawPath);
			fillerSuppressed = true;
			stopFillers();
			triggerPopUp({
				src: url,
				type,
				direction: (data.direction ?? "bottom") as Direction,
				position: data.position ?? 50,
			});
		}

		function onPopUpEnded(_data: any) {
			triggerPopUp(null);
			fillerSuppressed = false;
			// Delay until the slide-out animation finishes so popupIsVisible is false
			// before startFillersIfNeeded checks it.
			if (!overlayActive) setTimeout(() => startFillersIfNeeded(), POPUP_SLIDE_MS + 100);
		}

		// ── Data-change listeners ────────────────────────────────────────────
		// When pop-ups, screens, or programs are edited the server broadcasts
		// update-* events to all clients.  Re-requesting studio state causes
		// onStudioState to run again, which re-preloads any assets whose URLs
		// may have changed, keeping the local HTTP cache warm and correct.
		function onUpdateData() {
			socket.emit("get-studio-state", {});
		}

		socket.on("studio-state", onStudioState);
		socket.on("program-selected", onProgramSelected);
		socket.on("program-cleared", onProgramCleared);
		socket.on("overlay-activated", onOverlayActivated);
		socket.on("overlay-deactivated", onOverlayDeactivated);
		socket.on("popup-started", onPopUpStarted);
		socket.on("popup-ended", onPopUpEnded);
		socket.on("update-popups", onUpdateData);
		socket.on("update-programs", onUpdateData);
		socket.on("update-screens", onUpdateData);

		return () => {
			stopAllScheduling();
			socket.emit("leave-studio-room", {});
			socket.off("studio-state", onStudioState);
			socket.off("program-selected", onProgramSelected);
			socket.off("program-cleared", onProgramCleared);
			socket.off("overlay-activated", onOverlayActivated);
			socket.off("overlay-deactivated", onOverlayDeactivated);
			socket.off("popup-started", onPopUpStarted);
			socket.off("popup-ended", onPopUpEnded);
			socket.off("update-popups", onUpdateData);
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

<!-- PopUp overlay: sized, positioned, and animated entirely via direct DOM manipulation. -->
<!-- svelte-ignore a11y_media_has_caption -->
<div bind:this={popupContainer} class="popup-overlay-root" style="display:none">
	<img bind:this={popupImg} alt="popup" style="display:none;width:100%;height:100%" />
	<video
		bind:this={popupVideo}
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

	.popup-overlay-root {
		position: fixed;
		pointer-events: none;
		z-index: 10;
	}
</style>
