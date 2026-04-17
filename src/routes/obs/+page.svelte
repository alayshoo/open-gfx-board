<script lang="ts">
	import { onMount } from "svelte";
	import { socket } from "$lib/api/socket";
	import { imgUrl } from "$lib/api/api";
	import type { Program, StudioState } from "$lib/types";

	type Direction = "top" | "bottom" | "left" | "right";
	type MediaType = "image" | "video" | "html";

	// ── Constants ──────────────────────────────────────────────────────────────
	const FADE_MS = 500;
	const POPUP_SLIDE_MS = 1000;
	const HTML_POPUP_DEFAULT_WIDTH = 640;
	const HTML_POPUP_DEFAULT_HEIGHT = 360;
	const NUM_LAYERS = 3;

	// ── Overlay reactive state – one entry per layer (index 0 = layer 1 = top) ─
	type LayerOverlayState = {
		visible: boolean;
		path: string | null;
		type: MediaType;
		html: string | null;
	};

	let layers = $state<LayerOverlayState[]>([
		{ visible: false, path: null, type: "image", html: null },
		{ visible: false, path: null, type: "image", html: null },
		{ visible: false, path: null, type: "image", html: null },
	]);

	// ── Media cache ─────────────────────────────────────────────────────────────
	const preloaded = new Set<string>();

	function getType(rawPath: string | null): MediaType {
		const ext = rawPath?.split(".").pop()?.toLowerCase() ?? "";
		return ["mp4", "webm"].includes(ext) ? "video" : "image";
	}

	function preload(url: string, type: MediaType) {
		if (type === "html" || preloaded.has(url)) return;
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

	function preloadProgram(program: Program | null) {
		if (!program) return;
		for (const screen of program.graphics ?? []) {
			if (screen.media_type === "html") continue;
			const url = imgUrl(screen.graphics_path);
			if (url) preload(url, getType(screen.graphics_path));
		}
		const bg = imgUrl(program.background_graphics_path);
		if (bg) preload(bg, getType(program.background_graphics_path));
	}

	// ── Per-layer overlay element refs ──────────────────────────────────────────
	// Svelte 5: bind:this requires individual variables, not arrays.
	let overlayImg0: HTMLImageElement;
	let overlayImg1: HTMLImageElement;
	let overlayImg2: HTMLImageElement;
	let overlayVid0: HTMLVideoElement;
	let overlayVid1: HTMLVideoElement;
	let overlayVid2: HTMLVideoElement;

	function getOverlayImg(li: number): HTMLImageElement {
		return [overlayImg0, overlayImg1, overlayImg2][li];
	}
	function getOverlayVid(li: number): HTMLVideoElement {
		return [overlayVid0, overlayVid1, overlayVid2][li];
	}

	// ── Per-layer overlay animation engine ──────────────────────────────────────
	// Non-reactive: animation sequencing must not be interrupted by reactivity.
	const overlayBusy: boolean[] = [false, false, false];
	const overlayPending: ({ path: string | null; type: MediaType; html: string | null } | null)[] = [null, null, null];

	function sleep(ms: number): Promise<void> {
		return new Promise((r) => setTimeout(r, ms));
	}

	function waitForOverlayMedia(li: number, src: string, type: MediaType): Promise<void> {
		return new Promise((resolve) => {
			if (type === "html") { resolve(); return; }
			if (type === "image") {
				const img = getOverlayImg(li);
				if (!img || !img.src) { resolve(); return; }
				img.decode().then(resolve, resolve);
			} else {
				const vid = getOverlayVid(li);
				if (!vid) { resolve(); return; }
				if (vid.readyState >= 3) { resolve(); }
				else { vid.addEventListener("canplay", () => resolve(), { once: true }); }
			}
		});
	}

	async function show(li: number, path: string | null, type: MediaType, html: string | null = null) {
		overlayPending[li] = { path, type, html };
		if (overlayBusy[li]) return;
		overlayBusy[li] = true;

		while (overlayPending[li]) {
			const target = overlayPending[li]!;
			overlayPending[li] = null;

			if (layers[li].visible) {
				layers[li] = { ...layers[li], visible: false };
				await sleep(FADE_MS);
			}

			layers[li] = { visible: false, path: target.path, type: target.type, html: target.html };

			const hasContent = target.type === "html" ? target.html !== null : target.path !== null;
			if (hasContent) {
				await waitForOverlayMedia(li, target.path ?? "", target.type);
				if (!overlayPending[li]) {
					layers[li] = { ...layers[li], visible: true };
				}
			}
		}

		overlayBusy[li] = false;
	}

	// ── Per-layer popup element refs ─────────────────────────────────────────────
	let popupContainer0: HTMLDivElement;
	let popupContainer1: HTMLDivElement;
	let popupContainer2: HTMLDivElement;
	let popupImg0: HTMLImageElement;
	let popupImg1: HTMLImageElement;
	let popupImg2: HTMLImageElement;
	let popupVideo0: HTMLVideoElement;
	let popupVideo1: HTMLVideoElement;
	let popupVideo2: HTMLVideoElement;
	let popupIframe0: HTMLIFrameElement;
	let popupIframe1: HTMLIFrameElement;
	let popupIframe2: HTMLIFrameElement;

	function getPopupContainer(li: number): HTMLDivElement {
		return [popupContainer0, popupContainer1, popupContainer2][li];
	}
	function getPopupImg(li: number): HTMLImageElement {
		return [popupImg0, popupImg1, popupImg2][li];
	}
	function getPopupVideo(li: number): HTMLVideoElement {
		return [popupVideo0, popupVideo1, popupVideo2][li];
	}
	function getPopupIframe(li: number): HTMLIFrameElement {
		return [popupIframe0, popupIframe1, popupIframe2][li];
	}

	// ── Popup animation engine ───────────────────────────────────────────────────
	type PopUpPayload = {
		src: string | null;
		type: MediaType;
		html: string | null;
		direction: Direction;
		position: number;
		width: number | null;
		height: number | null;
	} | null;

	const popupBusy: boolean[] = [false, false, false];
	const popupPending: ({ payload: PopUpPayload } | null)[] = [null, null, null];
	const popupCurrentDirection: Direction[] = ["bottom", "bottom", "bottom"];
	const popupIsVisible: boolean[] = [false, false, false];

	function popupSlideOutTransform(dir: Direction): string {
		if (dir === "bottom") return "translateY(118%)";
		if (dir === "top") return "translateY(-118%)";
		if (dir === "left") return "translateX(-118%)";
		return "translateX(118%)";
	}

	function waitForPopUpMedia(li: number, type: MediaType): Promise<void> {
		return new Promise((resolve) => {
			if (type === "html") { resolve(); return; }
			const img = getPopupImg(li);
			const vid = getPopupVideo(li);
			if (type === "image") {
				if (img.complete && img.naturalWidth > 0) { resolve(); return; }
				img.addEventListener("load", () => resolve(), { once: true });
				img.addEventListener("error", () => resolve(), { once: true });
			} else {
				if (vid.readyState >= 3) { resolve(); return; }
				vid.addEventListener("canplay", () => resolve(), { once: true });
				vid.addEventListener("error", () => resolve(), { once: true });
			}
		});
	}

	function hideAllPopupMedia(li: number) {
		const img = getPopupImg(li);
		const vid = getPopupVideo(li);
		const ifr = getPopupIframe(li);
		img.src = ""; img.style.display = "none";
		vid.src = ""; vid.load(); vid.style.display = "none";
		ifr.srcdoc = ""; ifr.style.display = "none";
	}

	async function triggerPopUp(li: number, payload: PopUpPayload) {
		popupPending[li] = { payload };
		if (popupBusy[li]) return;
		popupBusy[li] = true;

		while (popupPending[li]) {
			const { payload: target } = popupPending[li]!;
			popupPending[li] = null;

			const container = getPopupContainer(li);

			// Step 1: slide out current popup if visible
			if (popupIsVisible[li]) {
				popupIsVisible[li] = false;
				container.style.transition = `transform ${POPUP_SLIDE_MS}ms cubic-bezier(0.4,0,0.2,1)`;
				container.style.transform = popupSlideOutTransform(popupCurrentDirection[li]);
				await sleep(POPUP_SLIDE_MS);
				hideAllPopupMedia(li);
				container.style.display = "none";
			}

			if (!target) continue;
			if (popupPending[li]) continue;

			// Step 2: set up new popup media (hidden, off-screen)
			popupCurrentDirection[li] = target.direction;
			const img = getPopupImg(li);
			const vid = getPopupVideo(li);
			const ifr = getPopupIframe(li);

			if (target.type === "html") {
				img.style.display = "none";
				vid.style.display = "none";
				ifr.style.display = "block";
				ifr.srcdoc = target.html ?? "";
			} else if (target.type === "video") {
				img.style.display = "none";
				ifr.style.display = "none";
				vid.style.display = "block";
				vid.src = target.src ?? "";
				vid.load();
			} else {
				vid.style.display = "none";
				ifr.style.display = "none";
				img.style.display = "block";
				img.src = target.src ?? "";
			}

			container.style.transition = "none";
			container.style.transform = popupSlideOutTransform(target.direction);
			container.style.display = "block";

			await waitForPopUpMedia(li, target.type);

			if (popupPending[li]) {
				hideAllPopupMedia(li);
				container.style.display = "none";
				continue;
			}

			// Step 3: size and position the container
			let naturalW: number;
			let naturalH: number;

			if (target.type === "html") {
				naturalW = target.width ?? HTML_POPUP_DEFAULT_WIDTH;
				naturalH = target.height ?? HTML_POPUP_DEFAULT_HEIGHT;
			} else if (target.type === "image") {
				naturalW = target.width ?? img.naturalWidth;
				naturalH = target.height ?? img.naturalHeight;
			} else {
				naturalW = target.width ?? vid.videoWidth;
				naturalH = target.height ?? vid.videoHeight;
			}

			const halfW = naturalW / 2;
			const halfH = naturalH / 2;
			const pos = target.position;

			container.style.width = `${naturalW}px`;
			container.style.height = `${naturalH}px`;
			container.style.top = "";
			container.style.bottom = "";
			container.style.left = "";
			container.style.right = "";

			if (target.direction === "bottom") {
				container.style.bottom = "30px";
				container.style.left = `calc(${pos}% - ${halfW}px)`;
			} else if (target.direction === "top") {
				container.style.top = "30px";
				container.style.left = `calc(${pos}% - ${halfW}px)`;
			} else if (target.direction === "left") {
				container.style.left = "30px";
				container.style.top = `calc(${pos}% - ${halfH}px)`;
			} else {
				container.style.right = "30px";
				container.style.top = `calc(${pos}% - ${halfH}px)`;
			}

			// Step 4: snap to off-screen start, then slide in
			void container.offsetHeight; // force reflow
			container.style.transition = `transform ${POPUP_SLIDE_MS}ms cubic-bezier(0.4,0,0.2,1)`;
			container.style.transform = "translate(0,0)";
			popupIsVisible[li] = true;

			await sleep(POPUP_SLIDE_MS);
		}

		popupBusy[li] = false;
	}

	// ── Popup payload builder ─────────────────────────────────────────────────────
	function buildPopUpPayload(pa: import("$lib/types").ProgramPopUp): PopUpPayload | null {
		const popup = pa.popup;
		if (!popup) return null;
		const w = popup.width ?? null;
		const h = popup.height ?? null;
		if (popup.media_type === "html") {
			if (!popup.html_content) return null;
			return {
				src: null, type: "html", html: popup.html_content,
				direction: (popup.direction ?? "bottom") as Direction,
				position: popup.position ?? 50, width: w, height: h,
			};
		}
		const rawPath = popup.image_path ?? null;
		const url = rawPath ? imgUrl(rawPath) : null;
		if (!url) return null;
		return {
			src: url, type: getType(rawPath), html: null,
			direction: (popup.direction ?? "bottom") as Direction,
			position: popup.position ?? 50, width: w, height: h,
		};
	}

	// ── Auto pop-up & filler scheduling state ────────────────────────────────────
	let currentProgram: Program | null = null;
	const overlayActive: boolean[] = [false, false, false];
	const fillerIsActive: boolean[] = [false, false, false];
	const fillerSuppressed: boolean[] = [false, false, false];
	const autoPopUpTimers = new Map<number, ReturnType<typeof setTimeout>>();
	const fillerRotationTimers: (ReturnType<typeof setInterval> | null)[] = [null, null, null];
	const fillerIndices: number[] = [0, 0, 0];

	function scheduleAutoPopUp(pa: import("$lib/types").ProgramPopUp): void {
		const li = ((pa.layer ?? 1) - 1);
		const base = 3_600_000 / Math.max(pa.frequency, 1);
		const delay = base * (0.75 + Math.random() * 0.5);
		const handle = setTimeout(async () => {
			autoPopUpTimers.delete(pa.id);
			if (!currentProgram) return;
			if (overlayActive.some(v => v)) { scheduleAutoPopUp(pa); return; }
			if (popupIsVisible[li] && !fillerIsActive[li]) {
				const h2 = setTimeout(() => {
					autoPopUpTimers.delete(pa.id);
					scheduleAutoPopUp(pa);
				}, 5_000);
				autoPopUpTimers.set(pa.id, h2);
				return;
			}
			if (fillerIsActive[li]) stopFillers(li);
			fillerSuppressed[li] = true;
			const payload = buildPopUpPayload(pa);
			if (payload) {
				await triggerPopUp(li, payload);
				await sleep(pa.duration * 1_000);
				triggerPopUp(li, null);
				await sleep(POPUP_SLIDE_MS + 100);
			}
			fillerSuppressed[li] = false;
			startFillersIfNeeded(li);
			if (currentProgram) scheduleAutoPopUp(pa);
		}, delay);
		autoPopUpTimers.set(pa.id, handle);
	}

	function startAutoPopUps(program: Program): void {
		stopAutoPopUps();
		for (const pa of program.program_popups) {
			const hasContent = pa.popup?.media_type === "html" ? !!pa.popup?.html_content : !!pa.popup?.image_path;
			if ((pa.popup_launch_type === "automatic" || pa.popup_launch_type === "both") && pa.frequency > 0 && hasContent) {
				scheduleAutoPopUp(pa);
			}
		}
	}

	function stopAutoPopUps(): void {
		for (const h of autoPopUpTimers.values()) clearTimeout(h);
		autoPopUpTimers.clear();
	}

	// ── Filler rotator ───────────────────────────────────────────────────────────
	function playFillerAt(popups: import("$lib/types").ProgramPopUp[], index: number, li: number): void {
		const payload = buildPopUpPayload(popups[index]);
		if (payload) triggerPopUp(li, payload);
	}

	function startFillersIfNeeded(li: number): void {
		if (fillerSuppressed[li] || overlayActive.some(v => v) || popupIsVisible[li] || fillerIsActive[li]) return;
		const popups = (currentProgram?.program_popups ?? []).filter((pa) => {
			const sameLayer = (pa.layer ?? 1) - 1 === li;
			const hasContent = pa.popup?.media_type === "html" ? !!pa.popup?.html_content : !!pa.popup?.image_path;
			return pa.popup_launch_type === "filler" && hasContent && sameLayer;
		});
		if (popups.length === 0) return;
		fillerIndices[li] = 0;
		playFillerAt(popups, 0, li);
		fillerIsActive[li] = true;
		if (popups.length > 1) {
			fillerRotationTimers[li] = setInterval(() => {
				if (fillerSuppressed[li] || overlayActive.some(v => v)) {
					stopFillers(li);
					triggerPopUp(li, null);
					return;
				}
				fillerIndices[li] = (fillerIndices[li] + 1) % popups.length;
				playFillerAt(popups, fillerIndices[li], li);
			}, 30_000);
		}
	}

	function stopFillers(li: number): void {
		if (fillerRotationTimers[li] !== null) {
			clearInterval(fillerRotationTimers[li]!);
			fillerRotationTimers[li] = null;
		}
		fillerIsActive[li] = false;
		fillerIndices[li] = 0;
	}

	function stopAllScheduling(): void {
		stopAutoPopUps();
		for (let li = 0; li < NUM_LAYERS; li++) {
			stopFillers(li);
			fillerSuppressed[li] = false;
			overlayActive[li] = false;
			fillerIsActive[li] = false;
			triggerPopUp(li, null);
		}
	}

	// ── Socket ─────────────────────────────────────────────────────────────────
	onMount(() => {
		socket.emit("join-studio-room", {});
		socket.emit("get-studio-state", {});

		function onReconnect() {
			socket.emit("join-studio-room", {});
			socket.emit("get-studio-state", {});
		}
		socket.on("connect", onReconnect);

		function onStudioState(data: StudioState) {
			preloadProgram(data.program);
			stopAllScheduling();
			currentProgram = data.program;

			// Blank all overlay layers first — any layer absent from activeOverlays must be cleared
			for (let li = 0; li < NUM_LAYERS; li++) {
				show(li, null, "image");
			}

			// Restore active overlays on each layer
			for (const overlay of data.activeOverlays ?? []) {
				const li = (overlay.layer ?? 1) - 1;
				const allowPopUps = overlay.allowPopUps ?? false;
				overlayActive[li] = !allowPopUps;
				const mediaType = (overlay.mediaType ?? "image") as MediaType;
				if (mediaType === "html") {
					show(li, null, "html", overlay.htmlContent ?? null);
				} else {
					const rawPath = overlay.graphicPath ?? null;
					const url = rawPath ? imgUrl(rawPath) : null;
					const type = getType(rawPath);
					if (url) preload(url, type);
					show(li, url, type);
				}
			}
			// Popups are suppressed globally if any active screen forbids them
			const stateBlocked = overlayActive.some(v => v);
			for (let li = 0; li < NUM_LAYERS; li++) {
				fillerSuppressed[li] = stateBlocked;
			}

			// Restore active popups on each layer
			for (const popup of data.activePopUps ?? []) {
				const li = (popup.layer ?? 1) - 1;
				fillerSuppressed[li] = true;
				const mediaType = (popup.mediaType ?? "image") as MediaType;
				if (mediaType === "html") {
					const html = popup.htmlContent ?? null;
					if (html) triggerPopUp(li, { src: null, type: "html", html, direction: (popup.direction ?? "bottom") as Direction, position: popup.position ?? 50, width: popup.width ?? null, height: popup.height ?? null });
				} else {
					const rawPath = popup.imagePath ?? null;
					const url = rawPath ? imgUrl(rawPath) : null;
					const type = getType(rawPath);
					if (url) preload(url, type);
					if (url) triggerPopUp(li, { src: url, type, html: null, direction: (popup.direction ?? "bottom") as Direction, position: popup.position ?? 50, width: popup.width ?? null, height: popup.height ?? null });
				}
			}

			if (currentProgram) startAutoPopUps(currentProgram);
			for (let li = 0; li < NUM_LAYERS; li++) {
				const li_ = li;
				setTimeout(() => startFillersIfNeeded(li_), POPUP_SLIDE_MS + 100);
			}
		}

		function onProgramSelected(data: any) {
			preloadProgram(data.program as Program);
			stopAllScheduling();
			for (let li = 0; li < NUM_LAYERS; li++) show(li, null, "image");
			currentProgram = data.program as Program;
			if (currentProgram) {
				startAutoPopUps(currentProgram);
				for (let li = 0; li < NUM_LAYERS; li++) {
					const li_ = li;
					setTimeout(() => startFillersIfNeeded(li_), POPUP_SLIDE_MS + 100);
				}
			}
		}

		function onProgramCleared(_data: any) {
			for (let li = 0; li < NUM_LAYERS; li++) show(li, null, "image");
			stopAllScheduling();
			currentProgram = null;
			for (let li = 0; li < NUM_LAYERS; li++) triggerPopUp(li, null);
		}

		function onOverlayActivated(data: any) {
			const li = ((data.layer ?? 1) - 1);
			const mediaType = (data.mediaType ?? "image") as MediaType;
			const allowPopUps = data.allowPopUps ?? false;

			if (mediaType === "html") {
				show(li, null, "html", data.htmlContent ?? null);
			} else {
				const rawPath: string | null = data.graphicPath ?? null;
				const url = rawPath ? imgUrl(rawPath) : null;
				const type = getType(rawPath);
				if (url) preload(url, type);
				show(li, url, type);
			}

			overlayActive[li] = !allowPopUps;
			// Re-evaluate globally: any active screen forbidding popups blocks all layers
			const activationBlocked = overlayActive.some(v => v);
			for (let l = 0; l < NUM_LAYERS; l++) {
				if (activationBlocked) {
					fillerSuppressed[l] = true;
					stopFillers(l);
					triggerPopUp(l, null);
				} else {
					fillerSuppressed[l] = false;
					startFillersIfNeeded(l);
				}
			}
		}

		function onOverlayDeactivated(data: any) {
			const li = ((data.layer ?? 1) - 1);
			show(li, null, "image");
			overlayActive[li] = false;
			// Re-evaluate globally: only unblock popups if no other layer is still forbidding them
			const deactivationBlocked = overlayActive.some(v => v);
			for (let l = 0; l < NUM_LAYERS; l++) {
				if (!deactivationBlocked) {
					fillerSuppressed[l] = false;
					startFillersIfNeeded(l);
				}
			}
		}

		function onPopUpStarted(data: any) {
			const li = ((data.layer ?? 1) - 1);
			const mediaType = (data.mediaType ?? "image") as MediaType;
			const popupW: number | null = data.width ?? null;
			const popupH: number | null = data.height ?? null;

			if (mediaType === "html") {
				const html: string | null = data.htmlContent ?? null;
				if (!html) return;
				fillerSuppressed[li] = true;
				stopFillers(li);
				triggerPopUp(li, { src: null, type: "html", html, direction: (data.direction ?? "bottom") as Direction, position: data.position ?? 50, width: popupW, height: popupH });
			} else {
				const rawPath: string | null = data.imagePath ?? null;
				const url = rawPath ? imgUrl(rawPath) : null;
				if (!url) return;
				const type = getType(rawPath);
				fillerSuppressed[li] = true;
				stopFillers(li);
				triggerPopUp(li, { src: url, type, html: null, direction: (data.direction ?? "bottom") as Direction, position: data.position ?? 50, width: popupW, height: popupH });
			}
		}

		function onPopUpEnded(data: any) {
			const li = ((data.layer ?? 1) - 1);
			triggerPopUp(li, null);
			fillerSuppressed[li] = false;
			if (!overlayActive.some(v => v)) setTimeout(() => startFillersIfNeeded(li), POPUP_SLIDE_MS + 100);
		}

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
			socket.off("connect", onReconnect);
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
		html, body {
			margin: 0;
			padding: 0;
			background: transparent !important;
			overflow: hidden;
		}
	</style>
</svelte:head>

<!--
	Layers are rendered bottom-to-top in the DOM so that higher z-index
	values correctly sit on top of lower ones visually.
	  Layer 3 (bottom)  z-index: 10 (overlay), 11 (popup)
	  Layer 2 (middle)  z-index: 20 (overlay), 21 (popup)
	  Layer 1 (top)     z-index: 30 (overlay), 31 (popup)
-->

<!-- ── Layer 3 overlay (bottom) ───────────────────────────── -->
<div class="overlay-root" class:visible={layers[2].visible} style="z-index:10">
	<img
		bind:this={overlayImg2}
		class="overlay-media overlay-media-img"
		class:active={layers[2].type === "image"}
		src={layers[2].type === "image" ? (layers[2].path ?? "") : ""}
		alt="overlay L3"
	/>
	<video
		bind:this={overlayVid2}
		class="overlay-media overlay-media-vid"
		class:active={layers[2].type === "video"}
		src={layers[2].type === "video" ? (layers[2].path ?? "") : ""}
		autoplay loop muted
	></video>
	<iframe
		class="overlay-media overlay-media-html"
		class:active={layers[2].type === "html"}
		srcdoc={layers[2].type === "html" ? (layers[2].html ?? "") : ""}
		sandbox="allow-scripts allow-same-origin"
		title="HTML overlay L3"
	></iframe>
</div>

<!-- ── Layer 2 overlay (middle) ───────────────────────────── -->
<div class="overlay-root" class:visible={layers[1].visible} style="z-index:20">
	<img
		bind:this={overlayImg1}
		class="overlay-media overlay-media-img"
		class:active={layers[1].type === "image"}
		src={layers[1].type === "image" ? (layers[1].path ?? "") : ""}
		alt="overlay L2"
	/>
	<video
		bind:this={overlayVid1}
		class="overlay-media overlay-media-vid"
		class:active={layers[1].type === "video"}
		src={layers[1].type === "video" ? (layers[1].path ?? "") : ""}
		autoplay loop muted
	></video>
	<iframe
		class="overlay-media overlay-media-html"
		class:active={layers[1].type === "html"}
		srcdoc={layers[1].type === "html" ? (layers[1].html ?? "") : ""}
		sandbox="allow-scripts allow-same-origin"
		title="HTML overlay L2"
	></iframe>
</div>

<!-- ── Layer 1 overlay (top) ──────────────────────────────── -->
<div class="overlay-root" class:visible={layers[0].visible} style="z-index:30">
	<img
		bind:this={overlayImg0}
		class="overlay-media overlay-media-img"
		class:active={layers[0].type === "image"}
		src={layers[0].type === "image" ? (layers[0].path ?? "") : ""}
		alt="overlay L1"
	/>
	<video
		bind:this={overlayVid0}
		class="overlay-media overlay-media-vid"
		class:active={layers[0].type === "video"}
		src={layers[0].type === "video" ? (layers[0].path ?? "") : ""}
		autoplay loop muted
	></video>
	<iframe
		class="overlay-media overlay-media-html"
		class:active={layers[0].type === "html"}
		srcdoc={layers[0].type === "html" ? (layers[0].html ?? "") : ""}
		sandbox="allow-scripts allow-same-origin"
		title="HTML overlay L1"
	></iframe>
</div>

<!-- ── Popup overlays: bottom to top ──────────────────────── -->

<!-- svelte-ignore a11y_media_has_caption -->
<div bind:this={popupContainer2} class="popup-overlay-root" style="display:none;z-index:11">
	<img bind:this={popupImg2} alt="popup L3" style="display:none;width:100%;height:100%" />
	<video bind:this={popupVideo2} autoplay loop muted style="display:none;width:100%;height:100%"></video>
	<iframe bind:this={popupIframe2} sandbox="allow-scripts allow-same-origin" title="HTML popup L3" style="display:none;width:100%;height:100%;border:none"></iframe>
</div>

<!-- svelte-ignore a11y_media_has_caption -->
<div bind:this={popupContainer1} class="popup-overlay-root" style="display:none;z-index:21">
	<img bind:this={popupImg1} alt="popup L2" style="display:none;width:100%;height:100%" />
	<video bind:this={popupVideo1} autoplay loop muted style="display:none;width:100%;height:100%"></video>
	<iframe bind:this={popupIframe1} sandbox="allow-scripts allow-same-origin" title="HTML popup L2" style="display:none;width:100%;height:100%;border:none"></iframe>
</div>

<!-- svelte-ignore a11y_media_has_caption -->
<div bind:this={popupContainer0} class="popup-overlay-root" style="display:none;z-index:31">
	<img bind:this={popupImg0} alt="popup L1" style="display:none;width:100%;height:100%" />
	<video bind:this={popupVideo0} autoplay loop muted style="display:none;width:100%;height:100%"></video>
	<iframe bind:this={popupIframe0} sandbox="allow-scripts allow-same-origin" title="HTML popup L1" style="display:none;width:100%;height:100%;border:none"></iframe>
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

	.overlay-media-html {
		border: none;
		background: transparent;
	}

	.popup-overlay-root {
		position: fixed;
		pointer-events: none;
	}
</style>
