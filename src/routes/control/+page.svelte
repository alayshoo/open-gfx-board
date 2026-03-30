<script lang="ts">
	import { onMount } from "svelte";
	import { page } from "$app/stores";
	import { goto } from "$app/navigation";
	import CmdPanel from "$lib/components/Cmd-Panel.svelte";
	import ScreenSelector from "$lib/components/GraphicsSelector.svelte";
	import AdLauncher from "$lib/components/AdLauncher.svelte";
	import StatusDot from "$lib/components/StatusDot.svelte";
	import { socket, connected, BACKEND_URL } from "$lib/api/socket";
	import { imgUrl } from "$lib/api/api";
	import { addToast } from "$lib/toasts";
	import type {
		Program,
		StudioState,
		ObsCommand,
		Graphic,
		ProgramAd,
		ActiveAd,
	} from "$lib/types";
	import MediaPreview from "$lib/components/MediaPreview.svelte";
    import { IS_TAURI } from "$lib/bridge";

	const studioId = $derived(Number($page.url.searchParams.get("studio")));
	const presetId = $derived(
		Number($page.url.searchParams.get("preset")) || null,
	);

	let program = $state<Program | null>(null);
	let activeGraphicId = $state<number | null>(null);
	let activeAdId = $state<number | null>(null);
	let studioCommands = $state<ObsCommand[]>([]);
	let isAdPlaying = $state(false);
	let adEndTimer: ReturnType<typeof setTimeout> | null = null;

	const graphics = $derived<Graphic[]>(program?.graphics ?? []);
	const programAds = $derived<ProgramAd[]>(program?.program_ads ?? []);
	const allowAdsMode = $derived(
		activeGraphicId !== null &&
			(program?.graphics.find((g) => g.id === activeGraphicId)
				?.allow_ads ??
				false),
	);
	const logoUrl = $derived(imgUrl(program?.logo_path));

	onMount(() => {
		if (!studioId) {
			goto("/");
			return;
		}

		socket.emit("join-studio-room", { studioId });
		socket.emit("get-studio-state", { studioId });

		// Named handlers prevent removing every global listener on cleanup
		function onStudioState(data: StudioState) {
			if (data.studioId !== studioId) return;
			program = data.program;
			activeGraphicId = data.activeOverlay?.graphicId ?? null;
			if (data.activeAd) {
				isAdPlaying = true;
				activeAdId = data.activeAd.adId;
			} else {
				isAdPlaying = false;
				activeAdId = null;
			}
		}

		function onProgramSelected(data: any) {
			if (data.studioId !== studioId) return;
			program = data.program;
			activeGraphicId = data.activeOverlay?.graphicId ?? null;
			isAdPlaying = false;
			activeAdId = null;

			// Auto-activate first graphic if no overlay is active and program has graphics
			if (
				!activeGraphicId &&
				program?.graphics &&
				program.graphics.length > 0
			) {
				triggerOverlay(program.graphics[0]);
			}
		}

		function onProgramCleared(data: any) {
			if (data.studioId !== studioId) return;
			program = null;
			activeGraphicId = null;
			isAdPlaying = false;
			activeAdId = null;
		}

		function onOverlayActivated(data: any) {
			if (data.studioId !== studioId) return;
			activeGraphicId = data.graphicId;
		}

		function onOverlayDeactivated(data: any) {
			if (data.studioId !== studioId) return;
			activeGraphicId = null;
		}

		function onAdStarted(data: any) {
			if (data.studioId !== studioId) return;
			isAdPlaying = true;
			activeAdId = data.adId;
		}

		function onAdEnded(data: any) {
			if (data.studioId !== studioId) return;
			isAdPlaying = false;
			activeAdId = null;
		}

		socket.on("studio-state", onStudioState);
		socket.on("program-selected", onProgramSelected);
		socket.on("program-cleared", onProgramCleared);
		socket.on("overlay-activated", onOverlayActivated);
		socket.on("overlay-deactivated", onOverlayDeactivated);
		socket.on("ad-started", onAdStarted);
		socket.on("ad-ended", onAdEnded);

		// Fetch studio commands for selected preset
		fetch(`${BACKEND_URL}/studios`)
			.then((r) => r.json())
			.then((d) => {
				const studio = d.studios?.find((s: any) => s.id === studioId);
				if (studio) {
					if (presetId) {
						const preset = studio.presets?.find(
							(p: any) => p.id === presetId,
						);
						studioCommands = preset?.commands ?? [];
					} else {
						studioCommands = studio.commands ?? [];
					}
				}
			});

		return () => {
			socket.off("studio-state", onStudioState);
			socket.off("program-selected", onProgramSelected);
			socket.off("program-cleared", onProgramCleared);
			socket.off("overlay-activated", onOverlayActivated);
			socket.off("overlay-deactivated", onOverlayDeactivated);
			socket.off("ad-started", onAdStarted);
			socket.off("ad-ended", onAdEnded);
			socket.emit("leave-studio-room", { studioId });
			if (adEndTimer) clearTimeout(adEndTimer);
		};
	});

	function triggerOverlay(graphic: Graphic) {
		if (activeGraphicId === graphic.id) {
			// Clicking the active overlay deactivates it
			socket.emit("deactivate-overlay", { studioId });
		} else {
			socket.emit("trigger-overlay", {
				studioId,
				programId: program!.id,
				graphicId: graphic.id,
				graphicPath: graphic.graphics_path,
				allowAds: graphic.allow_ads,
			});
		}
	}

	function triggerAd(pa: ProgramAd) {
		if (adEndTimer) {
			clearTimeout(adEndTimer);
			adEndTimer = null;
		}

		if (activeAdId === pa.ad_id) {
			// Clicking the active ad stops it early
			socket.emit("end-ad", { studioId });
		} else {
			socket.emit("trigger-ad", {
				studioId,
				programId: program!.id,
				adId: pa.ad_id,
				imagePath: pa.ad?.image_path,
				duration: pa.duration,
			});
			// Auto-end after the ad's duration has elapsed
			adEndTimer = setTimeout(() => {
				socket.emit("end-ad", { studioId });
				adEndTimer = null;
			}, pa.duration * 1000);
		}
	}

	function triggerCommand(cmd: ObsCommand) {
		socket.emit("trigger-obs-command", {
			studioId,
			commandId: cmd.id,
			shortcut: cmd.obs_command_shortcut,
		});
	}

	/* Resizing the windows */

	let splitPct = $state(50); // percentage for left panel
	let dragging = $state(false);

	function onDragStart(e: PointerEvent) {
		dragging = true;
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
	}

	function onDragMove(e: PointerEvent) {
		if (!dragging) return;
		const pct = (e.clientX / window.innerWidth) * 100;
		splitPct = Math.min(Math.max(pct, 20), 80); // clamp between 20% and 80%
	}

	function onDragEnd() {
		dragging = false;
	}

	/* Fullscreen */

	function toggleFullscreen() {
		const doc = document.documentElement;

		if (!document.fullscreenElement) {
			// Enter fullscreen
			if (doc.requestFullscreen) {
				doc.requestFullscreen();
			}
		} else {
			// Exit fullscreen
			if (document.exitFullscreen) {
				document.exitFullscreen();
			}
		}
	}
