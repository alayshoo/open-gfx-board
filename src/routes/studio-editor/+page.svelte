<script lang="ts">
	import { onMount } from 'svelte';
	import TopNav from '$lib/components/TitleBarWeb.svelte';
	import { socket } from '$lib/api/socket';
	import { fetchStudios } from '$lib/api/api';
	import { addToast } from '$lib/toasts';
	import type { Studio, ObsCommand, Preset } from '$lib/types';
	import { getBackendUrl } from '$lib/bridge';
	import { IS_TAURI } from '$lib/bridge';

	const SHORTCUTS = ['F13','F14','F15','F16','F17','F18','F19','F20','F21','F22','F23','F24'];

	let studio = $state<Studio | null>(null);
	let saving = $state(false);
	let copied = $state(false);

	// Preset selection (by index in the presets array)
	let selectedPresetIdx = $state<number | null>(null);
	let isCreatingPreset = $state(false);

	// Edit state for the currently selected preset
	let editPresetName = $state('');
	let editCommands = $state<ObsCommand[]>([]);

	// Studio name edit
	let editStudioName = $state('');
	let savingStudioName = $state(false);

	const allPresets = $derived(studio?.presets ?? []);
	const selectedPreset = $derived(
		selectedPresetIdx !== null ? allPresets[selectedPresetIdx] ?? null : null
	);
	const hasSelection = $derived(selectedPresetIdx !== null || isCreatingPreset);

	onMount(() => {
		loadData();

		socket.on('update-studios', () => loadData());

		return () => {
			socket.off('update-studios');
		};
	});

	async function loadData() {
		const data = await fetchStudios();
		const s = data[0] ?? null;
		studio = s;
		editStudioName = s?.name ?? '';
		// Clear selection if selected preset no longer exists
		if (selectedPresetIdx !== null && selectedPresetIdx >= (s?.presets?.length ?? 0)) {
			selectedPresetIdx = null;
		}
	}

	function selectPreset(idx: number) {
		isCreatingPreset = false;
		selectedPresetIdx = idx;
		const p = allPresets[idx];
		editPresetName = p.name;
		editCommands = p.commands.map((c) => ({ ...c }));
	}

	function openNewPreset() {
		isCreatingPreset = true;
		selectedPresetIdx = null;
		editPresetName = `Preset ${allPresets.length + 1}`;
		editCommands = [];
	}

	function addCommand() {
		const newCmd: ObsCommand = {
			id: null,
			obs_command_name: 'New Command',
			obs_command_color: '#38bdf8',
			obs_command_description: '',
			obs_command_shortcut: SHORTCUTS[editCommands.length % SHORTCUTS.length],
		};
		editCommands = [...editCommands, newCmd];
	}

	function removeCommand(idx: number) {
		editCommands = editCommands.filter((_, i) => i !== idx);
	}

	async function savePreset() {
		if (!studio) return;
		if (!editPresetName.trim()) {
			addToast('error', 'Preset name is required.');
			return;
		}
		saving = true;
		try {
			let updatedPresets: Preset[];
			if (isCreatingPreset) {
				updatedPresets = [
					...allPresets,
					{ id: null, studio_id: studio.id, name: editPresetName.trim(), commands: editCommands },
				];
			} else {
				updatedPresets = allPresets.map((p, i) =>
					i === selectedPresetIdx
						? { ...p, name: editPresetName.trim(), commands: editCommands }
						: p
				);
			}

			const res = await fetch(`${getBackendUrl()}/studios/${studio.id}`, {
				method: 'PUT',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					name: studio.name,
					obs_browser_source_address: studio.obs_browser_source_address,
					presets: updatedPresets.map((p) => ({
						name: p.name,
						commands: p.commands.map((c) => ({
							obs_command_name: c.obs_command_name,
							obs_command_color: c.obs_command_color,
							obs_command_shortcut: c.obs_command_shortcut,
							obs_command_description: c.obs_command_description,
						})),
					})),
				}),
			});
			const data = await res.json();
			if (data.success) {
				studio = data.studio;
				if (isCreatingPreset) {
					// Select the newly created preset (last in list)
					const newIdx = (data.studio.presets?.length ?? 1) - 1;
					selectedPresetIdx = newIdx;
					isCreatingPreset = false;
				}
				addToast('success', 'Preset saved.');
			} else {
				addToast('error', data.error ?? 'Save failed.');
			}
		} catch {
			addToast('error', 'Request failed.');
		} finally {
			saving = false;
		}
	}

	async function deletePreset() {
		if (!studio || selectedPresetIdx === null) return;
		const preset = allPresets[selectedPresetIdx];
		if (!confirm(`Delete preset "${preset?.name}"?`)) return;

		const updatedPresets = allPresets.filter((_, i) => i !== selectedPresetIdx);
		saving = true;
		try {
			const res = await fetch(`${getBackendUrl()}/studios/${studio.id}`, {
				method: 'PUT',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					name: studio.name,
					obs_browser_source_address: studio.obs_browser_source_address,
					presets: updatedPresets.map((p) => ({
						name: p.name,
						commands: p.commands.map((c) => ({
							obs_command_name: c.obs_command_name,
							obs_command_color: c.obs_command_color,
							obs_command_shortcut: c.obs_command_shortcut,
							obs_command_description: c.obs_command_description,
						})),
					})),
				}),
			});
			const data = await res.json();
			if (data.success) {
				studio = data.studio;
				selectedPresetIdx = null;
				addToast('success', 'Preset deleted.');
			} else {
				addToast('error', data.error ?? 'Delete failed.');
			}
		} catch {
			addToast('error', 'Request failed.');
		} finally {
			saving = false;
		}
	}

	async function saveStudioName() {
		if (!studio || !editStudioName.trim()) return;
		savingStudioName = true;
		try {
			const res = await fetch(`${getBackendUrl()}/studios/${studio.id}`, {
				method: 'PUT',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					name: editStudioName.trim(),
					obs_browser_source_address: studio.obs_browser_source_address,
					presets: allPresets.map((p) => ({
						name: p.name,
						commands: p.commands.map((c) => ({
							obs_command_name: c.obs_command_name,
							obs_command_color: c.obs_command_color,
							obs_command_shortcut: c.obs_command_shortcut,
							obs_command_description: c.obs_command_description,
						})),
					})),
				}),
			});
			const data = await res.json();
			if (data.success) {
				studio = data.studio;
				addToast('success', 'Studio name updated.');
			} else {
				addToast('error', data.error ?? 'Save failed.');
			}
		} catch {
			addToast('error', 'Request failed.');
		} finally {
			savingStudioName = false;
		}
	}

	async function copyObsUrl() {
		if (!studio) return;
		const full = `${getBackendUrl()}${studio.obs_browser_source_address}`;
		await navigator.clipboard.writeText(full);
		copied = true;
		setTimeout(() => (copied = false), 2000);
	}
