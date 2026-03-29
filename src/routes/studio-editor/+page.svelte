<script lang="ts">
	import { onMount } from 'svelte';
	import TopNav from '$lib/components/TitleBarWeb.svelte';
	import { socket } from '$lib/api/socket';
	import { fetchStudios } from '$lib/api/api';
	import { addToast } from '$lib/stores/toasts';
	import type { Studio, ObsCommand, Preset } from '$lib/types';
	import { getBackendUrl } from '$lib/bridge';
	import { IS_TAURI } from '$lib/bridge';

	const SHORTCUTS = ['F13','F14','F15','F16','F17','F18','F19','F20','F21','F22','F23','F24'];

	let studios = $state<Studio[]>([]);
	let saving = $state(false);
	let copied = $state(false);

	// Selection state
	let selectedId = $state<number | null>(null);
	let isCreatingNew = $state(false);

	// Edit state (populated when a studio is selected or New is clicked)
	let editId = $state<number | null>(null);
	let editName = $state('');
	let editBrowserAddr = $state('');
	let editPresets = $state<Preset[]>([]);
	let expandedPresetIdx = $state<number | null>(null);

	const isNew = $derived(isCreatingNew);
	const hasSelection = $derived(isCreatingNew || selectedId !== null);
	const allCommandsCount = $derived(editPresets.reduce((sum, p) => sum + p.commands.length, 0));

	onMount(() => {
		fetchStudios().then((data) => { studios = data; });

		socket.on('studio-created', (data: any) => {
			if (data.success) {
				// Deduplicate: direct HTTP update may have already added it
				if (!studios.some((s) => s.id === data.studio.id)) {
					studios = [...studios, data.studio];
				}
				addToast('success', 'Studio created.');
				// Auto-select only if not already done by the HTTP handler
				if (selectedId !== data.studio.id) {
					isCreatingNew = false;
					selectedId = data.studio.id;
					editId = data.studio.id;
					editBrowserAddr = data.studio.obs_browser_source_address;
				}
			}
		});

		socket.on('studio-updated', (data: any) => {
			if (data.success) {
				studios = studios.map((s) => (s.id === data.studio.id ? data.studio : s));
				addToast('success', 'Studio saved.');
				if (selectedId === data.studio.id) {
					editBrowserAddr = data.studio.obs_browser_source_address;
					editPresets = (data.studio.presets ?? []).map((p: any) => ({
						...p,
						commands: p.commands.map((c: any) => ({ ...c })),
					}));
				}
			}
		});

		socket.on('studio-deleted', (data: any) => {
			if (data.success) {
				studios = studios.filter((s) => s.id !== data.id);
				addToast('success', 'Studio deleted.');
				if (selectedId === data.id) {
					selectedId = null;
					isCreatingNew = false;
				}
			}
		});

		socket.on('update-studios', () => {
			fetchStudios().then((data) => { studios = data; });
		});

		return () => {
			socket.off('studio-created');
			socket.off('studio-updated');
			socket.off('studio-deleted');
			socket.off('update-studios');
		};
	});

	function openNew() {
		isCreatingNew = true;
		selectedId = null;
		editId = null;
		editName = '';
		editBrowserAddr = '';
		editPresets = [];
		expandedPresetIdx = null;
	}

	function selectStudio(s: Studio) {
		isCreatingNew = false;
		selectedId = s.id;
		editId = s.id;
		editName = s.name;
		editBrowserAddr = s.obs_browser_source_address;
		editPresets = (s.presets ?? []).map((p) => ({
			...p,
			commands: p.commands.map((c) => ({ ...c })),
		}));
		if (editPresets.length === 0 && s.commands.length > 0) {
			editPresets = [{
				id: null,
				studio_id: s.id,
				name: 'Default',
				commands: s.commands.map((c) => ({ ...c })),
			}];
		}
		expandedPresetIdx = editPresets.length > 0 ? 0 : null;
	}

	async function deleteCurrentStudio() {
		const studio = studios.find((s) => s.id === selectedId);
		if (!studio) return;
		if (!confirm(`Delete studio "${studio.name}"?`)) return;
		const res = await fetch(`${getBackendUrl()}/studios/${studio.id}`, { method: 'DELETE' });
		const data = await res.json();
		if (!data.success) addToast('error', data.error ?? 'Delete failed.');
	}

	function addPreset() {
		editPresets = [...editPresets, { id: null, studio_id: editId ?? undefined, name: `Preset ${editPresets.length + 1}`, commands: [] }];
		expandedPresetIdx = editPresets.length - 1;
	}

	function removePreset(idx: number) {
		editPresets = editPresets.filter((_, i) => i !== idx);
		if (expandedPresetIdx !== null && expandedPresetIdx >= editPresets.length) {
			expandedPresetIdx = editPresets.length > 0 ? editPresets.length - 1 : null;
		}
	}

	function addCommand(presetIdx: number) {
		const preset = editPresets[presetIdx];
		const newCmd: ObsCommand = {
			id: null,
			obs_command_name: 'New Command',
			obs_command_color: '#38bdf8',
			obs_command_description: '',
			obs_command_shortcut: SHORTCUTS[preset.commands.length % SHORTCUTS.length],
		};
		editPresets = editPresets.map((p, i) =>
			i === presetIdx ? { ...p, commands: [...p.commands, newCmd] } : p
		);
	}

	function removeCommand(presetIdx: number, cmdIdx: number) {
		editPresets = editPresets.map((p, i) =>
			i === presetIdx ? { ...p, commands: p.commands.filter((_, ci) => ci !== cmdIdx) } : p
		);
	}

	async function save() {
		if (!editName.trim()) {
			addToast('error', 'Studio name is required.');
			return;
		}
		saving = true;
		try {
			if (isNew) {
				const res = await fetch(`${getBackendUrl()}/studios`, {
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({ name: editName.trim() }),
				});
				const data = await res.json();
				if (data.success) {
					// Update list and auto-select the new studio immediately
					studios = [...studios, data.studio];
					isCreatingNew = false;
					selectedId = data.studio.id;
					editId = data.studio.id;
					editBrowserAddr = data.studio.obs_browser_source_address;
					editPresets = [];
				} else {
					addToast('error', data.error ?? 'Create failed.');
				}
				// socket 'studio-created' event deduplicates if also received
			} else {
				const res = await fetch(`${getBackendUrl()}/studios/${editId}`, {
					method: 'PUT',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({
						name: editName.trim(),
						obs_browser_source_address: editBrowserAddr,
						presets: editPresets.map((p) => ({
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
				if (!data.success) addToast('error', data.error ?? 'Save failed.');
			}
		} catch {
			addToast('error', 'Request failed.');
		} finally {
			saving = false;
		}
	}

	async function copyAddr() {
		const full = `${getBackendUrl()}${editBrowserAddr}`;
		await navigator.clipboard.writeText(full);
		copied = true;
		setTimeout(() => (copied = false), 2000);
	}
</script>

<div class="editor-wrap">
	{#if !IS_TAURI}
		<TopNav back={{ href: '/', label: 'Studios' }} />
	{/if}
	<div class="editor-body">

		<!-- ── Sidebar ── -->
		<aside class="sidebar">
			<div class="sidebar-header">
				<span class="sidebar-title">
					Studios
					<span class="badge">{studios.length}</span>
				</span>
				<button class="btn btn-primary btn-sm" onclick={openNew}>
					<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
						<path d="M12 5v14M5 12h14"/>
					</svg>
					New
				</button>
			</div>
			<div class="sidebar-list">
				{#if isCreatingNew}
					<div class="sidebar-item selected">
						<span class="item-name">New Studio…</span>
					</div>
				{/if}
				{#each studios as s (s.id)}
					<button
						class="sidebar-item"
						class:selected={selectedId === s.id}
						onclick={() => selectStudio(s)}
					>
						<span class="item-name">{s.name}</span>
						<span class="item-meta">
							{(s.presets ?? []).length} presets · {(s.presets ?? []).reduce((n, p) => n + p.commands.length, 0) + (s.commands?.length ?? 0)} cmds
						</span>
					</button>
				{:else}
					{#if !isCreatingNew}
						<div class="sidebar-empty">No studios yet.<br/>Click "New" to get started.</div>
					{/if}
				{/each}
			</div>
		</aside>

		<!-- ── Main Editor Area ── -->
		<main class="editor-main">
			{#if hasSelection}
				<div class="editor-panel">
					<div class="panel-header">
						<div class="panel-title-area">
							<h1 class="panel-title">{isNew ? 'New Studio' : (editName || 'Untitled')}</h1>
							{#if !isNew}
								<span class="panel-id">ID #{editId}</span>
							{/if}
						</div>
						<div class="panel-actions">
							{#if !isNew}
								<button class="btn btn-danger btn-sm" onclick={deleteCurrentStudio}>Delete</button>
							{/if}
							<button class="btn btn-ghost btn-sm" onclick={() => { selectedId = null; isCreatingNew = false; }}>
								Cancel
							</button>
							<button class="btn btn-primary" onclick={save} disabled={saving}>
								{saving ? 'Saving…' : isNew ? 'Create Studio' : 'Save Changes'}
							</button>
						</div>
					</div>

					<div class="form-body">
						<!-- Name -->
						<div class="field-group">
							<label class="field-label" for="studio-name">Studio Name</label>
							<input id="studio-name" class="form-input" type="text" bind:value={editName} placeholder="e.g. Studio A" />
						</div>

						{#if !isNew}
							<!-- OBS Source URL -->
							<div class="field-group">
								<span class="field-label">OBS Browser Source URL</span>
								<div class="copy-row">
									<span class="mono-url url-display">{getBackendUrl()}{editBrowserAddr}</span>
									<button class="btn btn-ghost btn-sm" onclick={copyAddr}>
										{copied ? '✓ Copied' : 'Copy'}
									</button>
								</div>
								<p class="helper-text">Paste this URL into your OBS browser source.</p>
							</div>

							<!-- Presets -->
							<div class="field-group">
								<div class="field-group-header">
									<span class="field-label">
										Presets
										<span class="badge-sm">{editPresets.length}</span>
									</span>
									<button class="btn btn-secondary btn-sm" onclick={addPreset}>+ Add Preset</button>
								</div>

								<div class="sub-card">
									{#if editPresets.length === 0}
										<div class="sub-empty">No presets yet. Click "Add Preset" to create one.</div>
									{:else}
										{#each editPresets as preset, pi}
											<div class="preset-block">
												<div class="preset-header" role="button" tabindex="0"
													onclick={() => { expandedPresetIdx = expandedPresetIdx === pi ? null : pi; }}
													onkeydown={(e) => e.key === 'Enter' && (expandedPresetIdx = expandedPresetIdx === pi ? null : pi)}
												>
													<div class="preset-header-left">
														<svg class="chevron" class:rotated={expandedPresetIdx === pi} width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
															<polyline points="6 9 12 15 18 9"/>
														</svg>
														<input
															class="preset-name-input"
															type="text"
															bind:value={preset.name}
															onclick={(e) => e.stopPropagation()}
															placeholder="Preset name"
														/>
														<span class="badge-sm">{preset.commands.length} cmds</span>
													</div>
													<button class="btn btn-danger btn-icon btn-sm"
														aria-label="Remove preset"
														onclick={(e) => { e.stopPropagation(); removePreset(pi); }}
													>
														<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
															<path d="M18 6L6 18M6 6l12 12"/>
														</svg>
													</button>
												</div>

												{#if expandedPresetIdx === pi}
													<div class="preset-body">
														<div class="preset-actions">
															<button class="btn btn-secondary btn-sm" onclick={() => addCommand(pi)}>+ Add Command</button>
														</div>
														{#if preset.commands.length > 0}
															<table class="data-table">
																<thead>
																	<tr>
																		<th>Name</th>
																		<th style="width:90px">Shortcut</th>
																		<th style="width:80px">Color</th>
																		<th>Description</th>
																		<th style="width:36px"></th>
																	</tr>
																</thead>
																<tbody>
																	{#each preset.commands as cmd, ci}
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
																				<button class="btn btn-danger btn-icon btn-sm" aria-label="Remove command" onclick={() => removeCommand(pi, ci)}>
																					<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
																						<path d="M18 6L6 18M6 6l12 12"/>
																					</svg>
																				</button>
																			</td>
																		</tr>
																	{/each}
																</tbody>
															</table>
														{:else}
															<div class="sub-empty">No commands. Add one above.</div>
														{/if}
													</div>
												{/if}
											</div>
										{/each}
									{/if}
								</div>
							</div>
						{/if}
					</div>
				</div>
			{:else}
				<div class="empty-state">
					<svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.2" class="empty-icon">
						<path d="M15 10l4.553-2.069A1 1 0 0121 8.82V15.18a1 1 0 01-1.447.893L15 14M3 8a2 2 0 012-2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V8z"/>
					</svg>
					<p class="empty-title">No studio selected</p>
					<p class="empty-hint">Pick a studio from the sidebar, or create a new one.</p>
					<button class="btn btn-primary btn-sm" onclick={openNew}>
						<svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
							<path d="M12 5v14M5 12h14"/>
						</svg>
						New Studio
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
		width: 260px;
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

	/* ── Main editor area ── */
	.editor-main {
		flex: 1;
		overflow-y: auto;
		padding: 32px 40px;
		background: var(--bg);
	}

	.editor-panel {
		max-width: 880px;
	}

	/* ── Panel header ── */
	.panel-header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 16px;
		margin-bottom: 32px;
		padding-bottom: 24px;
		border-bottom: 1px solid var(--border-1);
	}

	.panel-title-area {
		display: flex;
		align-items: baseline;
		gap: 12px;
		min-width: 0;
	}

	.panel-title {
		font-size: 20px;
		font-weight: 700;
		color: var(--text-1);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.panel-id {
		font-size: 12px;
		color: var(--text-3);
		font-variant-numeric: tabular-nums;
		flex-shrink: 0;
	}

	.panel-actions {
		display: flex;
		align-items: center;
		gap: 8px;
		flex-shrink: 0;
	}

	/* ── Form body ── */
	.form-body {
		display: flex;
		flex-direction: column;
		gap: 28px;
	}

	.field-group {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.field-group-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.field-label {
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

	.form-input {
		width: 100%;
		padding: 9px 12px;
		background: var(--surface-2);
		border: 1px solid var(--border-1);
		border-radius: var(--r-sm);
		color: var(--text-1);
		font-size: 13px;
		font-family: inherit;
		outline: none;
		transition: border-color 0.15s;
	}

	.form-input:focus {
		border-color: var(--accent);
	}

	.copy-row {
		display: flex;
		align-items: center;
		gap: 8px;
		background: var(--surface-2);
		border: 1px solid var(--border-1);
		border-radius: var(--r-sm);
		padding: 8px 12px;
	}

	.mono-url {
		font-size: 13px;
		color: var(--text-3);
		font-family: ui-monospace, 'Courier New', monospace;
	}

	.url-display {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.helper-text {
		font-size: 11px;
		color: var(--text-3);
	}

	/* ── Sub-card (presets container) ── */
	.sub-card {
		background: var(--surface-1);
		border: 1px solid var(--border-1);
		border-radius: var(--r);
		overflow: hidden;
	}

	.sub-empty {
		padding: 20px;
		text-align: center;
		font-size: 12px;
		color: var(--text-3);
	}

	/* ── Preset accordion ── */
	.preset-block {
		border-bottom: 1px solid var(--border-1);
	}

	.preset-block:last-child {
		border-bottom: none;
	}

	.preset-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 10px 12px;
		cursor: pointer;
		user-select: none;
		transition: background 0.1s;
	}

	.preset-header:hover {
		background: var(--surface-2);
	}

	.preset-header-left {
		display: flex;
		align-items: center;
		gap: 8px;
		flex: 1;
		min-width: 0;
	}

	.chevron {
		flex-shrink: 0;
		transition: transform 0.2s;
		color: var(--text-3);
	}

	.chevron.rotated {
		transform: rotate(180deg);
	}

	.preset-name-input {
		background: transparent;
		border: 1px solid transparent;
		color: var(--text-1);
		font-size: 14px;
		font-weight: 500;
		font-family: inherit;
		padding: 2px 6px;
		border-radius: var(--r-sm);
		outline: none;
		min-width: 0;
		flex: 1;
		transition: border-color 0.15s;
	}

	.preset-name-input:focus {
		border-color: var(--border-2);
		background: var(--surface-3);
	}

	.preset-body {
		padding: 10px 12px 12px;
		background: rgba(0,0,0,0.15);
		border-top: 1px solid var(--border-1);
	}

	.preset-actions {
		display: flex;
		justify-content: flex-end;
		margin-bottom: 10px;
	}

	/* ── Inline command table inputs ── */
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
