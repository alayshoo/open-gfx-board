<script lang="ts">
	import { onMount } from "svelte";
	import { page } from "$app/stores";
	import CmdPanel from "$lib/components/Cmd-Panel.svelte";
	import ScreenSelector from "$lib/components/ScreensSelector.svelte";
	import PopUpLauncher from "$lib/components/PopUpLauncher.svelte";
	import StatusDot from "$lib/components/StatusDot.svelte";
	import PluginHost from "$lib/components/PluginHost.svelte";
	import { socket, connected, BACKEND_URL } from "$lib/api/socket";
	import { imgUrl } from "$lib/api/api";
	import { fetchPlugins, fetchPluginManifest } from "$lib/api/plugins";
	import { fetchProgramPluginIds } from "$lib/programPlugins";
	import { addToast } from "$lib/toasts";
	import type {
		Program,
		StudioState,
		ObsCommand,
		Graphic,
		ProgramPopUp,
		ActivePopUp,
		PluginInfo,
		PluginManifest,
	} from "$lib/types";
	import MediaPreview from "$lib/components/MediaPreview.svelte";
    import { IS_TAURI } from "$lib/bridge";

	const presetId = $derived(
		Number($page.url.searchParams.get("preset")) || null,
	);

	let program = $state<Program | null>(null);
	// Per-layer active state (key = layer number 1|2|3)
	let activeScreenIds = $state<Record<number, number | null>>({ 1: null, 2: null, 3: null });
	let activePopUpIds = $state<Record<number, number | null>>({ 1: null, 2: null, 3: null });
	let studioCommands = $state<ObsCommand[]>([]);
	let isPopUpPlaying = $state<Record<number, boolean>>({ 1: false, 2: false, 3: false });
	let popupEndTimers: Record<number, ReturnType<typeof setTimeout> | null> = { 1: null, 2: null, 3: null };

	// Plugin state
	let controlPlugins = $state<PluginInfo[]>([]);
	let pluginManifests = $state<Record<string, PluginManifest>>({});

	// Per-program plugin filter: fetched from the server so it is shared across
	// all clients. null = no program loaded; [] = program loaded, no plugins enabled.
	let programPluginFilter = $state<string[] | null>(null);

	$effect(() => {
		const currentProgram = program;
		if (!currentProgram) {
			programPluginFilter = null;
			return;
		}
		const pid = currentProgram.id;
		fetchProgramPluginIds(pid).then((ids) => {
			// Guard against a stale response if program changed while fetching
			if (program?.id === pid) {
				programPluginFilter = ids;
			}
		});
	});

	const visibleControlPlugins = $derived(
		programPluginFilter === null
			? []
			: controlPlugins.filter((p) => programPluginFilter!.includes(p.id))
	);

	const screens = $derived<Graphic[]>(program?.graphics ?? []);
	const programPopUps = $derived<ProgramPopUp[]>(program?.program_popups ?? []);
	// Per-layer: true when that layer's active screen allows popups
	const allowPopUpsPerLayer = $derived<Record<number, boolean>>({
		1: activeScreenIds[1] !== null && (program?.graphics.find((g) => g.id === activeScreenIds[1])?.allow_popups ?? false),
		2: activeScreenIds[2] !== null && (program?.graphics.find((g) => g.id === activeScreenIds[2])?.allow_popups ?? false),
		3: activeScreenIds[3] !== null && (program?.graphics.find((g) => g.id === activeScreenIds[3])?.allow_popups ?? false),
	});
	const logoUrl = $derived(imgUrl(program?.logo_path));

	onMount(() => {
		socket.emit("join-studio-room", {});
		socket.emit("get-studio-state", {});

		// Re-join the studio room after a socket reconnect so we keep receiving
		// broadcasts (plugin-state-updated, overlay-activated, etc.).  Without
		// this, a server restart or network drop silently breaks all live updates.
		function onReconnect() {
			socket.emit("join-studio-room", {});
			socket.emit("get-studio-state", {});
		}
		socket.on("connect", onReconnect);

		// Named handlers prevent removing every global listener on cleanup
		function onStudioState(data: StudioState) {
			program = data.program;
			// Reset all layers
			activeScreenIds = { 1: null, 2: null, 3: null };
			activePopUpIds = { 1: null, 2: null, 3: null };
			isPopUpPlaying = { 1: false, 2: false, 3: false };
			for (const overlay of data.activeOverlays ?? []) {
				activeScreenIds = { ...activeScreenIds, [overlay.layer]: overlay.graphicId };
			}
			for (const popup of data.activePopUps ?? []) {
				activePopUpIds = { ...activePopUpIds, [popup.layer]: popup.popupId };
				isPopUpPlaying = { ...isPopUpPlaying, [popup.layer]: true };
			}
		}

		function onProgramSelected(data: any) {
			program = data.program;
			activeScreenIds = { 1: null, 2: null, 3: null };
			activePopUpIds = { 1: null, 2: null, 3: null };
			isPopUpPlaying = { 1: false, 2: false, 3: false };
			// Restore active overlays from the payload if provided
			for (const overlay of data.activeOverlays ?? []) {
				activeScreenIds = { ...activeScreenIds, [overlay.layer]: overlay.graphicId };
			}
			// Auto-activate first screen if nothing is active on layer 1
			if (!activeScreenIds[1] && program?.graphics && program.graphics.length > 0) {
				triggerOverlay(program.graphics[0]);
			}
		}

		function onProgramCleared(_data: any) {
			program = null;
			activeScreenIds = { 1: null, 2: null, 3: null };
			activePopUpIds = { 1: null, 2: null, 3: null };
			isPopUpPlaying = { 1: false, 2: false, 3: false };
		}

		function onOverlayActivated(data: any) {
			const layer = data.layer ?? 1;
			activeScreenIds = { ...activeScreenIds, [layer]: data.graphicId };
		}

		function onOverlayDeactivated(data: any) {
			const layer = data.layer ?? 1;
			activeScreenIds = { ...activeScreenIds, [layer]: null };
		}

		function onPopUpStarted(data: any) {
			const layer: number = data.layer ?? 1;
			isPopUpPlaying = { ...isPopUpPlaying, [layer]: true };
			activePopUpIds = { ...activePopUpIds, [layer]: data.popupId };
			if (popupEndTimers[layer]) {
				clearTimeout(popupEndTimers[layer]!);
				popupEndTimers[layer] = null;
			}
			const dur: number = data.duration ?? 0;
			if (dur > 0) {
				popupEndTimers[layer] = setTimeout(() => {
					socket.emit("end-popup", { layer });
					popupEndTimers[layer] = null;
				}, dur * 1000);
			}
		}

		function onPopUpEnded(data: any) {
			const layer: number = data.layer ?? 1;
			isPopUpPlaying = { ...isPopUpPlaying, [layer]: false };
			activePopUpIds = { ...activePopUpIds, [layer]: null };
			if (popupEndTimers[layer]) {
				clearTimeout(popupEndTimers[layer]!);
				popupEndTimers[layer] = null;
			}
		}

		// ── Data-change listeners ─────────────────
		function onUpdateData() {
			socket.emit("get-studio-state", {});
		}

		function fetchStudioCommands() {
			fetch(`${BACKEND_URL}/studios`)
				.then((r) => r.json())
				.then((d) => {
					const studio = d.studios?.[0];
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
		}

		function onUpdateStudios() {
			// Preset / command names may have changed – re-fetch studio list
			fetchStudioCommands();
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
		socket.on("update-studios", onUpdateStudios);

		// Fetch studio commands for selected preset
		fetchStudioCommands();

		// Load enabled plugins with control components
		fetchPlugins().then(async (allPlugins) => {
			const withControl = allPlugins.filter((p) => p.enabled && p.has_control);
			for (const p of withControl) {
				try {
					pluginManifests[p.id] = await fetchPluginManifest(p.id);
				} catch { /* skip */ }
			}
			controlPlugins = withControl.filter((p) => pluginManifests[p.id]);
		}).catch(() => {});

		return () => {
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
			socket.off("update-studios", onUpdateStudios);
			socket.emit("leave-studio-room", {});
			for (const t of Object.values(popupEndTimers)) { if (t) clearTimeout(t); }
		};
	});

	function triggerOverlay(graphic: Graphic) {
		const layer = graphic.layer ?? 1;
		if (activeScreenIds[layer] === graphic.id) {
			socket.emit("deactivate-overlay", { layer });
		} else {
			socket.emit("trigger-overlay", {
				programId: program!.id,
				graphicId: graphic.id,
				graphicPath: graphic.graphics_path,
				allowPopUps: graphic.allow_popups,
				layer,
			});
		}
	}

	function triggerPopUp(pa: ProgramPopUp) {
		const layer = pa.layer ?? 1;
		if (popupEndTimers[layer]) {
			clearTimeout(popupEndTimers[layer]!);
			popupEndTimers[layer] = null;
		}
		if (activePopUpIds[layer] === pa.popup_id) {
			socket.emit("end-popup", { layer });
		} else {
			socket.emit("trigger-popup", {
				programId: program!.id,
				popupId: pa.popup_id,
				duration: pa.duration,
				layer,
			});
		}
	}

	function triggerCommand(cmd: ObsCommand) {
		socket.emit("trigger-obs-command", {
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
		splitPct = Math.min(Math.max(pct, 25), 75); // clamp between 20% and 80%
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
				href="/program-selector"
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
				<div class="header-stack">
					<StatusDot connected={$connected} />
					<a class="nav-link" href="/">Settings</a>
				</div>
				{#if !IS_TAURI}
					<button class="nav-link fullscreen-btn" onclick={toggleFullscreen} title="Fullscreen">
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
							<polyline points="15 3 21 3 21 9"></polyline>
							<polyline points="9 21 3 21 3 15"></polyline>
							<line x1="21" y1="3" x2="14" y2="10"></line>
							<line x1="3" y1="21" x2="10" y2="14"></line>
						</svg>
					</button>
				{/if}
			</div>
		</header>

		<!-- Screens -->
		<section class="panel-section screens-section">
			<ScreenSelector
				{screens}
				{activeScreenIds}
				onTrigger={triggerOverlay}
			/>
		</section>

		<!-- PopUps -->
		<section class="panel-section popup-section">
			<PopUpLauncher
				{programPopUps}
				{activePopUpIds}
				allowPopUpsPerLayer={allowPopUpsPerLayer}
				onTrigger={triggerPopUp}
			/>
		</section>

		<!-- Plugin control sections -->
		{#each visibleControlPlugins as plugin (plugin.id)}
			{#if pluginManifests[plugin.id]}
				<section class="panel-section plugin-section">
					<div class="plugin-section-header">
						<span class="plugin-section-title">{plugin.name}</span>
					</div>
					<PluginHost
						pluginId={plugin.id}
						componentType="control"
						manifest={pluginManifests[plugin.id]}
					/>
				</section>
			{/if}
		{/each}
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
		/* Move scrollbar to the left so it doesn't crowd the resize divider */
		direction: rtl;
	}

	.main-panel > * {
		direction: ltr;
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
		flex-direction: row;
		align-items: stretch;
		gap: 8px;
		flex-shrink: 0;
	}

	.header-stack {
		display: flex;
		flex-direction: column;
		align-items: stretch;
		gap: 8px;
	}

	.nav-link {
		display: inline-flex;
		align-items: center;
		justify-content: center;
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

	.fullscreen-btn {
		align-self: stretch;
		padding: 8px 12px;
	}

	.fullscreen-btn svg {
		width: clamp(12px, 3cqi, 18px);
		height: clamp(12px, 3cqi, 18px);
		flex-shrink: 0;
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

	/* Plugin sections */
	.plugin-section-header {
		margin-bottom: 8px;
	}

	.plugin-section-title {
		font-size: 0.75rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--text-3);
	}
</style>