</script>

<div class="editor-wrap">
	{#if !IS_TAURI}
		<TopNav back={{ href: '/', label: 'Presets' }} />
	{/if}
	<div class="editor-body">

		<!-- ── Sidebar ── -->
		<aside class="sidebar">
			<div class="sidebar-header">
				<span class="sidebar-title">
					Presets
					<span class="badge">{allPresets.length}</span>
				</span>
				<button class="btn btn-primary btn-sm" onclick={openNewPreset}>
					<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
						<path d="M12 5v14M5 12h14"/>
					</svg>
					New
				</button>
			</div>

			<div class="sidebar-list">
				{#if isCreatingPreset}
					<div class="sidebar-item selected">
						<span class="item-name">New Preset…</span>
					</div>
				{/if}
				{#each allPresets as preset, idx (preset.id)}
					<button
						class="sidebar-item"
						class:selected={!isCreatingPreset && selectedPresetIdx === idx}
						onclick={() => selectPreset(idx)}
					>
						<span class="item-name">{preset.name}</span>
						<span class="item-meta">{preset.commands.length} command{preset.commands.length !== 1 ? 's' : ''}</span>
					</button>
				{:else}
					{#if !isCreatingPreset}
						<div class="sidebar-empty">No presets yet.<br/>Click "New" to get started.</div>
					{/if}
				{/each}
			</div>

			<!-- Studio info at bottom of sidebar -->
			{#if studio}
				<div class="studio-info">
					<div class="studio-info-label">Studio Name</div>
					<div class="studio-name-row">
						<input
							class="studio-name-input"
							type="text"
							bind:value={editStudioName}
							placeholder="Studio name"
						/>
						<button
							class="btn btn-ghost btn-sm btn-icon"
							onclick={saveStudioName}
							disabled={savingStudioName || editStudioName.trim() === studio.name}
							title="Save studio name"
						>
							<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
								<polyline points="20 6 9 17 4 12"/>
							</svg>
						</button>
					</div>
					<div class="obs-url-row">
						<span class="obs-url-label">OBS Source</span>
						<button class="btn btn-ghost btn-xs" onclick={copyObsUrl}>
							{copied ? '✓ Copied' : 'Copy URL'}
						</button>
					</div>
				</div>
			{/if}
		</aside>

		<!-- ── Main Editor Area ── -->
		<main class="editor-main">
			{#if hasSelection}
				<div class="editor-panel">
					<div class="panel-header">
						<div class="panel-title-area">
							<input
								class="preset-name-input"
								type="text"
								bind:value={editPresetName}
								placeholder="Preset name"
							/>
						</div>
						<div class="panel-actions">
							{#if !isCreatingPreset}
								<button class="btn btn-danger btn-sm" onclick={deletePreset} disabled={saving}>Delete</button>
							{/if}
							<button class="btn btn-ghost btn-sm" onclick={() => { selectedPresetIdx = null; isCreatingPreset = false; }}>
								Cancel
							</button>
							<button class="btn btn-primary" onclick={savePreset} disabled={saving}>
								{saving ? 'Saving…' : isCreatingPreset ? 'Create Preset' : 'Save Changes'}
							</button>
						</div>
					</div>

					<div class="commands-area">
						<div class="commands-header">
							<span class="commands-title">
								Commands
								<span class="badge-sm">{editCommands.length}</span>
							</span>
							<button class="btn btn-secondary btn-sm" onclick={addCommand}>
								<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
									<path d="M12 5v14M5 12h14"/>
								</svg>
								Add Command
							</button>
						</div>

						{#if editCommands.length === 0}
							<div class="commands-empty">
								No commands yet. Click "Add Command" to create one.
							</div>
						{:else}
							<table class="data-table">
								<thead>
									<tr>
										<th>Name</th>
										<th style="width:100px">Shortcut</th>
										<th style="width:100px">Color</th>
										<th>Description</th>
										<th style="width:40px"></th>
									</tr>
								</thead>
								<tbody>
									{#each editCommands as cmd, ci}
										<tr>
											<td>
												<input class="form-input-inline" type="text" bind:value={cmd.obs_command_name} placeholder="Command name" />
											</td>
											<td>
												<select class="form-select-inline" bind:value={cmd.obs_command_shortcut}>
													{#each SHORTCUTS as s}
														<option value={s}>{s}</option>
													{/each}
												</select>
											</td>
											<td>
												<div class="color-cell">
													<input type="color" bind:value={cmd.obs_command_color} />
													<span class="color-hex">{cmd.obs_command_color}</span>
												</div>
											</td>
											<td>
												<input class="form-input-inline" type="text" bind:value={cmd.obs_command_description} placeholder="Optional description" />
											</td>
											<td>
												<button class="btn btn-danger btn-icon btn-sm" aria-label="Remove command" onclick={() => removeCommand(ci)}>
													<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
														<path d="M18 6L6 18M6 6l12 12"/>
													</svg>
												</button>
											</td>
										</tr>
									{/each}
								</tbody>
							</table>
						{/if}
					</div>
				</div>
			{:else}
				<div class="empty-state">
					<svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.2" class="empty-icon">
						<rect x="3" y="3" width="18" height="18" rx="2"/>
						<path d="M9 9h6M9 12h6M9 15h4"/>
					</svg>
					<p class="empty-title">No preset selected</p>
					<p class="empty-hint">Pick a preset from the sidebar, or create a new one.</p>
					<button class="btn btn-primary btn-sm" onclick={openNewPreset}>
						<svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
							<path d="M12 5v14M5 12h14"/>
						</svg>
						New Preset
					</button>
				</div>
			{/if}
		</main>
	</div>
</div>

<style>
	/* ── Layout ── */
	.editor-wrap {
		height: 100vh;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.editor-body {
		flex: 1;
		display: flex;
		overflow: hidden;
	}

	/* ── Sidebar ── */
	.sidebar {
		width: 240px;
		flex-shrink: 0;
		background: var(--surface-1);
		border-right: 1px solid var(--border-1);
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.sidebar-header {
		padding: 12px 16px;
		border-bottom: 1px solid var(--border-1);
		display: flex;
		align-items: center;
		justify-content: space-between;
		flex-shrink: 0;
	}

	.sidebar-title {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.07em;
		color: var(--text-3);
		display: flex;
		align-items: center;
		gap: 7px;
	}

	.badge {
		background: var(--surface-3);
		color: var(--text-2);
		font-size: 11px;
		font-weight: 700;
		border-radius: 999px;
		padding: 1px 7px;
		text-transform: none;
		letter-spacing: normal;
	}

	.sidebar-list {
		flex: 1;
		overflow-y: auto;
	}

	.sidebar-item {
		display: flex;
		flex-direction: column;
		gap: 3px;
		padding: 11px 16px;
		width: 100%;
		text-align: left;
		background: transparent;
		border: none;
		border-left: 3px solid transparent;
		cursor: pointer;
		transition: background 0.1s;
	}

	.sidebar-item:hover {
		background: var(--surface-2);
	}

	.sidebar-item.selected {
		background: var(--accent-dim);
		border-left-color: var(--accent);
	}

	.item-name {
		font-size: 13px;
		font-weight: 500;
		color: var(--text-1);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.item-meta {
		font-size: 11px;
		color: var(--text-3);
	}

	.sidebar-empty {
		padding: 28px 16px;
		font-size: 12px;
		color: var(--text-3);
		text-align: center;
		line-height: 1.7;
	}

	/* ── Studio info panel ── */
	.studio-info {
		border-top: 1px solid var(--border-1);
		padding: 12px 16px;
		flex-shrink: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.studio-info-label {
		font-size: 10px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.07em;
		color: var(--text-3);
	}

	.studio-name-row {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.studio-name-input {
		flex: 1;
		font-size: 12px;
		padding: 5px 8px;
		background: var(--surface-2);
		border: 1px solid var(--border-1);
		border-radius: var(--r-sm);
		color: var(--text-1);
		min-width: 0;
	}

	.obs-url-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.obs-url-label {
		font-size: 11px;
		color: var(--text-3);
	}

	.btn-xs {
		padding: 4px 8px;
		font-size: 11px;
	}

	/* ── Main editor area ── */
	.editor-main {
		flex: 1;
		overflow-y: auto;
		padding: 32px 40px;
		background: var(--bg);
	}

	.editor-panel {
		max-width: 960px;
	}

	/* ── Panel header ── */
	.panel-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		margin-bottom: 28px;
		padding-bottom: 20px;
		border-bottom: 1px solid var(--border-1);
	}

	.panel-title-area {
		flex: 1;
		min-width: 0;
	}

	.preset-name-input {
		font-size: 20px;
		font-weight: 700;
		color: var(--text-1);
		background: transparent;
		border: 1px solid transparent;
		border-radius: var(--r-sm);
		padding: 4px 8px;
		margin: -4px -8px;
		font-family: inherit;
		width: 100%;
		transition: border-color 0.15s;
	}

	.preset-name-input:focus {
		border-color: var(--border-2);
		background: var(--surface-2);
	}

	.panel-actions {
		display: flex;
		align-items: center;
		gap: 8px;
		flex-shrink: 0;
	}

	/* ── Commands area ── */
	.commands-area {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.commands-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.commands-title {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.07em;
		color: var(--text-3);
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.badge-sm {
		font-size: 10px;
		background: var(--surface-3);
		color: var(--text-2);
		border-radius: 4px;
		padding: 1px 6px;
		text-transform: none;
		letter-spacing: normal;
		font-weight: 600;
	}

	.commands-empty {
		padding: 32px;
		text-align: center;
		font-size: 13px;
		color: var(--text-3);
		background: var(--surface-1);
		border: 1px solid var(--border-1);
		border-radius: var(--r);
	}

	/* ── Inline table inputs ── */
	.form-input-inline {
		background: transparent;
		border: 1px solid transparent;
		color: var(--text-1);
		font-size: 13px;
		font-family: inherit;
		padding: 4px 6px;
		border-radius: var(--r-sm);
		width: 100%;
		outline: none;
		transition: border-color 0.15s;
	}

	.form-input-inline:focus {
		border-color: var(--border-2);
		background: var(--surface-3);
	}

	.form-select-inline {
		background: var(--surface-3);
		border: 1px solid var(--border-1);
		color: var(--text-1);
		font-size: 12px;
		font-family: inherit;
		padding: 4px 6px;
		border-radius: var(--r-sm);
		width: 100%;
		outline: none;
		cursor: pointer;
	}

	.color-cell {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.color-hex {
		font-size: 10px;
		font-family: ui-monospace, monospace;
		color: var(--text-3);
	}

	/* ── Empty state ── */
	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 10px;
		height: calc(100vh - 48px - 64px);
		color: var(--text-3);
		text-align: center;
	}

	.empty-icon {
		color: var(--text-3);
		opacity: 0.35;
		margin-bottom: 4px;
	}

	.empty-title {
		font-size: 15px;
		font-weight: 600;
		color: var(--text-2);
	}

	.empty-hint {
		font-size: 13px;
		color: var(--text-3);
		margin-bottom: 6px;
	}
</style>
