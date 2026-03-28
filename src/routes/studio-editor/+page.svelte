<script lang="ts">
	import { onMount } from 'svelte';
	import TopNav from '$lib/components/TopNav.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import { socket, BACKEND_URL } from '$lib/socket';
	import { fetchStudios } from '$lib/api';
	import { addToast } from '$lib/stores/toasts';
	import type { Studio, ObsCommand } from '$lib/types';

	const SHORTCUTS = ['F13','F14','F15','F16','F17','F18','F19','F20','F21','F22','F23','F24'];

	let studios = $state<Studio[]>([]);
	let modalOpen = $state(false);
	let saving = $state(false);
	let copied = $state(false);

	// Edit state (committed only on Save)
	let editId = $state<number | null>(null);
	let editName = $state('');
	let editBrowserAddr = $state('');
	let editCommands = $state<ObsCommand[]>([]);

	const isNew = $derived(editId === null);

	onMount(() => {
		fetchStudios().then((data) => { studios = data; });

		socket.on('studio-created', (data: any) => {
			if (data.success) {
				studios = [...studios, data.studio];
				addToast('success', 'Studio created.');
			}
		});

		socket.on('studio-updated', (data: any) => {
			if (data.success) {
				studios = studios.map((s) => (s.id === data.studio.id ? data.studio : s));
				addToast('success', 'Studio saved.');
				modalOpen = false;
			}
		});

		socket.on('studio-deleted', (data: any) => {
			if (data.success) {
				studios = studios.filter((s) => s.id !== data.id);
				addToast('success', 'Studio deleted.');
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
		editId = null;
		editName = '';
		editBrowserAddr = '';
		editCommands = [];
		modalOpen = true;
	}

	function openEdit(s: Studio) {
		editId = s.id;
		editName = s.name;
		editBrowserAddr = s.obs_browser_source_address;
		editCommands = s.commands.map((c) => ({ ...c }));
		modalOpen = true;
	}

	function deleteStudio(s: Studio) {
		if (!confirm(`Delete studio "${s.name}"?`)) return;
		socket.emit('delete-studio', { id: s.id });
	}

	function addCommand() {
		editCommands = [
			...editCommands,
			{
				id: null,
				obs_command_name: 'New Command',
				obs_command_color: '#38bdf8',
				obs_command_description: '',
				obs_command_shortcut: SHORTCUTS[editCommands.length % SHORTCUTS.length],
			},
		];
	}

	function removeCommand(i: number) {
		editCommands = editCommands.filter((_, idx) => idx !== i);
	}

	function save() {
		if (!editName.trim()) {
			addToast('error', 'Studio name is required.');
			return;
		}
		saving = true;
		if (isNew) {
			socket.emit('create-studio', { name: editName.trim() });
			modalOpen = false;
		} else {
			socket.emit('edit-studio', {
				id: editId,
				name: editName.trim(),
				obs_commands: editCommands,
			});
		}
		saving = false;
	}

	async function copyAddr() {
		const full = `${BACKEND_URL}${editBrowserAddr}`;
		await navigator.clipboard.writeText(full);
		copied = true;
		setTimeout(() => (copied = false), 2000);
	}
</script>

<div class="page-wrap">
	<TopNav back={{ href: '/studio-selector', label: 'Studios' }} />
	<main class="page-content">
		<div class="section-header">
			<h2 class="section-title">
				<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M15 10l4.553-2.069A1 1 0 0121 8.82V15.18a1 1 0 01-1.447.893L15 14M3 8a2 2 0 012-2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V8z"/>
				</svg>
				Studios
				<span class="badge">{studios.length}</span>
			</h2>
			<button class="btn btn-primary" onclick={openNew}>
				<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
					<path d="M12 5v14M5 12h14"/>
				</svg>
				New Studio
			</button>
		</div>

		<div class="card">
			<table class="data-table">
				<thead>
					<tr>
						<th style="width:36px">#</th>
						<th>Name</th>
						<th>OBS Source URL</th>
						<th>Commands</th>
						<th style="width:100px">Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each studios as s (s.id)}
						<tr>
							<td><span class="id-cell">{s.id}</span></td>
							<td><span class="fw-medium">{s.name}</span></td>
							<td>
								<span class="mono-url">{BACKEND_URL}{s.obs_browser_source_address}</span>
							</td>
							<td><span class="count-badge">{s.commands.length}</span></td>
							<td>
								<div class="row-actions">
									<button class="btn btn-ghost btn-sm" onclick={() => openEdit(s)}>Edit</button>
									<button class="btn btn-danger btn-sm" onclick={() => deleteStudio(s)}>Del</button>
								</div>
							</td>
						</tr>
					{:else}
						<tr>
							<td colspan="5" class="empty-row">No studios yet.</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	</main>
</div>

<!-- Edit Modal -->
<Modal bind:open={modalOpen} title={isNew ? 'New Studio' : `Edit — ${editName}`} width="700px">
	{#snippet footer()}
		<button class="btn btn-ghost" onclick={() => { modalOpen = false; }}>Cancel</button>
		<button class="btn btn-primary" onclick={save} disabled={saving}>
			{saving ? 'Saving…' : 'Save Studio'}
		</button>
	{/snippet}

	<div class="form-rows">
		<!-- Name -->
		<div class="form-row">
			<label class="form-label" for="studio-name">Studio Name</label>
			<input id="studio-name" class="form-input" type="text" bind:value={editName} placeholder="e.g. Studio A" />
		</div>

		{#if !isNew}
			<!-- OBS Source URL -->
			<div class="form-row">
				<span class="form-label">OBS Browser Source URL</span>
				<div class="copy-row">
					<span class="mono-url url-display">{BACKEND_URL}{editBrowserAddr}</span>
					<button class="btn btn-ghost btn-sm" onclick={copyAddr}>
						{copied ? '✓ Copied' : 'Copy'}
					</button>
				</div>
				<p class="helper-text">Paste this URL into your OBS browser source.</p>
			</div>

			<!-- Commands table -->
			<div class="form-row">
				<div class="cmds-header">
					<span class="form-label">
						OBS Commands
						<span class="label-count">{editCommands.length}</span>
					</span>
					<button class="btn btn-secondary btn-sm" onclick={addCommand}>+ Add Command</button>
				</div>

				<div class="sub-card">
					{#if editCommands.length > 0}
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
								{#each editCommands as cmd, i}
									<tr>
										<td>
											<input
												class="form-input-inline"
												type="text"
												bind:value={cmd.obs_command_name}
												placeholder="Command name"
											/>
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
											<input
												class="form-input-inline"
												type="text"
												bind:value={cmd.obs_command_description}
												placeholder="Optional description"
											/>
										</td>
										<td>
											<button class="btn btn-danger btn-icon btn-sm" aria-label="Remove command" onclick={() => removeCommand(i)}>
												<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
													<path d="M18 6L6 18M6 6l12 12"/>
												</svg>
											</button>
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					{:else}
						<div class="empty-cmds">No commands yet. Add one above.</div>
					{/if}
				</div>
			</div>
		{/if}
	</div>
</Modal>

<style>
	.badge {
		background: var(--surface-3);
		color: var(--text-2);
		font-size: 11px;
		font-weight: 700;
		border-radius: 999px;
		padding: 2px 8px;
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

	.mono-url {
		font-size: 14px;
		color: var(--text-3);
		font-family: ui-monospace, 'Courier New', monospace;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: 320px;
		display: inline-block;
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

	/* Form */
	.form-rows {
		display: flex;
		flex-direction: column;
		gap: 20px;
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
	}

	.form-input:focus {
		border-color: var(--accent);
	}

	.form-input-inline {
		background: transparent;
		border: 1px solid transparent;
		color: var(--text-1);
		font-size: 14px;
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
		font-size: 13px;
		font-family: inherit;
		padding: 4px 6px;
		border-radius: var(--r-sm);
		width: 100%;
		outline: none;
		cursor: pointer;
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

	.url-display {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		display: block;
	}

	.helper-text {
		font-size: 11px;
		color: var(--text-3);
	}

	.cmds-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.sub-card {
		background: var(--surface-2);
		border: 1px solid var(--border-1);
		border-radius: var(--r);
		overflow: hidden;
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

	.empty-cmds {
		padding: 20px;
		text-align: center;
		font-size: 12px;
		color: var(--text-3);
	}
</style>
