<script lang="ts">
	import { onMount } from 'svelte';
	import { beforeNavigate } from '$app/navigation';
	import TopNav from '$lib/components/TitleBarWeb.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import { socket } from '$lib/api/socket';
	import { fetchPrograms, fetchPopUps, fetchScreens, uploadProgramImage, imgUrl } from '$lib/api/api';
	import { fetchPlugins } from '$lib/api/plugins';
	import { addToast } from '$lib/toasts';
	import type { Program, PopUp, Screen, ProgramPopUp, PluginInfo } from '$lib/types';
	import MediaPreview from '$lib/components/MediaPreview.svelte';
	import { getBackendUrl } from '$lib/bridge';
	import { IS_TAURI } from '$lib/bridge';
	import { showConfirm } from '$lib/confirm';
	import { fetchProgramPluginIds, setProgramPluginIds } from '$lib/programPlugins';

	/* ─── State ─────────────────────────────────────────────── */
	let programs = $state<Program[]>([]);
	let allPopUps = $state<PopUp[]>([]);
	let allScreens = $state<Screen[]>([]);
	let allControlPlugins = $state<PluginInfo[]>([]);
	let addPopUpModalOpen = $state(false);
	let addScreenModalOpen = $state(false);

	// Selection state
	let selectedId = $state<number | null>(null);
	let isCreatingNew = $state(false);
	let activeTab = $state<'details' | 'plugins'>('details');

	// Edit state
	let editId = $state<number | null>(null);
	let editName = $state('');
	let editLogoPath = $state<string | null>(null);
	let editBgPath = $state<string | null>(null);
	let editScreenIds = $state<number[]>([]);
	let editProgramPopUps = $state<ProgramPopUp[]>([]);
	let editPluginIds = $state<string[]>([]);

	let saving = $state(false);

	/* ─── Dirty tracking ─────────────────────────────────── */
	let savedSnapshot = $state('');

	function makeSnapshot(): string {
		return JSON.stringify([
			editName,
			editLogoPath,
			editBgPath,
			editScreenIds,
			editProgramPopUps.map((pa) => [pa.popup_id, pa.popup_launch_type, pa.duration, pa.frequency]),
			[...editPluginIds].sort(),
		]);
	}

	function takeSnapshot() {
		savedSnapshot = makeSnapshot();
	}

	const isNew = $derived(isCreatingNew);
	const hasSelection = $derived(isCreatingNew || selectedId !== null);
	const isDirty = $derived(hasSelection && makeSnapshot() !== savedSnapshot);

	const editScreens = $derived<Screen[]>(
		editScreenIds.map((id) => allScreens.find((s) => s.id === id)).filter((s): s is Screen => !!s)
	);

	/* ─── Navigation guard ──────────────────────────────────── */
	beforeNavigate(({ cancel }) => {
		if (isDirty && !window.confirm('You have unsaved changes. Leave this page?')) {
			cancel();
		}
	});

	/* ─── Lifecycle ──────────────────────────────────────────── */
	onMount(() => {
		Promise.all([fetchPrograms(), fetchPopUps(), fetchScreens(), fetchPlugins()]).then(([p, a, s, pl]) => {
			programs = p;
			allPopUps = a;
			allScreens = s;
			allControlPlugins = pl.filter((plug) => plug.enabled && plug.has_control);
		});

		socket.on('program-created', (data: any) => {
			if (data.success) {
				// Deduplicate: direct HTTP response may have already added it
				if (!programs.some((p) => p.id === data.program.id)) {
					programs = [data.program, ...programs];
				}
				addToast('success', 'Program created.');
				// Auto-select only if not already done by the HTTP handler
				if (selectedId !== data.program.id) {
					isCreatingNew = false;
					selectedId = data.program.id;
					editId = data.program.id;
					editLogoPath = data.program.logo_path;
					editBgPath = data.program.background_graphics_path;
					editScreenIds = [];
					editProgramPopUps = [];
				}
			}
		});

		socket.on('program-updated', (data: any) => {
			if (data.success) {
				programs = programs.map((p) => (p.id === data.program.id ? data.program : p));
				addToast('success', 'Program saved.');
			}
		});

		socket.on('program-deleted', (data: any) => {
			if (data.success) {
				programs = programs.filter((p) => p.id !== data.id);
				addToast('success', 'Program deleted.');
				if (selectedId === data.id) {
					selectedId = null;
					isCreatingNew = false;
				}
			}
		});

		socket.on('update-programs', () => {
			fetchPrograms().then((data) => { programs = data; });
		});

		return () => {
			socket.off('program-created');
			socket.off('program-updated');
			socket.off('program-deleted');
			socket.off('update-programs');
		};
	});

	/* ─── Selection ──────────────────────────────────────────── */
	async function openNew() {
		if (isDirty) {
			const ok = await showConfirm({
				title: 'Unsaved Changes',
				message: 'Discard unsaved changes and create a new program?',
				confirmLabel: 'Discard',
			});
			if (!ok) return;
		}
		isCreatingNew = true;
		selectedId = null;
		editId = null;
		editName = '';
		editLogoPath = null;
		editBgPath = null;
		editScreenIds = [];
		editProgramPopUps = [];
		editPluginIds = [];
		activeTab = 'details';
		takeSnapshot();
	}

	async function selectProgram(p: Program) {
		if (isDirty) {
			const ok = await showConfirm({
				title: 'Unsaved Changes',
				message: `Discard unsaved changes and switch to "${p.name}"?`,
				confirmLabel: 'Discard',
			});
			if (!ok) return;
		}
		isCreatingNew = false;
		selectedId = p.id;
		editId = p.id;
		editName = p.name;
		editLogoPath = p.logo_path;
		editBgPath = p.background_graphics_path;
		editScreenIds = p.graphics.map((g) => g.id);
		editProgramPopUps = p.program_popups.map((pa) => ({ ...pa }));
		// Load saved plugin preferences from the server; default to none if unset
		editPluginIds = await fetchProgramPluginIds(p.id);
		activeTab = 'details';
		takeSnapshot();
	}

	async function cancelEdit() {
		if (isDirty) {
			const ok = await showConfirm({
				title: 'Unsaved Changes',
				message: 'Discard unsaved changes?',
				confirmLabel: 'Discard',
			});
			if (!ok) return;
		}
		selectedId = null;
		isCreatingNew = false;
	}

	/* ─── Program CRUD ───────────────────────────────────────── */
	async function deleteCurrentProgram() {
		const program = programs.find((p) => p.id === selectedId);
		if (!program) return;
		if (!await showConfirm({ title: 'Delete Program', message: `Delete program "${program.name}"? This cannot be undone.`, confirmLabel: 'Delete' })) return;
		const res = await fetch(`${getBackendUrl()}/programs/${program.id}`, { method: 'DELETE' });
		const data = await res.json();
		if (!data.success) addToast('error', data.error ?? 'Delete failed.');
	}

	async function save() {
		if (!editName.trim()) {
			addToast('error', 'Program name is required.');
			return;
		}
		saving = true;
		try {
			if (isNew) {
				const res = await fetch(`${getBackendUrl()}/programs`, {
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({ name: editName.trim() }),
				});
				const data = await res.json();
				if (data.success) {
					// Update list and auto-select immediately from HTTP response
					if (!programs.some((p) => p.id === data.program.id)) {
						programs = [data.program, ...programs];
					}
					isCreatingNew = false;
					selectedId = data.program.id;
					editId = data.program.id;
					editLogoPath = data.program.logo_path;
					editBgPath = data.program.background_graphics_path;
					editScreenIds = [];
					editProgramPopUps = [];
					// Persist plugin preferences for the newly-created program.
					await setProgramPluginIds(data.program.id, editPluginIds);
					takeSnapshot();
				} else {
					addToast('error', data.error ?? 'Create failed.');
				}
			} else {
				const res = await fetch(`${getBackendUrl()}/programs/${editId}`, {
					method: 'PUT',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({
						name: editName.trim(),
						logo_path: editLogoPath,
						background_graphics_path: editBgPath,
						screen_ids: editScreenIds,
						popups: editProgramPopUps.map((pa) => ({
							popup_id: pa.popup_id,
							popup_launch_type: pa.popup_launch_type,
							duration: pa.duration,
							frequency: pa.frequency,
						})),
					}),
				});
				const data = await res.json();
				if (data.success && editId !== null) {
					// Persist plugin preferences server-side.
					await setProgramPluginIds(editId, editPluginIds);
					takeSnapshot();
				}
				if (!data.success) addToast('error', data.error ?? 'Save failed.');
			}
		} catch {
			addToast('error', 'Request failed.');
		} finally {
			saving = false;
		}
	}

	/* ─── Screen helpers ─────────────────────────────────────── */
	function addScreenToProgram(screen: Screen) {
		if (editScreenIds.includes(screen.id)) return;
		editScreenIds = [...editScreenIds, screen.id];
	}

	function removeScreenFromProgram(screenId: number) {
		editScreenIds = editScreenIds.filter((id) => id !== screenId);
	}

	const availableScreens = $derived(
		allScreens.filter((s) => !editScreenIds.includes(s.id))
	);

	/* ─── Logo / Background upload ────────────────────────────── */
	async function uploadLogoImage(e: Event) {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (!file || editId == null) return;
		try {
			const result = await uploadProgramImage(file, editId, 'logo');
			if (result.success) {
				editLogoPath = result.imagePath;
				addToast('success', 'Logo uploaded.');
			} else {
				addToast('error', 'Upload failed.');
			}
		} catch {
			addToast('error', 'Upload failed.');
		}
	}

	async function uploadBgImage(e: Event) {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (!file || editId == null) return;
		try {
			const result = await uploadProgramImage(file, editId, 'background');
			if (result.success) {
				editBgPath = result.imagePath;
				addToast('success', 'Background uploaded.');
			} else {
				addToast('error', 'Upload failed.');
			}
		} catch {
			addToast('error', 'Upload failed.');
		}
	}

	/* ─── PopUp helpers ──────────────────────────────────────── */
	function addPopUpToProgram(popup: PopUp) {
		if (editProgramPopUps.some((pa) => pa.popup_id === popup.id)) return;
		editProgramPopUps = [
			...editProgramPopUps,
			{
				id: -(Date.now()),
				popup_id: popup.id,
				program_id: editId ?? 0,
				popup_launch_type: 'manual',
				duration: 10,
				frequency: 1,
				popup,
			},
		];
	}

	async function removePopUpFromProgram(popupId: number) {
		const pa = editProgramPopUps.find((a) => a.popup_id === popupId);
		const popupName = pa?.popup?.name ?? `PopUp ${popupId}`;
		if (!await showConfirm({ title: 'Remove Pop-Up', message: `Remove "${popupName}" from this program?`, confirmLabel: 'Remove' })) return;
		editProgramPopUps = editProgramPopUps.filter((a) => a.popup_id !== popupId);
	}

	function updateProgramPopUp(popupId: number, patch: Partial<ProgramPopUp>) {
		editProgramPopUps = editProgramPopUps.map((pa) =>
			pa.popup_id === popupId ? { ...pa, ...patch } : pa
		);
	}

	const availablePopUps = $derived(
		allPopUps.filter((a) => !editProgramPopUps.some((pa) => pa.popup_id === a.id))
	);

	/* ─── Plugin helpers ────────────────────────────────────────── */
	function togglePlugin(pluginId: string, enabled: boolean) {
		if (enabled) {
			if (!editPluginIds.includes(pluginId)) {
				editPluginIds = [...editPluginIds, pluginId];
			}
		} else {
			editPluginIds = editPluginIds.filter((id) => id !== pluginId);
		}
	}

	/* ─── Image file input refs ─────────────────────────────────── */
	let logoInput = $state() as unknown as HTMLInputElement;
	let bgInput = $state() as unknown as HTMLInputElement;

	/* ─── Reorder: Screens ───────────────────────────────────────── */
	function moveScreenUp(i: number) {
		if (i === 0) return;
		const ids = [...editScreenIds];
		[ids[i - 1], ids[i]] = [ids[i], ids[i - 1]];
		editScreenIds = ids;
	}

	function moveScreenDown(i: number) {
		if (i === editScreenIds.length - 1) return;
		const ids = [...editScreenIds];
		[ids[i], ids[i + 1]] = [ids[i + 1], ids[i]];
		editScreenIds = ids;
	}

	/* ─── Reorder: PopUps ────────────────────────────────────────── */
	function movePopUpUp(i: number) {
		if (i === 0) return;
		const popups = [...editProgramPopUps];
		[popups[i - 1], popups[i]] = [popups[i], popups[i - 1]];
		editProgramPopUps = popups;
	}

	function movePopUpDown(i: number) {
		if (i === editProgramPopUps.length - 1) return;
		const popups = [...editProgramPopUps];
		[popups[i], popups[i + 1]] = [popups[i + 1], popups[i]];
		editProgramPopUps = popups;
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
					Programs
					<span class="badge">{programs.length}</span>
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
						<div class="item-thumb-wrap">
							<div class="item-thumb-empty">—</div>
						</div>
						<div class="item-info">
							<span class="item-name">New Program…</span>
						</div>
					</div>
				{/if}
				{#each programs as p (p.id)}
					<button
						class="sidebar-item"
						class:selected={selectedId === p.id}
						onclick={() => selectProgram(p)}
					>
						<div class="item-thumb-wrap">
							{#if p.logo_path}
								<MediaPreview class="item-thumb-img" src={imgUrl(p.logo_path)} alt={p.name} />
							{:else}
								<div class="item-thumb-empty">—</div>
							{/if}
						</div>
						<div class="item-info">
							<span class="item-name">{p.name}</span>
							<span class="item-meta">{p.graphics.length} screens · {p.program_popups.length} pop-ups</span>
						</div>
					</button>
				{:else}
					{#if !isCreatingNew}
						<div class="sidebar-empty">No programs yet.<br/>Click "New" to get started.</div>
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
							<h1 class="panel-title">{isNew ? 'New Program' : (editName || 'Untitled')}</h1>
							{#if isDirty}
								<span class="unsaved-badge">Unsaved</span>
							{/if}
							{#if !isNew}
								<span class="panel-id">ID #{editId}</span>
							{/if}
						</div>
						<div class="panel-actions">
							{#if !isNew}
								<button class="btn btn-danger btn-sm" onclick={deleteCurrentProgram}>Delete</button>
							{/if}
							<button class="btn btn-ghost btn-sm" onclick={cancelEdit}>
								Cancel
							</button>
							<button class="btn btn-primary" onclick={save} disabled={saving}>
								{saving ? 'Saving…' : isNew ? 'Create Program' : 'Save Changes'}
							</button>
						</div>
					</div>

					<div class="form-body">
						{#if isNew}
						<!-- Program Name -->
						<div class="field-group">
							<label class="field-label" for="program-name">Program Name</label>
							<input id="program-name" class="form-input" type="text" bind:value={editName} placeholder="e.g. Morning News" />
						</div>
						{/if}

						{#if !isNew}
						<!-- Tab nav -->
						<div class="tab-nav">
							<button
								class="tab-btn"
								class:active={activeTab === 'details'}
								onclick={() => (activeTab = 'details')}
							>Details</button>
							<button
								class="tab-btn"
								class:active={activeTab === 'screens'}
								onclick={() => (activeTab = 'screens')}
							>
								Screens
								<span class="tab-pill">{editScreenIds.length}</span>
							</button>
							<button
								class="tab-btn"
								class:active={activeTab === 'popups'}
								onclick={() => (activeTab = 'popups')}
							>
								PopUps
								<span class="tab-pill">{editProgramPopUps.length}</span>
							</button>
							<button
								class="tab-btn"
								class:active={activeTab === 'plugins'}
								onclick={() => (activeTab = 'plugins')}
							>
								Plugins
								<span class="tab-pill">{editPluginIds.length}</span>
							</button>
						</div>
						{/if}

						{#if !isNew && activeTab === 'details'}
							<!-- Program Name -->
							<div class="field-group">
								<label class="field-label" for="program-name">Program Name</label>
								<input id="program-name" class="form-input" type="text" bind:value={editName} placeholder="e.g. Morning News" />
							</div>

							<!-- Logo + Background side by side -->
							<div class="image-pair">
								<div class="field-group">
									<label class="field-label" for="logo-upload">Program Logo</label>
									<div class="image-preview-box">
										{#if editLogoPath}
											<MediaPreview src={imgUrl(editLogoPath)} alt="Logo" />
										{:else}
											<span class="img-placeholder">Logo</span>
										{/if}
									</div>
									<div class="img-actions">
										<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
										<!-- svelte-ignore a11y_click_events_have_key_events -->
										<input id="logo-upload" bind:this={logoInput} type="file" accept="image/*,video/webm" style="display:none" onchange={uploadLogoImage} />
										<button class="btn btn-secondary btn-sm" style="flex:1" onclick={() => logoInput.click()}>Upload Logo</button>
										{#if editLogoPath}
											<button class="btn btn-danger btn-sm" onclick={() => { editLogoPath = null; }}>Delete</button>
										{/if}
									</div>
								</div>

								<div class="field-group">
									<label class="field-label" for="bg-upload">Background Image</label>
									<div class="image-preview-box">
										{#if editBgPath}
											<MediaPreview src={imgUrl(editBgPath)} alt="Background" style="object-fit:cover" />
										{:else}
											<span class="img-placeholder">Background</span>
										{/if}
									</div>
									<div class="img-actions">
										<input id="bg-upload" bind:this={bgInput} type="file" accept="image/*,video/webm" style="display:none" onchange={uploadBgImage} />
										<button class="btn btn-secondary btn-sm" style="flex:1" onclick={() => bgInput.click()}>Upload Background</button>
										{#if editBgPath}
											<button class="btn btn-danger btn-sm" onclick={() => { editBgPath = null; }}>Delete</button>
										{/if}
									</div>
								</div>
							</div>

						{:else if !isNew && activeTab === 'screens'}
							<!-- Screens -->
							<div class="field-group">
								<div class="field-group-header">
									<span class="field-label">
										Screens
										<span class="badge-sm">{editScreenIds.length}</span>
									</span>
									<div class="header-actions">
										<button class="btn btn-secondary btn-sm" onclick={() => { addScreenModalOpen = true; }}>
											<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M12 5v14M5 12h14"/></svg>
											Add Screen…
										</button>
									</div>
								</div>
								<div class="sub-card">
									{#if editScreens.length > 0}
										<table class="data-table">
											<thead>
												<tr>
													<th style="width:52px"></th>
													<th>Name</th>
													<th style="width:110px">Preview</th>
													<th style="width:90px">Type</th>
													<th style="width:80px">Allow PopUps</th>
													<th style="width:44px"></th>
												</tr>
											</thead>
											<tbody>
												{#each editScreens as s, i (s.id)}
													<tr>
														<td class="reorder-cell">
															<button class="btn-reorder" aria-label="Move up" disabled={i === 0} onclick={() => moveScreenUp(i)}>
																<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M18 15l-6-6-6 6"/></svg>
															</button>
															<button class="btn-reorder" aria-label="Move down" disabled={i === editScreens.length - 1} onclick={() => moveScreenDown(i)}>
																<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M6 9l6 6 6-6"/></svg>
															</button>
														</td>
														<td><span class="fw-medium">{s.graphics_name}</span></td>
														<td class="preview-cell">
															{#if s.graphics_path}
																<MediaPreview class="graphic-thumb" src={imgUrl(s.graphics_path)} alt={s.graphics_name} />
															{:else}
																<span class="no-image-text">No image</span>
															{/if}
														</td>
														<td><span class="badge-sm">{s.media_type}</span></td>
														<td><span class="badge-sm">{s.allow_popups ? 'Yes' : 'No'}</span></td>
														<td>
															<button class="btn btn-danger btn-icon btn-sm" aria-label="Remove screen" onclick={() => removeScreenFromProgram(s.id)}>
																<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M18 6L6 18M6 6l12 12"/></svg>
															</button>
														</td>
													</tr>
												{/each}
											</tbody>
										</table>
									{:else}
										<p class="sub-empty">No screens added yet. Click "Add Screen…" above.</p>
									{/if}
								</div>
							</div>

						{:else if !isNew && activeTab === 'popups'}
							<!-- PopUps -->
							<div class="field-group">
								<div class="field-group-header">
									<span class="field-label">
										PopUps
										<span class="badge-sm">{editProgramPopUps.length}</span>
									</span>
									{#if availablePopUps.length > 0}
										<button class="btn btn-secondary btn-sm" onclick={() => { addPopUpModalOpen = true; }}>
											<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M12 5v14M5 12h14"/></svg>
											Add PopUp…
										</button>
									{:else if allPopUps.length === 0}
										<a href="/popup-editor" class="helper-link">Create pop-ups first →</a>
									{/if}
								</div>
								<div class="sub-card">
									{#if editProgramPopUps.length > 0}
										<table class="data-table popups-table">
											<thead>
												<tr>
													<th style="width:52px"></th>
													<th>PopUp</th>
													<th style="width:140px">Launch Type</th>
													<th style="width:90px">Duration (s)</th>
													<th style="width:105px">Frequency (/hr)</th>
													<th style="width:44px"></th>
												</tr>
											</thead>
											<tbody>
												{#each editProgramPopUps as pa, i (pa.popup_id)}
													{@const popupDetails = allPopUps.find((a) => a.id === pa.popup_id) ?? pa.popup}
													<tr>
														<td class="reorder-cell">
															<button class="btn-reorder" aria-label="Move up" disabled={i === 0} onclick={() => movePopUpUp(i)}>
																<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M18 15l-6-6-6 6"/></svg>
															</button>
															<button class="btn-reorder" aria-label="Move down" disabled={i === editProgramPopUps.length - 1} onclick={() => movePopUpDown(i)}>
																<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M6 9l6 6 6-6"/></svg>
															</button>
														</td>
														<td>
															<div class="popup-row-info">
																<div class="popup-thumb-wrap">
																	{#if popupDetails?.image_path}
																		<MediaPreview class="popup-thumb" src={imgUrl(popupDetails.image_path)} alt={popupDetails?.name} />
																	{:else}
																		<span class="popup-thumb-empty">—</span>
																	{/if}
																</div>
																<div>
																	<div class="popup-name-text">{popupDetails?.name ?? `PopUp ${pa.popup_id}`}</div>
																	<div class="popup-sponsor-text">{popupDetails?.sponsor_name || 'No sponsor'}</div>
																</div>
															</div>
														</td>
														<td>
															<select
																class="form-select"
																value={pa.popup_launch_type}
																onchange={(e) => updateProgramPopUp(pa.popup_id, { popup_launch_type: (e.target as HTMLSelectElement).value as any })}
															>
																<option value="manual">Manual</option>
																<option value="automatic">Automatic</option>
																<option value="both">Both</option>
																<option value="filler">Filler</option>
																<option value="hidden">Hidden</option>
															</select>
														</td>
														<td>
															<input
																class="form-input number-input"
																type="number"
																min="0"
																value={pa.duration}
																disabled={pa.popup_launch_type === 'filler'}
																oninput={(e) => updateProgramPopUp(pa.popup_id, { duration: Math.max(0, Number((e.target as HTMLInputElement).value)) })}
															/>
														</td>
														<td>
															<input
																class="form-input number-input"
																type="number"
																min="0"
																value={pa.frequency}
																disabled={pa.popup_launch_type === 'manual' || pa.popup_launch_type === 'filler' || pa.popup_launch_type === 'hidden'}
																oninput={(e) => updateProgramPopUp(pa.popup_id, { frequency: Math.max(0, Number((e.target as HTMLInputElement).value)) })}
															/>
														</td>
														<td>
															<button class="btn btn-danger btn-icon btn-sm" aria-label="Remove pop-up" onclick={() => removePopUpFromProgram(pa.popup_id)}>
																<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M18 6L6 18M6 6l12 12"/></svg>
															</button>
														</td>
													</tr>
												{/each}
											</tbody>
										</table>
									{:else}
										<p class="sub-empty">No pop-ups added yet. Use "Add PopUp…" above.</p>
									{/if}
								</div>
							</div>

						{:else if !isNew && activeTab === 'plugins'}

							<!-- ── Plugins tab ── -->
							{#if allControlPlugins.length > 0}
								<div class="plugin-list sub-card">
									{#each allControlPlugins as plugin (plugin.id)}
										{@const enabled = editPluginIds.includes(plugin.id)}
										<label class="plugin-row" class:plugin-row-disabled={!enabled}>
											<div class="plugin-row-info">
												<span class="plugin-row-name">{plugin.name}</span>
												<span class="plugin-row-meta">{plugin.description}</span>
											</div>
											<button
												type="button"
												role="switch"
												aria-checked={enabled}
												class="plugin-toggle"
												class:plugin-toggle-on={enabled}
												onclick={() => togglePlugin(plugin.id, !enabled)}
											>
												<span class="plugin-toggle-thumb"></span>
											</button>
										</label>
									{/each}
								</div>
								<p class="plugins-hint">Disabled plugins won't show their control section on the control page for this program.</p>
							{:else}
								<div class="sub-card">
									<p class="sub-empty">
										No system-wide enabled plugins with controls.
										<a href="/plugin-editor" class="helper-link">Manage plugins →</a>
									</p>
								</div>
							{/if}

						{/if}
					</div>
				</div>
			{:else}
				<div class="empty-state">
					<svg xmlns="http://www.w3.org/2000/svg" height="64px" viewBox="0 -960 960 960" width="64px" fill="currentColor"><path d="M390-358.46 641.54-520 390-681.54v323.08ZM340-140v-80H172.31Q142-220 121-241q-21-21-21-51.31v-455.38Q100-778 121-799q21-21 51.31-21h615.38Q818-820 839-799q21 21 21 51.31v455.38Q860-262 839-241q-21 21-51.31 21H620v80H340ZM172.31-280h615.38q4.62 0 8.46-3.85 3.85-3.84 3.85-8.46v-455.38q0-4.62-3.85-8.46-3.84-3.85-8.46-3.85H172.31q-4.62 0-8.46 3.85-3.85 3.84-3.85 8.46v455.38q0 4.62 3.85 8.46 3.84 3.85 8.46 3.85ZM160-280v-480 480Z"/></svg>
					<h2>No program selected</h2>
					<p>Pick a program from the sidebar, or create a new one.</p>
					<button class="btn btn-primary btn-sm" onclick={openNew}>
						<svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
							<path d="M12 5v14M5 12h14"/>
						</svg>
						New Program
					</button>
				</div>
			{/if}
		</main>
	</div>
</div>

<!-- Add Screen Picker Modal -->
<Modal bind:open={addScreenModalOpen} title="Add Screens" width="700px">
	{#snippet footer()}
		<button class="btn btn-ghost" onclick={() => { addScreenModalOpen = false; }}>Done</button>
	{/snippet}
	<div class="picker-grid">
		{#each availableScreens as s (s.id)}
			<button class="picker-card" onclick={() => addScreenToProgram(s)} aria-label="Add {s.graphics_name}">
				<div class="picker-img-wrap">
					{#if s.graphics_path}
						<MediaPreview class="picker-img" src={imgUrl(s.graphics_path)} alt={s.graphics_name} />
					{:else}
						<span class="picker-empty">—</span>
					{/if}
				</div>
				<div class="picker-info">
					<div class="picker-name">{s.graphics_name}</div>
					<div class="picker-sub">{s.media_type} · {s.allow_popups ? 'PopUps OK' : 'No PopUps'}</div>
				</div>
				<div class="picker-add-btn">Add</div>
			</button>
		{:else}
			<p class="picker-empty-msg">No more screens available. <a href="/screen-editor" class="helper-link">Create one →</a></p>
		{/each}
	</div>
</Modal>

<!-- Add PopUp Picker Modal -->
<Modal bind:open={addPopUpModalOpen} title="Add PopUps" width="700px">
	{#snippet footer()}
		<button class="btn btn-ghost" onclick={() => { addPopUpModalOpen = false; }}>Done</button>
	{/snippet}
	<div class="picker-grid">
		{#each availablePopUps as popup (popup.id)}
			<button class="picker-card" onclick={() => addPopUpToProgram(popup)} aria-label="Add {popup.name}">
				<div class="picker-img-wrap">
					{#if popup.image_path}
						<MediaPreview class="picker-img" src={imgUrl(popup.image_path)} alt={popup.name} />
					{:else}
						<span class="picker-empty">—</span>
					{/if}
				</div>
				<div class="picker-info">
					<div class="picker-name">{popup.name}</div>
					<div class="picker-sub">{popup.sponsor_name || 'No sponsor'}</div>
				</div>
				<div class="picker-add-btn">Add</div>
			</button>
		{:else}
			<p class="picker-empty-msg">No more pop-ups available to add.</p>
		{/each}
	</div>
</Modal>

<style>
	/* ── Reorder buttons ── */
	.reorder-cell {
		width: 52px;
		padding: 0 4px !important;
		text-align: center;
		white-space: nowrap;
	}

	.btn-reorder {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 20px;
		height: 20px;
		padding: 0;
		border: 1px solid var(--border-1);
		border-radius: 4px;
		background: var(--surface-2);
		color: var(--text-2);
		cursor: pointer;
		transition: background 0.12s, color 0.12s, opacity 0.12s;
	}

	.btn-reorder:hover:not(:disabled) {
		background: var(--surface-3);
		color: var(--text-1);
	}

	.btn-reorder:disabled {
		opacity: 0.25;
		cursor: default;
	}

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

	.sidebar-list {
		flex: 1;
		overflow-y: auto;
	}

	.sidebar-item {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px 14px;
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

	.item-thumb-wrap {
		flex-shrink: 0;
		width: 48px;
		height: 30px;
		border-radius: var(--r-sm);
		overflow: hidden;
		background: var(--surface-3);
		display: flex;
		align-items: center;
		justify-content: center;
	}

	:global(.item-thumb-img) {
		width: 48px;
		height: 30px;
		object-fit: cover;
	}

	.item-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
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

	.header-actions {
		display: flex;
		align-items: center;
		gap: 8px;
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
		box-sizing: border-box;
	}

	.form-input:focus {
		border-color: var(--accent);
	}

	.form-input:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.form-select {
		padding: 6px 8px;
		background: var(--surface-2);
		border: 1px solid var(--border-1);
		border-radius: var(--r-sm);
		color: var(--text-1);
		font-size: 12px;
		font-family: inherit;
		outline: none;
		width: 100%;
		cursor: pointer;
		transition: border-color 0.15s;
	}

	.form-select:focus {
		border-color: var(--accent);
	}

	.number-input {
		padding: 6px 8px;
		font-size: 13px;
		text-align: center;
		-moz-appearance: textfield;
		appearance: textfield;
	}

	.number-input::-webkit-inner-spin-button,
	.number-input::-webkit-outer-spin-button {
		-webkit-appearance: none;
		margin: 0;
	}

	/* ── Logo / Background image pair ── */
	.image-pair {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 20px;
	}

	.image-preview-box {
		width: 100%;
		aspect-ratio: 3/2;
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid var(--border-1);
		border-radius: var(--r);
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
	}

	:global(.image-preview-box img), :global(.image-preview-box video) {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
	}

	.img-placeholder {
		color: var(--text-3);
		font-size: 1.8rem;
	}

	.img-actions {
		display: flex;
		gap: 8px;
	}

	/* ── Sub-card (screens/popups container) ── */
	.sub-card {
		background: var(--surface-1);
		border: 1px solid var(--border-1);
		border-radius: var(--r);
		overflow: hidden;
	}

	/* ── Screens table ── */
	.preview-cell {
		text-align: center;
	}

	:global(.graphic-thumb) {
		max-height: 48px;
		max-width: 96px;
		object-fit: contain;
		border-radius: 4px;
	}

	/* ── PopUps table ── */
	.popup-row-info {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.popup-thumb-wrap {
		width: 72px;
		height: 40px;
		background: rgba(255, 255, 255, 0.05);
		border-radius: 4px;
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
		flex-shrink: 0;
	}

	:global(.popup-thumb) {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
	}

	.popups-table :global(td) {
		vertical-align: middle;
	}

	/* ── Picker grid (Add Screen / Add PopUp modals) ── */
	.picker-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
		gap: 14px;
	}

	.picker-card {
		background: var(--surface-2);
		border: 1px solid var(--border-1);
		border-radius: var(--r);
		overflow: hidden;
		display: flex;
		flex-direction: column;
		cursor: pointer;
		transition: border-color 0.15s, transform 0.15s;
		position: relative;
		text-align: left;
		padding: 0;
		font-family: inherit;
		outline: none;
	}

	.picker-card:hover, .picker-card:focus-visible {
		border-color: var(--accent);
		transform: translateY(-2px);
	}

	.picker-img-wrap {
		width: 100%;
		aspect-ratio: 16/9;
		background: rgba(255, 255, 255, 0.05);
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
		border-bottom: 1px solid var(--border-1);
	}

	:global(.picker-img) {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
	}

	.picker-info {
		padding: 10px 12px;
		flex: 1;
	}

	.picker-add-btn {
		position: absolute;
		bottom: 8px;
		right: 8px;
		opacity: 0;
		transition: opacity 0.15s;
		background: var(--accent);
		color: #000;
		padding: 3px 8px;
		font-size: 11px;
		font-weight: 700;
		border-radius: var(--r-sm);
	}

	.picker-card:hover .picker-add-btn,
	.picker-card:focus-visible .picker-add-btn {
		opacity: 1;
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

	/* ── Unsaved badge ── */
	.unsaved-badge {
		display: inline-flex;
		align-items: center;
		padding: 2px 8px;
		font-size: 10px;
		font-weight: 700;
		letter-spacing: 0.04em;
		text-transform: uppercase;
		background: color-mix(in srgb, var(--accent) 15%, transparent);
		color: var(--accent);
		border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
		border-radius: 20px;
	}

	/* ── Tab nav ── */
	.tab-nav {
		display: flex;
		gap: 4px;
		border-bottom: 1px solid var(--border-1);
		padding-bottom: 0;
		margin-bottom: 4px;
	}

	.tab-btn {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 8px 14px;
		font-size: 13px;
		font-weight: 500;
		font-family: inherit;
		color: var(--text-3);
		background: transparent;
		border: none;
		border-bottom: 2px solid transparent;
		cursor: pointer;
		transition: color 0.15s, border-color 0.15s;
		margin-bottom: -1px;
	}

	.tab-btn:hover {
		color: var(--text-1);
	}

	.tab-btn.active {
		color: var(--text-1);
		border-bottom-color: var(--accent);
	}

	.tab-pill {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 1px 6px;
		font-size: 10px;
		font-weight: 700;
		background: var(--accent-dim);
		color: var(--accent);
		border-radius: 20px;
	}

	/* ── Plugin list ── */
	.plugin-list {
		display: flex;
		flex-direction: column;
		divide-y: 1px solid var(--border-1);
	}

	.plugin-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		padding: 12px 16px;
		cursor: pointer;
		transition: background 0.1s;
		border-bottom: 1px solid var(--border-1);
	}

	.plugin-row:last-child {
		border-bottom: none;
	}

	.plugin-row:hover {
		background: var(--surface-2);
	}

	.plugin-row-disabled .plugin-row-name {
		color: var(--text-3);
	}

	.plugin-row-info {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}

	.plugin-row-name {
		font-size: 13px;
		font-weight: 600;
		color: var(--text-1);
		transition: color 0.15s;
	}

	.plugin-row-meta {
		font-size: 11px;
		color: var(--text-3);
	}

	/* Toggle switch */
	.plugin-toggle {
		flex-shrink: 0;
		position: relative;
		width: 36px;
		height: 20px;
		border-radius: 10px;
		border: none;
		background: var(--surface-3);
		cursor: pointer;
		padding: 0;
		transition: background 0.2s;
	}

	.plugin-toggle-on {
		background: var(--accent);
	}

	.plugin-toggle-thumb {
		position: absolute;
		top: 3px;
		left: 3px;
		width: 14px;
		height: 14px;
		border-radius: 50%;
		background: #fff;
		transition: transform 0.2s cubic-bezier(0.16, 1, 0.3, 1);
		pointer-events: none;
	}

	.plugin-toggle-on .plugin-toggle-thumb {
		transform: translateX(16px);
	}

	.plugins-hint {
		font-size: 11px;
		color: var(--text-3);
		margin: 0;
		padding-top: 6px;
	}
</style>
