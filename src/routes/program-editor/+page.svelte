<script lang="ts">
	import { onMount } from 'svelte';
	import TopNav from '$lib/components/TitleBarWeb.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import { socket } from '$lib/api/socket';
	import { fetchPrograms, fetchAdvertisements, fetchScreens, uploadProgramImage, imgUrl } from '$lib/api/api';
	import { addToast } from '$lib/toasts';
	import type { Program, Advertisement, Screen, ProgramAd } from '$lib/types';
	import MediaPreview from '$lib/components/MediaPreview.svelte';
	import { getBackendUrl } from '$lib/bridge';
	import { IS_TAURI } from '$lib/bridge';

	/* ─── State ─────────────────────────────────────────────── */
	let programs = $state<Program[]>([]);
	let allAds = $state<Advertisement[]>([]);
	let allScreens = $state<Screen[]>([]);
	let addAdModalOpen = $state(false);
	let addScreenModalOpen = $state(false);

	// Selection state
	let selectedId = $state<number | null>(null);
	let isCreatingNew = $state(false);

	// Edit state
	let editId = $state<number | null>(null);
	let editName = $state('');
	let editLogoPath = $state<string | null>(null);
	let editBgPath = $state<string | null>(null);
	let editScreenIds = $state<number[]>([]);
	let editProgramAds = $state<ProgramAd[]>([]);

	let saving = $state(false);

	const isNew = $derived(isCreatingNew);
	const hasSelection = $derived(isCreatingNew || selectedId !== null);

	const editScreens = $derived<Screen[]>(
		editScreenIds.map((id) => allScreens.find((s) => s.id === id)).filter((s): s is Screen => !!s)
	);

	/* ─── Lifecycle ──────────────────────────────────────────── */
	onMount(() => {
		Promise.all([fetchPrograms(), fetchAdvertisements(), fetchScreens()]).then(([p, a, s]) => {
			programs = p;
			allAds = a;
			allScreens = s;
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
					editProgramAds = [];
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
	function openNew() {
		isCreatingNew = true;
		selectedId = null;
		editId = null;
		editName = '';
		editLogoPath = null;
		editBgPath = null;
		editScreenIds = [];
		editProgramAds = [];
	}

	function selectProgram(p: Program) {
		isCreatingNew = false;
		selectedId = p.id;
		editId = p.id;
		editName = p.name;
		editLogoPath = p.logo_path;
		editBgPath = p.background_graphics_path;
		editScreenIds = p.graphics.map((g) => g.id);
		editProgramAds = p.program_ads.map((pa) => ({ ...pa }));
	}

	/* ─── Program CRUD ───────────────────────────────────────── */
	async function deleteCurrentProgram() {
		const program = programs.find((p) => p.id === selectedId);
		if (!program) return;
		if (!confirm(`Delete program "${program.name}"?`)) return;
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
					editProgramAds = [];
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
						ads: editProgramAds.map((pa) => ({
							ad_id: pa.ad_id,
							ad_launch_type: pa.ad_launch_type,
							duration: pa.duration,
							frequency: pa.frequency,
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

	/* ─── Ads helpers ────────────────────────────────────────── */
	function addAdToProgram(ad: Advertisement) {
		if (editProgramAds.some((pa) => pa.ad_id === ad.id)) return;
		editProgramAds = [
			...editProgramAds,
			{
				id: -(Date.now()),
				ad_id: ad.id,
				program_id: editId ?? 0,
				ad_launch_type: 'manual',
				duration: 10,
				frequency: 1,
				ad,
			},
		];
	}

	function removeAdFromProgram(adId: number) {
		const pa = editProgramAds.find((a) => a.ad_id === adId);
		const adName = pa?.ad?.name ?? `Ad ${adId}`;
		if (!confirm(`Remove "${adName}" from this program?`)) return;
		editProgramAds = editProgramAds.filter((a) => a.ad_id !== adId);
	}

	function updateProgramAd(adId: number, patch: Partial<ProgramAd>) {
		editProgramAds = editProgramAds.map((pa) =>
			pa.ad_id === adId ? { ...pa, ...patch } : pa
		);
	}

	const availableAds = $derived(
		allAds.filter((a) => !editProgramAds.some((pa) => pa.ad_id === a.id))
	);

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

	/* ─── Reorder: Ads ───────────────────────────────────────────── */
	function moveAdUp(i: number) {
		if (i === 0) return;
		const ads = [...editProgramAds];
		[ads[i - 1], ads[i]] = [ads[i], ads[i - 1]];
		editProgramAds = ads;
	}

	function moveAdDown(i: number) {
		if (i === editProgramAds.length - 1) return;
		const ads = [...editProgramAds];
		[ads[i], ads[i + 1]] = [ads[i + 1], ads[i]];
		editProgramAds = ads;
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
							<span class="item-meta">{p.graphics.length} screens · {p.program_ads.length} ads</span>
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
							{#if !isNew}
								<span class="panel-id">ID #{editId}</span>
							{/if}
						</div>
						<div class="panel-actions">
							{#if !isNew}
								<button class="btn btn-danger btn-sm" onclick={deleteCurrentProgram}>Delete</button>
							{/if}
							<button class="btn btn-ghost btn-sm" onclick={() => { selectedId = null; isCreatingNew = false; }}>
								Cancel
							</button>
							<button class="btn btn-primary" onclick={save} disabled={saving}>
								{saving ? 'Saving…' : isNew ? 'Create Program' : 'Save Changes'}
							</button>
						</div>
					</div>

					<div class="form-body">
						<!-- Program Name -->
						<div class="field-group">
							<label class="field-label" for="program-name">Program Name</label>
							<input id="program-name" class="form-input" type="text" bind:value={editName} placeholder="e.g. Morning News" />
						</div>

						{#if !isNew}
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
									<label class="field-label" for="bg-upload">Background Graphic</label>
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

							<!-- Screens & Overlays -->
							<div class="field-group">
								<div class="field-group-header">
									<span class="field-label">
										Screens &amp; Overlays
										<span class="badge-sm">{editScreenIds.length}</span>
									</span>
									<div class="header-actions">
										<button class="btn btn-secondary btn-sm" onclick={() => { addScreenModalOpen = true; }}>
											<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M12 5v14M5 12h14"/></svg>
											Add Screen…
										</button>
										<a href="/screen-editor" class="helper-link" target="_blank">Manage →</a>
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
													<th style="width:80px">Allow Ads</th>
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
														<td><span class="badge-sm">{s.allow_ads ? 'Yes' : 'No'}</span></td>
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

							<!-- Advertisements -->
							<div class="field-group">
								<div class="field-group-header">
									<span class="field-label">
										Advertisements
										<span class="badge-sm">{editProgramAds.length}</span>
									</span>
									{#if availableAds.length > 0}
										<button class="btn btn-secondary btn-sm" onclick={() => { addAdModalOpen = true; }}>
											<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M12 5v14M5 12h14"/></svg>
											Add Ad…
										</button>
									{:else if allAds.length === 0}
										<a href="/ad-editor" class="helper-link">Create ads first →</a>
									{/if}
								</div>
								<div class="sub-card">
									{#if editProgramAds.length > 0}
										<table class="data-table ads-table">
											<thead>
												<tr>
													<th style="width:52px"></th>
													<th>Ad</th>
													<th style="width:140px">Launch Type</th>
													<th style="width:90px">Duration (s)</th>
													<th style="width:105px">Frequency (/hr)</th>
													<th style="width:44px"></th>
												</tr>
											</thead>
											<tbody>
												{#each editProgramAds as pa, i (pa.ad_id)}
													{@const adDetails = allAds.find((a) => a.id === pa.ad_id) ?? pa.ad}
													<tr>
														<td class="reorder-cell">
															<button class="btn-reorder" aria-label="Move up" disabled={i === 0} onclick={() => moveAdUp(i)}>
																<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M18 15l-6-6-6 6"/></svg>
															</button>
															<button class="btn-reorder" aria-label="Move down" disabled={i === editProgramAds.length - 1} onclick={() => moveAdDown(i)}>
																<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M6 9l6 6 6-6"/></svg>
															</button>
														</td>
														<td>
															<div class="ad-row-info">
																<div class="ad-thumb-wrap">
																	{#if adDetails?.image_path}
																		<MediaPreview class="ad-thumb" src={imgUrl(adDetails.image_path)} alt={adDetails?.name} />
																	{:else}
																		<span class="ad-thumb-empty">—</span>
																	{/if}
																</div>
																<div>
																	<div class="ad-name-text">{adDetails?.name ?? `Ad ${pa.ad_id}`}</div>
																	<div class="ad-sponsor-text">{adDetails?.sponsor_name || 'No sponsor'}</div>
																</div>
															</div>
														</td>
														<td>
															<select
																class="form-select"
																value={pa.ad_launch_type}
																onchange={(e) => updateProgramAd(pa.ad_id, { ad_launch_type: (e.target as HTMLSelectElement).value as any })}
															>
																<option value="manual">Manual</option>
																<option value="automatic">Automatic</option>
																<option value="both">Both</option>
																<option value="filler">Filler</option>
															</select>
														</td>
														<td>
															<input
																class="form-input number-input"
																type="number"
																min="0"
																value={pa.duration}
																disabled={pa.ad_launch_type === 'filler'}
																oninput={(e) => updateProgramAd(pa.ad_id, { duration: Math.max(0, Number((e.target as HTMLInputElement).value)) })}
															/>
														</td>
														<td>
															<input
																class="form-input number-input"
																type="number"
																min="0"
																value={pa.frequency}
																disabled={pa.ad_launch_type === 'manual' || pa.ad_launch_type === 'filler'}
																oninput={(e) => updateProgramAd(pa.ad_id, { frequency: Math.max(0, Number((e.target as HTMLInputElement).value)) })}
															/>
														</td>
														<td>
															<button class="btn btn-danger btn-icon btn-sm" aria-label="Remove ad" onclick={() => removeAdFromProgram(pa.ad_id)}>
																<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M18 6L6 18M6 6l12 12"/></svg>
															</button>
														</td>
													</tr>
												{/each}
											</tbody>
										</table>
									{:else}
										<p class="sub-empty">No ads added yet. Use "Add Ad…" above.</p>
									{/if}
								</div>
							</div>
						{/if}
					</div>
				</div>
			{:else}
				<div class="empty-state">
					<svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.2" class="empty-icon">
						<rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8M12 17v4"/>
					</svg>
					<p class="empty-title">No program selected</p>
					<p class="empty-hint">Pick a program from the sidebar, or create a new one.</p>
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
					<div class="picker-sub">{s.media_type} · {s.allow_ads ? 'Ads OK' : 'No Ads'}</div>
				</div>
				<div class="picker-add-btn">Add</div>
			</button>
		{:else}
			<p class="picker-empty-msg">No more screens available. <a href="/screen-editor" class="helper-link">Create one →</a></p>
		{/each}
	</div>
</Modal>

<!-- Add Ad Picker Modal -->
<Modal bind:open={addAdModalOpen} title="Add Advertisements" width="700px">
	{#snippet footer()}
		<button class="btn btn-ghost" onclick={() => { addAdModalOpen = false; }}>Done</button>
	{/snippet}
	<div class="picker-grid">
		{#each availableAds as ad (ad.id)}
			<button class="picker-card" onclick={() => addAdToProgram(ad)} aria-label="Add {ad.name}">
				<div class="picker-img-wrap">
					{#if ad.image_path}
						<MediaPreview class="picker-img" src={imgUrl(ad.image_path)} alt={ad.name} />
					{:else}
						<span class="picker-empty">—</span>
					{/if}
				</div>
				<div class="picker-info">
					<div class="picker-name">{ad.name}</div>
					<div class="picker-sub">{ad.sponsor_name || 'No sponsor'}</div>
				</div>
				<div class="picker-add-btn">Add</div>
			</button>
		{:else}
			<p class="picker-empty-msg">No more ads available to add.</p>
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

	.item-thumb-empty {
		font-size: 11px;
		color: var(--text-3);
	}

	.item-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
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

	.header-actions {
		display: flex;
		align-items: center;
		gap: 8px;
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

	.helper-link {
		font-size: 12px;
		color: var(--accent);
		text-decoration: none;
	}

	.helper-link:hover {
		text-decoration: underline;
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

	/* ── Sub-card (screens/ads container) ── */
	.sub-card {
		background: var(--surface-1);
		border: 1px solid var(--border-1);
		border-radius: var(--r);
		overflow: hidden;
	}

	.sub-empty {
		font-size: 12px;
		color: var(--text-3);
		padding: 18px;
		text-align: center;
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

	.no-image-text {
		font-size: 11px;
		color: var(--text-3);
	}

	.fw-medium {
		font-weight: 500;
		color: var(--text-1);
	}

	/* ── Ads table ── */
	.ad-row-info {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.ad-thumb-wrap {
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

	:global(.ad-thumb) {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
	}

	.ad-thumb-empty {
		color: var(--text-3);
		font-size: 1rem;
	}

	.ad-name-text {
		font-weight: 600;
		font-size: 13px;
		color: var(--text-1);
	}

	.ad-sponsor-text {
		font-size: 11px;
		color: var(--text-3);
	}

	.ads-table :global(td) {
		vertical-align: middle;
	}

	/* ── Picker grid (Add Screen / Add Ad modals) ── */
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

	.picker-empty {
		color: var(--text-3);
		font-size: 1.5rem;
	}

	.picker-info {
		padding: 10px 12px;
		flex: 1;
	}

	.picker-name {
		font-weight: 600;
		font-size: 13px;
		color: var(--text-1);
		margin-bottom: 2px;
	}

	.picker-sub {
		font-size: 11px;
		color: var(--text-3);
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

	.picker-empty-msg {
		font-size: 12px;
		color: var(--text-3);
		padding: 20px;
		text-align: center;
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
