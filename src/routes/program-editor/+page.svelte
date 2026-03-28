<script lang="ts">
	import { onMount } from 'svelte';
	import TopNav from '$lib/components/TopNav.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import { socket } from '$lib/socket';
	import { fetchPrograms, fetchAdvertisements, uploadProgramImage, imgUrl } from '$lib/api';
	import { addToast } from '$lib/stores/toasts';
	import type { Program, Advertisement, Graphic, ProgramAd } from '$lib/types';
	import MediaPreview from '$lib/components/MediaPreview.svelte';

	/* ─── State ─────────────────────────────────────────────── */
	let programs = $state<Program[]>([]);
	let allAds = $state<Advertisement[]>([]);
	let modalOpen = $state(false);
	let addAdModalOpen = $state(false);

	// Editing
	let editId = $state<number | null>(null);
	let editName = $state('');
	let editLogoPath = $state<string | null>(null);
	let editBgPath = $state<string | null>(null);
	let editGraphics = $state<Graphic[]>([]);
	/** Full ProgramAd entries held locally; saved on "Save Program" */
	let editProgramAds = $state<ProgramAd[]>([]);

	let saving = $state(false);
	let uploadingGraphicId = $state<number | null>(null); // which graphic row is uploading

	const isNew = $derived(editId === null);

	/* ─── Lifecycle ──────────────────────────────────────────── */
	onMount(() => {
		Promise.all([fetchPrograms(), fetchAdvertisements()]).then(([p, a]) => {
			programs = p;
			allAds = a;
		});

		socket.on('program-created', (data: any) => {
			if (data.success) {
				programs = [data.program, ...programs];
				addToast('success', 'Program created.');
			}
		});

		socket.on('program-updated', (data: any) => {
			if (data.success) {
				programs = programs.map((p) => (p.id === data.program.id ? data.program : p));
				addToast('success', 'Program saved.');
				modalOpen = false;
			}
		});

		socket.on('program-deleted', (data: any) => {
			if (data.success) {
				programs = programs.filter((p) => p.id !== data.id);
				addToast('success', 'Program deleted.');
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

	/* ─── Modal open/close ───────────────────────────────────── */
	function openNew() {
		editId = null;
		editName = '';
		editLogoPath = null;
		editBgPath = null;
		editGraphics = [];
		editProgramAds = [];
		modalOpen = true;
	}

	function openEdit(p: Program) {
		editId = p.id;
		editName = p.name;
		editLogoPath = p.logo_path;
		editBgPath = p.background_graphics_path;
		editGraphics = p.graphics.map((g) => ({ ...g }));
		// Deep copy program_ads so edits stay local
		editProgramAds = p.program_ads.map((pa) => ({ ...pa }));
		modalOpen = true;
	}

	/* ─── Program CRUD ───────────────────────────────────────── */
	function deleteProgram(p: Program) {
		if (!confirm(`Delete program "${p.name}"?`)) return;
		socket.emit('delete-program', { id: p.id });
	}

	function save() {
		if (!editName.trim()) {
			addToast('error', 'Program name is required.');
			return;
		}
		saving = true;
		if (isNew) {
			socket.emit('create-program', { name: editName.trim() });
			modalOpen = false;
			saving = false;
		} else {
			socket.emit('edit-program', {
				id: editId,
				name: editName.trim(),
				logo_path: editLogoPath,
				background_graphics_path: editBgPath,
				graphics: editGraphics,
				ads: editProgramAds.map((pa) => ({
					ad_id: pa.ad_id,
					ad_launch_type: pa.ad_launch_type,
					duration: pa.duration,
					frequency: pa.frequency,
				})),
			});
			saving = false;
		}
	}

	/* ─── Graphics helpers ───────────────────────────────────── */
	function addGraphic() {
		editGraphics = [
			...editGraphics,
			{
				id: -(Date.now()),
				graphics_name: `Graphic ${editGraphics.length + 1}`,
				graphics_path: '',
				allow_ads: true,
				program_id: editId ?? 0,
			},
		];
	}

	function removeGraphic(idx: number) {
		editGraphics = editGraphics.filter((_, i) => i !== idx);
	}

	async function uploadGraphicImage(e: Event, graphic: Graphic) {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (!file || editId == null) return;
		uploadingGraphicId = graphic.id;
		try {
			const result = await uploadProgramImage(file, editId, 'graphic', graphic.id > 0 ? graphic.id : undefined);
			if (result.success) {
				editGraphics = editGraphics.map((g) =>
					g.id === graphic.id ? { ...g, graphics_path: result.imagePath } : g
				);
				addToast('success', 'Graphic image uploaded.');
			} else {
				addToast('error', 'Upload failed.');
			}
		} catch {
			addToast('error', 'Upload failed.');
		} finally {
			uploadingGraphicId = null;
		}
	}

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
		if (editProgramAds.some((pa) => pa.ad_id === ad.id)) return; // already added
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

	/** Ads not yet added to the program */
	const availableAds = $derived(
		allAds.filter((a) => !editProgramAds.some((pa) => pa.ad_id === a.id))
	);

	/* ─── Image file input helpers ─────────────────────────────── */
	let logoInput = $state() as unknown as HTMLInputElement;
	let bgInput = $state() as unknown as HTMLInputElement;
</script>

<div class="page-wrap">
	<TopNav back={{ href: '/studio-selector', label: 'Studios' }} />
	<main class="page-content">
		<div class="section-header">
			<h2 class="section-title">
				<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8M12 17v4"/>
				</svg>
				Programs
				<span class="badge">{programs.length}</span>
			</h2>
			<button class="btn btn-primary" onclick={openNew}>
				<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
					<path d="M12 5v14M5 12h14"/>
				</svg>
				New Program
			</button>
		</div>

		<div class="card">
			<table class="data-table">
				<thead>
					<tr>
						<th style="width:36px">#</th>
						<th style="width:120px">Logo</th>
						<th>Name</th>
						<th>Graphics</th>
						<th>Ads</th>
						<th style="width:100px">Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each programs as p (p.id)}
						<tr>
							<td><span class="id-cell">{p.id}</span></td>
							<td>
								{#if p.logo_path}
									<MediaPreview class="logo-thumb" src={imgUrl(p.logo_path)} alt={p.name} />
								{:else}
									<div class="logo-empty">—</div>
								{/if}
							</td>
							<td><span class="fw-medium">{p.name}</span></td>
							<td><span class="count-badge">{p.graphics.length}</span></td>
							<td><span class="count-badge">{p.program_ads.length}</span></td>
							<td>
								<div class="row-actions">
									<button class="btn btn-ghost btn-sm" onclick={() => openEdit(p)}>Edit</button>
									<button class="btn btn-danger btn-sm" onclick={() => deleteProgram(p)}>Del</button>
								</div>
							</td>
						</tr>
					{:else}
						<tr>
							<td colspan="6" class="empty-row">No programs yet.</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	</main>
</div>

<!-- Edit Modal -->
<Modal bind:open={modalOpen} title={isNew ? 'New Program' : `Edit — ${editName}`} width="960px">
	{#snippet footer()}
		<button class="btn btn-ghost" onclick={() => { modalOpen = false; }}>Cancel</button>
		<button class="btn btn-primary" onclick={save} disabled={saving}>
			{saving ? 'Saving…' : 'Save Program'}
		</button>
	{/snippet}

	<div class="form-rows">
		<!-- ── Program Name ── -->
		<div class="form-row">
			<label class="form-label" for="program-name">Program Name</label>
			<input id="program-name" class="form-input" type="text" bind:value={editName} placeholder="e.g. Morning News" />
		</div>

		{#if !isNew}
			<!-- ── Logo + Background side by side ── -->
			<div class="image-pair">
				<!-- Logo -->
				<div class="form-row">
					<label class="form-label" for="logo-upload">Program Logo</label>
					<div class="image-preview-box" class:has-image={editLogoPath}>
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

				<!-- Background -->
				<div class="form-row">
					<label class="form-label" for="bg-upload">Background Graphic</label>
					<div class="image-preview-box" class:has-image={editBgPath}>
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

			<!-- ── Graphics & Overlays ── -->
			<div class="form-row">
				<div class="sub-section-header">
					<span class="form-label">Graphics &amp; Overlays <span class="label-count">{editGraphics.length}</span></span>
					<button class="btn btn-secondary btn-sm" onclick={addGraphic}>
						<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M12 5v14M5 12h14"/></svg>
						Add Graphic
					</button>
				</div>
				<div class="sub-card">
					{#if editGraphics.length > 0}
						<table class="data-table">
							<thead>
								<tr>
									<th>Name</th>
									<th style="width:110px">Preview</th>
									<th style="width:90px">Upload</th>
									<th style="width:80px">Allow Ads</th>
									<th style="width:44px"></th>
								</tr>
							</thead>
							<tbody>
								{#each editGraphics as g, i (g.id)}
									<tr>
										<td>
											<input
												class="form-input-inline"
												type="text"
												bind:value={g.graphics_name}
												placeholder="Overlay name"
											/>
										</td>
										<td class="preview-cell">
											{#if g.graphics_path}
												<MediaPreview
													class="graphic-thumb"
													src={imgUrl(g.graphics_path)}
													alt={g.graphics_name}
												/>
											{:else}
												<span class="no-image-text">No image</span>
											{/if}
										</td>
										<td>
											{#if editId != null}
												<!-- svelte-ignore a11y_label_has_associated_control -->
												<label class="upload-btn-label btn btn-secondary btn-sm" title="Upload image">
													{uploadingGraphicId === g.id ? 'Uploading…' : 'Upload'}
													<input
														type="file"
														accept="image/*,video/webm"
														style="display:none"
														onchange={(e) => uploadGraphicImage(e, g)}
														disabled={uploadingGraphicId === g.id}
													/>
												</label>
											{/if}
										</td>
										<td>
											<label class="toggle">
												<input type="checkbox" bind:checked={g.allow_ads} aria-label="Allow ads on this graphic" />
												<span class="toggle-track"></span>
											</label>
										</td>
										<td>
											<button
												class="btn btn-danger btn-icon btn-sm"
												aria-label="Remove graphic"
												onclick={() => removeGraphic(i)}
											>
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
						<p class="sub-empty">No graphics added yet. Click "Add Graphic" to create one.</p>
					{/if}
				</div>
			</div>

			<!-- ── Advertisements ── -->
			<div class="form-row">
				<div class="sub-section-header">
					<span class="form-label">Advertisements <span class="label-count">{editProgramAds.length}</span></span>

					<!-- Add ad dropdown -->
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
									<th>Ad</th>
									<th style="width:140px">Launch Type</th>
									<th style="width:90px">Duration (s)</th>
									<th style="width:105px">Frequency (/hr)</th>
									<th style="width:44px"></th>
								</tr>
							</thead>
							<tbody>
								{#each editProgramAds as pa (pa.ad_id)}
									{@const adDetails = allAds.find((a) => a.id === pa.ad_id) ?? pa.ad}
									<tr>
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
												<option value="automatic_and_manual">Both</option>
											</select>
										</td>
										<td>
											<input
												class="form-input number-input"
												type="number"
												min="0"
												value={pa.duration}
												oninput={(e) => updateProgramAd(pa.ad_id, { duration: Math.max(0, Number((e.target as HTMLInputElement).value)) })}
											/>
										</td>
										<td>
											<input
												class="form-input number-input"
												type="number"
												min="0"
												value={pa.frequency}
												disabled={pa.ad_launch_type === 'manual'}
												oninput={(e) => updateProgramAd(pa.ad_id, { frequency: Math.max(0, Number((e.target as HTMLInputElement).value)) })}
											/>
										</td>
										<td>
											<button
												class="btn btn-danger btn-icon btn-sm"
												aria-label="Remove ad"
												onclick={() => removeAdFromProgram(pa.ad_id)}
											>
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
						<p class="sub-empty">No ads added yet. Use "Add Ad…" above to select from available advertisements.</p>
					{/if}
				</div>
			</div>
		{/if}
	</div>
</Modal>

<!-- Add Ad Modal -->
<Modal bind:open={addAdModalOpen} title="Add Advertisements" width="700px">
	{#snippet footer()}
		<button class="btn btn-ghost" onclick={() => { addAdModalOpen = false; }}>Done</button>
	{/snippet}

	<div class="available-ads-grid">
		{#each availableAds as ad (ad.id)}
			<button class="ad-card" onclick={() => addAdToProgram(ad)} aria-label="Add {ad.name}">
				<div class="ad-card-img-wrap">
					{#if ad.image_path}
						<MediaPreview class="ad-card-img" src={imgUrl(ad.image_path)} alt={ad.name} />
					{:else}
						<span class="ad-card-empty">—</span>
					{/if}
				</div>
				<div class="ad-card-info">
					<div class="ad-card-name">{ad.name}</div>
					<div class="ad-card-sponsor">{ad.sponsor_name || 'No sponsor'}</div>
				</div>
				<div class="ad-card-add-btn">Add</div>
			</button>
		{:else}
			<p class="sub-empty">No more ads available to add.</p>
		{/each}
	</div>
</Modal>

<style>
	/* ── Table page styles ── */
	.badge {
		background: var(--surface-3);
		color: var(--text-2);
		font-size: 11px;
		font-weight: 700;
		border-radius: 999px;
		padding: 2px 8px;
		font-style: normal;
	}

	:global(.logo-thumb) {
		width: 120px;
		height: 60px;
		object-fit: contain;
		border-radius: var(--r-sm);
		background: var(--surface-2);
	}

	.logo-empty {
		color: var(--text-3);
		font-size: 12px;
	}

	.id-cell {
		font-size: 14px;
		color: var(--text-3);
		font-variant-numeric: tabular-nums;
	}

	.fw-medium {
		font-weight: 500;
		color: var(--text-1);
		font-size: 16px;
	}

	.count-badge {
		background: var(--surface-3);
		color: var(--text-2);
		font-size: 14px;
		font-weight: 600;
		border-radius: 4px;
		padding: 2px 7px;
	}

	.row-actions {
		display: flex;
		gap: 4px;
	}

	.empty-row {
		text-align: center;
		color: var(--text-3);
		font-size: 14px;
		padding: 24px !important;
	}

	/* ── Modal form layout ── */
	.form-rows {
		display: flex;
		flex-direction: column;
		gap: 20px;
	}

	.form-row {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.form-label {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.07em;
		color: var(--text-3);
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.label-count {
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
		/* strip native OS spinners */
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
		gap: 16px;
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
		font-size: 2rem;
	}

	.img-actions {
		display: flex;
		gap: 8px;
	}

	/* ── Sub-section header (label + action button in a row) ── */
	.sub-section-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	/* ── Sub-card ── */
	.sub-card {
		background: var(--surface-2);
		border: 1px solid var(--border-1);
		border-radius: var(--r);
		overflow: hidden;
	}

	.sub-empty {
		font-size: 12px;
		color: var(--text-3);
		padding: 16px;
		text-align: center;
	}

	/* ── Toggle switch ── */
	.toggle {
		display: flex;
		align-items: center;
		cursor: pointer;
	}

	.toggle input {
		display: none;
	}

	.toggle-track {
		width: 28px;
		height: 16px;
		background: var(--surface-3);
		border: 1px solid var(--border-2);
		border-radius: 999px;
		position: relative;
		transition: all 0.15s;
	}

	.toggle-track::after {
		content: '';
		position: absolute;
		top: 2px;
		left: 2px;
		width: 10px;
		height: 10px;
		border-radius: 50%;
		background: var(--text-3);
		transition: all 0.15s;
	}

	.toggle input:checked + .toggle-track {
		background: var(--accent-dim);
		border-color: var(--accent);
	}

	.toggle input:checked + .toggle-track::after {
		background: var(--accent);
		transform: translateX(12px);
	}

	/* ── Graphics table ── */
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

	.upload-btn-label {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		cursor: pointer;
	}

	/* ── Add Ad Modal ── */
	.available-ads-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
		gap: 16px;
	}

	.ad-card {
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

	.ad-card:hover, .ad-card:focus-visible {
		border-color: var(--accent);
		transform: translateY(-2px);
	}

	.ad-card-img-wrap {
		width: 100%;
		aspect-ratio: 16/9;
		background: rgba(255, 255, 255, 0.05);
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
		border-bottom: 1px solid var(--border-1);
	}

	:global(.ad-card-img) {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
	}

	.ad-card-empty {
		color: var(--text-3);
		font-size: 2rem;
	}

	.ad-card-info {
		padding: 12px;
		flex: 1;
	}

	.ad-card-name {
		font-weight: 600;
		font-size: 13px;
		color: var(--text-1);
		margin-bottom: 2px;
	}

	.ad-card-sponsor {
		font-size: 11px;
		color: var(--text-3);
	}

	.ad-card-add-btn {
		position: absolute;
		bottom: 8px;
		right: 8px;
		opacity: 0;
		transition: opacity 0.15s;
		/* match btn btn-secondary btn-sm */
		background: var(--surface-3);
		color: var(--text-1);
		border: 1px solid var(--border-2);
		padding: 4px 8px;
		font-size: 11px;
		font-weight: 600;
		border-radius: var(--r-sm);
	}

	.ad-card:hover .ad-card-add-btn, .ad-card:focus-visible .ad-card-add-btn {
		opacity: 1;
	}

	.helper-link {
		font-size: 12px;
		color: var(--accent);
		text-decoration: none;
	}

	.helper-link:hover {
		text-decoration: underline;
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

	/* Make ads table a bit roomier */
	.ads-table :global(td) {
		vertical-align: middle;
	}
</style>