</script>

<div class="layout">
	<!-- LEFT: Main panel -->
	<div class="main-panel" style="width: {splitPct}%">
		<!-- Header -->
		<header class="header">
			<a
				class="program-selector"
				href="/program-selector?studio={studioId}"
			>
				{#if logoUrl}
					<MediaPreview
						class="prog-logo"
						src={logoUrl}
						alt={program?.name}
					/>
				{:else}
					<div class="prog-info">
						<span class="prog-name"
							>{program?.name ?? "No program selected"}</span
						>
					</div>
				{/if}
				<svg
					class="chevron-down"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2.5"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<polyline points="6 9 12 15 18 9"></polyline>
				</svg>
			</a>

			<div class="header-right">
				<StatusDot connected={$connected} />
				<div class="header-links">
					{#if !IS_TAURI}
						<button class="nav-link" onclick={toggleFullscreen}>Fullscreen</button>
					{/if}
					<a class="nav-link" href="/">Switch Preset</a>
				</div>
			</div>
		</header>

		<!-- Graphics -->
		<section class="panel-section screens-section">
			<ScreenSelector
				{graphics}
				{activeGraphicId}
				onTrigger={triggerOverlay}
			/>
		</section>

		<!-- Ads -->
		<section class="panel-section ad-section">
			<AdLauncher
				{programAds}
				{activeAdId}
				{allowAdsMode}
				onTrigger={triggerAd}
			/>
		</section>
	</div>

	<!-- Dragable divider -->
	<div
		class="divider"
		class:dragging
		onpointerdown={onDragStart}
		onpointermove={onDragMove}
		onpointerup={onDragEnd}
		role="separator"
		aria-valuenow={splitPct}
	>
		<span>.</span><span>.</span><span>.</span>
	</div>

	<!-- RIGHT: Command panel -->
	<div class="cmd-panel" style="width: {100 - splitPct}%">
		<CmdPanel commands={studioCommands} onCommand={triggerCommand} />
	</div>
</div>

<style>
	.layout {
		display: flex;
		width: 100vw;
		height: 100%;
		overflow: hidden;
		background: var(--bg);
		position: relative;
	}

	.main-panel {
		display: flex;
		flex-direction: column;
		flex-shrink: 0;
		height: 100%;
		box-sizing: border-box;
		padding: 10px;
		gap: 20px;
		overflow-y: auto;
		overflow-x: hidden;
		container-type: inline-size;
	}

	/* Header */
	.header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		min-height: 60px;
		gap: 10px;
		flex-shrink: 0;
	}

	.program-selector {
		display: flex;
		align-items: center;
		gap: clamp(8px, 2cqi, 16px);
		text-decoration: none;
		min-width: 0;
		padding: clamp(6px, 1.5cqi, 12px) clamp(10px, 2cqi, 20px);
		background: var(--surface-2);
		border: 1px solid var(--border-1);
		border-radius: 12px;
		transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
	}

	.program-selector:hover {
		background: var(--surface-3);
		border-color: var(--border-2);
		transform: translateY(-1px);
	}

	.chevron-down {
		width: clamp(14px, 3.5cqi, 28px);
		height: clamp(14px, 3.5cqi, 28px);
		color: var(--text-3);
		opacity: 0.6;
		flex-shrink: 0;
		transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
	}

	.program-selector:hover .chevron-down {
		opacity: 1;
		transform: translateY(2px);
		color: var(--text-1);
	}

	:global(.prog-logo) {
		width: clamp(30px, 8cqi, 60px);
		height: clamp(30px, 8cqi, 60px);
		object-fit: contain;
		flex-shrink: 0;
	}

	.prog-info {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}

	.prog-name {
		font-size: clamp(1.1rem, 5.5cqi, 2.2rem);
		font-weight: 600;
		color: var(--text-1);
		line-height: 1.1;
		word-break: break-word;
	}

	.header-right {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 12px;
		flex-shrink: 0;
	}

	.header-links {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		justify-content: flex-end;
	}

	.nav-link {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		font-size: clamp(0.3rem, 4.5cqi, 0.6rem);
		color: var(--text-3);
		text-decoration: none;
		padding: 8px 14px;
		border-radius: 8px;
		background: var(--surface-2);
		border: 1px solid var(--border-1);
		transition: all 0.15s;
	}

	.nav-link:hover {
		color: var(--text-1);
		border-color: var(--border-2);
	}

	/* Sections */
	.panel-section {
		display: flex;
		flex-direction: column;
		background: var(--surface-1);
		border: 1px solid var(--border-1);
		border-radius: var(--r-lg);
		padding: 16px;
		flex-shrink: 0;
		min-height: min-content;
	}

	.divider {
		display: flex;
		flex-direction: column;
		justify-content: center;
		align-items: center;

		width: 5px;
		height: 100%;

		font-size: 36px;
		font-weight: 600;
		line-height: 50%;

		cursor: col-resize;
		color: var(--border-2);
		flex-shrink: 0;
		transition: color 0.15s;
		touch-action: none;
	}

	.divider:hover,
	.divider.dragging {
		color: var(--border-3);
	}

	/* Right panel */
	.cmd-panel {
		position: relative;
		top: unset;
		right: unset;
		flex-shrink: 0;
		height: 100%;
		box-sizing: border-box;
		padding: 10px;
	}
</style>
