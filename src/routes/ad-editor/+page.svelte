<script lang="ts">
	import { onMount } from 'svelte';
	import TopNav from '$lib/components/TopNav.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import ImageUpload from '$lib/components/ImageUpload.svelte';
	import { socket } from '$lib/socket';
	import { fetchAdvertisements, imgUrl } from '$lib/api';
	import { addToast } from '$lib/stores/toasts';
	import type { Advertisement } from '$lib/types';
	import MediaPreview from '$lib/components/MediaPreview.svelte';

	let ads = $state<Advertisement[]>([]);
	let modalOpen = $state(false);

	// Edit state (committed only on Save)
	let editId = $state<number | null>(null);
	let editName = $state('');
	let editSponsor = $state('');
	let editImagePath = $state<string | null>(null);
	let saving = $state(false);

	const isNew = $derived(editId === null);

	onMount(() => {
		fetchAdvertisements().then((data) => { ads = data; });

		socket.on('ad-created', (data: any) => {
			if (data.success) {
				ads = [...ads, data.ad];
				addToast('success', 'Ad created.');
			}
		});

		socket.on('ad-updated', (data: any) => {
			if (data.success) {
				ads = ads.map((a) => (a.id === data.ad.id ? data.ad : a));
				addToast('success', 'Ad saved.');
				modalOpen = false;
			}
		});

		socket.on('ad-deleted', (data: any) => {
			if (data.success) {
				ads = ads.filter((a) => a.id !== data.id);
				addToast('success', 'Ad deleted.');
			}
		});

		socket.on('update-ads', () => {
			fetchAdvertisements().then((data) => { ads = data; });
		});

		return () => {
			socket.off('ad-created');
			socket.off('ad-updated');
			socket.off('ad-deleted');
			socket.off('update-ads');
		};
	});

	function openNew() {
		editId = null;
		editName = '';
		editSponsor = '';
		editImagePath = null;
		modalOpen = true;
	}

	function openEdit(ad: Advertisement) {
		editId = ad.id;
		editName = ad.name;
		editSponsor = ad.sponsor_name ?? '';
		editImagePath = ad.image_path;
		modalOpen = true;
	}

	function deleteAd(ad: Advertisement) {
		if (!confirm(`Delete ad "${ad.name}"?`)) return;
		socket.emit('delete-ad', { id: ad.id });
	}

	function save() {
		if (!editName.trim()) {
			addToast('error', 'Ad name is required.');
			return;
		}
		saving = true;
		if (isNew) {
			socket.emit('create-ad', { name: editName.trim() });
			modalOpen = false;
		} else {
			socket.emit('edit-ad', {
				id: editId,
				name: editName.trim(),
				sponsor_name: editSponsor.trim(),
			});
		}
		saving = false;
	}
</script>

<div class="page-wrap">
	<TopNav back={{ href: '/studio-selector', label: 'Studios' }} />
	<main class="page-content">
		<div class="section-header">
			<h2 class="section-title">
				<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<rect x="2" y="7" width="20" height="14" rx="2"/><path d="M16 7V5a2 2 0 00-4 0v2"/>
				</svg>
				Advertisements
				<span class="badge">{ads.length}</span>
			</h2>
			<button class="btn btn-primary" onclick={openNew}>
				<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
					<path d="M12 5v14M5 12h14"/>
				</svg>
				New Ad
			</button>
		</div>

		<div class="card">
			<table class="data-table">
				<thead>
					<tr>
						<th style="width:36px">#</th>
						<th style="width:200px">Image</th>
						<th style="width:15%">Name</th>
						<th style="width:12%">Sponsor</th>
						<th>In Programs</th>
						<th style="width:120px">Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each ads as ad (ad.id)}
						<tr>
							<td><span class="id-cell">{ad.id}</span></td>
							<td>
								{#if ad.image_path}
									<MediaPreview class="ad-thumb" src={imgUrl(ad.image_path)} alt={ad.name} />
								{:else}
									<div class="no-img">—</div>
								{/if}
							</td>
							<td><span class="fw-medium">{ad.name}</span></td>
							<td>
								{#if ad.sponsor_name}
									<span class="sponsor-name">{ad.sponsor_name}</span>
								{:else}
									<span class="text-dim">—</span>
								{/if}
							</td>
							<td>
								{#if ad.programs && ad.programs.length > 0}
									<div class="prog-pills">
										{#each ad.programs.slice(0, 3) as p}
											<span class="prog-pill">{p.name}</span>
										{/each}
										{#if ad.programs.length > 3}
											<span class="text-dim">+{ad.programs.length - 3}</span>
										{/if}
									</div>
								{:else}
									<span class="text-dim">None</span>
								{/if}
							</td>
							<td>
								<div class="row-actions">
									<button class="btn btn-ghost btn-sm" onclick={() => openEdit(ad)}>Edit</button>
									<button class="btn btn-danger btn-sm" onclick={() => deleteAd(ad)}>Del</button>
								</div>
							</td>
						</tr>
					{:else}
						<tr>
							<td colspan="6" class="empty-row">No ads yet.</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	</main>
</div>

<!-- Edit Modal -->
<Modal bind:open={modalOpen} title={isNew ? 'New Advertisement' : `Edit — ${editName}`} width="520px">
	{#snippet footer()}
		<button class="btn btn-ghost" onclick={() => { modalOpen = false; }}>Cancel</button>
		<button class="btn btn-primary" onclick={save} disabled={saving}>
			{saving ? 'Saving…' : 'Save Ad'}
		</button>
	{/snippet}

	<div class="form-rows">
		<div class="form-row">
			<label class="form-label" for="ad-name">Ad Name</label>
			<input id="ad-name" class="form-input" type="text" bind:value={editName} placeholder="e.g. Summer Campaign" />
		</div>

		<div class="form-row">
			<label class="form-label" for="sponsor-name">Sponsor Name</label>
			<input id="sponsor-name" class="form-input" type="text" bind:value={editSponsor} placeholder="e.g. Acme Corp" />
		</div>

		<div class="form-row">
			<label class="form-label" for="ad-image-upload">Image</label>
			<ImageUpload
				inputId="ad-image-upload"
				endpoint="/advertisements/upload-image"
				id={editId!}
				currentPath={editImagePath}
				onuploaded={(path) => { editImagePath = path; }}
			/>
		</div>
	</div>
</Modal>

<style>
	/* Override page content to use full width */
	:global(.page-content) {
		max-width: none;
		padding: 24px;
	}

	.badge {
		background: var(--surface-3);
		color: var(--text-2);
		font-size: 11px;
		font-weight: 700;
		border-radius: 999px;
		padding: 2px 8px;
	}

	:global(.ad-thumb) {
		width: 350px;
		height: 60px;
		object-fit: contain;
		border-radius: var(--r-sm);
		background: var(--surface-2);
		border: 1px solid var(--border-1);
	}

	.no-img {
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

	.sponsor-name {
		font-size: 14px;
		color: var(--text-2);
	}

	.text-dim {
		font-size: 14px;
		color: var(--text-3);
	}

	.prog-pills {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
		align-items: center;
	}

	.prog-pill {
		font-size: 12px;
		font-weight: 500;
		background: var(--surface-3);
		color: var(--text-2);
		border-radius: 3px;
		padding: 2px 6px;
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

	.form-rows {
		display: flex;
		flex-direction: column;
		gap: 18px;
	}

	.form-row {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.form-label {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.07em;
		color: var(--text-3);
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
</style>
