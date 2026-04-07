<script lang="ts">
	import { onMount } from 'svelte';
	import TopNav from '$lib/components/TitleBarWeb.svelte';
	import { hasData } from '$lib/api/api';
	import { BACKEND_URL } from '$lib/api/socket';
	import { addToast } from '$lib/toasts';
	import { IS_TAURI, getCurrentPort } from '$lib/bridge';

	type Tab = 'import-export' | 'server' | 'updates' | 'about';
	let activeTab = $state<Tab>('import-export');

	let appVersion = $state('');
	let canExport = $state(false);
	let importing = $state(false);
	let exporting = $state(false);
	let fileInput: HTMLInputElement;

	// Server tab state
	let currentPort = $state(0);

	// Updates tab state
	type UpdateStatus = 'idle' | 'checking' | 'up-to-date' | 'available';
	let updateStatus = $state<UpdateStatus>('idle');
	let updateVersion = $state<string | null>(null);
	let updateInstalling = $state(false);
	let updateError = $state<string | null>(null);
	let preferredPortInput = $state('');
	let portSaving = $state(false);
	let portSaved = $state(false);

	onMount(async () => {
		canExport = await hasData();
		currentPort = getCurrentPort();
		if (IS_TAURI) {
			const { getVersion } = await import('@tauri-apps/api/app');
			appVersion = await getVersion();

			// Load saved preferred port from Tauri
			const { invoke } = await import('@tauri-apps/api/core');
			const preferred: number | null = await invoke('get_preferred_port');
			if (preferred != null) preferredPortInput = String(preferred);
		} else {
			// Web/dev: load from localStorage
			const stored = localStorage.getItem('preferred_port');
			if (stored) preferredPortInput = stored;
		}
	});

	async function savePreferredPort() {
		const raw = String(preferredPortInput).trim();
		const portNum = raw === '' ? null : parseInt(raw, 10);

		if (portNum !== null && (isNaN(portNum) || portNum < 1024 || portNum > 65535)) {
			addToast('error', 'Port must be between 1024 and 65535.');
			return;
		}

		portSaving = true;
		try {
			if (IS_TAURI) {
				const { invoke } = await import('@tauri-apps/api/core');
				await invoke('set_preferred_port', { port: portNum });
			} else {
				if (portNum === null) {
					localStorage.removeItem('preferred_port');
				} else {
					localStorage.setItem('preferred_port', String(portNum));
				}
			}
			portSaved = true;
			setTimeout(() => (portSaved = false), 3000);
		} catch {
			addToast('error', 'Failed to save port preference.');
		} finally {
			portSaving = false;
		}
	}

	function resetPort() {
		preferredPortInput = '';
	}

	async function checkForUpdates() {
		if (!IS_TAURI) return;
		updateStatus = 'checking';
		updateError = null;
		try {
			const { invoke } = await import('@tauri-apps/api/core');
			const version: string | null = await invoke('check_for_updates');
			if (version) {
				updateVersion = version;
				updateStatus = 'available';
			} else {
				updateStatus = 'up-to-date';
			}
		} catch (e) {
			updateError = String(e);
			updateStatus = 'idle';
		}
	}

	async function installUpdate() {
		if (!IS_TAURI) return;
		updateInstalling = true;
		updateError = null;
		try {
			const { invoke } = await import('@tauri-apps/api/core');
			await invoke('install_update');
		} catch (e) {
			updateError = String(e);
			updateInstalling = false;
		}
	}

	async function doExport() {
		exporting = true;
		try {
			const res = await fetch(`${BACKEND_URL}/export`);
			if (!res.ok) throw new Error('Export failed');
			const blob = await res.blob();
			const now = new Date();
			const stamp = now.toISOString().slice(0, 19).replace(/[T:]/g, '-');
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = `obs-manager-backup-${stamp}.zip`;
			a.click();
			URL.revokeObjectURL(url);
			addToast('success', 'Database exported successfully.');
		} catch {
			addToast('error', 'Export failed.');
		} finally {
			exporting = false;
		}
	}

	function triggerImport() {
		fileInput.click();
	}

	async function onFileChange(e: Event) {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (!file) return;

		if (!file.name.endsWith('.zip')) {
			addToast('error', 'Please select a .zip backup file.');
			return;
		}

		const confirmed = confirm(
			'Warning: Importing will replace ALL current data. This cannot be undone. Continue?'
		);
		if (!confirmed) return;

		importing = true;
		try {
			const formData = new FormData();
			formData.append('file', file);
			const res = await fetch(`${BACKEND_URL}/import`, {
				method: 'POST',
				body: formData,
			});
			const data = await res.json();
			if (data.success) {
				addToast('success', 'Database imported. Reload to see changes.');
				canExport = true;
			} else {
				addToast('error', data.error ?? 'Import failed.');
			}
		} catch {
			addToast('error', 'Import failed.');
		} finally {
			importing = false;
			fileInput.value = '';
		}
	}
</script>

<div class="editor-wrap">
	{#if !IS_TAURI}
		<TopNav back={{ href: '/', label: 'Control' }} />
	{/if}
	<div class="editor-body">

		<!-- ── Sidebar ── -->
		<aside class="sidebar">
			<div class="sidebar-header">
				<span class="sidebar-title">Settings</span>
			</div>
			<nav class="sidebar-list">
				<button
					class="sidebar-item"
					class:selected={activeTab === 'import-export'}
					onclick={() => (activeTab = 'import-export')}
				>
					<div class="item-icon">
						<svg xmlns="http://www.w3.org/2000/svg" height="24px" viewBox="0 -960 960 960" width="24px" fill="currentColor"><path d="M480-140q-145.61 0-242.81-41.12Q140-222.23 140-283.85V-680q0-57.92 99.54-98.96Q339.08-820 480-820q140.92 0 240.46 41.04Q820-737.92 820-680v396.15q0 61.62-97.19 102.73Q625.61-140 480-140Zm0-461.69q87.46 0 176.12-24.73 88.65-24.73 102.73-53.35-13.7-29.38-101.66-54.81Q569.23-760 480-760q-89.08 0-176.58 24.73-87.5 24.73-103.04 53.96 15.16 30 102.27 54.81 87.12 24.81 177.35 24.81Zm0 200.15q41.62 0 81-4t75.27-11.69q35.88-7.69 67.19-19.08 31.31-11.38 56.54-25.77V-604q-25.23 14.38-56.54 25.77-31.31 11.38-67.19 19.07-35.89 7.7-75.27 11.7-39.38 4-81 4-42.38 0-82.58-4.2-40.19-4.19-75.88-11.88t-66.5-18.88Q224.23-589.62 200-604v141.92q24.23 14.39 55.04 25.58 30.81 11.19 66.5 18.88 35.69 7.7 75.88 11.89 40.2 4.19 82.58 4.19ZM480-200q48.69 0 95.62-6.42 46.92-6.43 85.38-17.54 38.46-11.12 64.88-25.81 26.43-14.69 34.12-30.85v-121.46q-25.23 14.39-56.54 25.77-31.31 11.39-67.19 19.08-35.89 7.69-75.27 11.69-39.38 4-81 4-42.38 0-82.58-4.19-40.19-4.19-75.88-11.89-35.69-7.69-66.5-18.88-30.81-11.19-55.04-25.58V-280q7.69 16.54 33.81 30.73 26.11 14.19 64.57 25.31 38.47 11.11 85.7 17.54Q431.31-200 480-200Z"/></svg>
					</div>
					<span class="item-label">Import &amp; Export</span>
				</button>

				<button
					class="sidebar-item"
					class:selected={activeTab === 'server'}
					onclick={() => (activeTab = 'server')}
				>
					<div class="item-icon">
						<svg xmlns="http://www.w3.org/2000/svg" height="24px" viewBox="0 -960 960 960" width="24px" fill="currentColor"><path d="M300.05-697.69q-20.82 0-35.43 14.57Q250-668.55 250-647.74q0 20.82 14.57 35.43 14.57 14.62 35.38 14.62 20.82 0 35.43-14.57Q350-626.83 350-647.65q0-20.81-14.57-35.43-14.57-14.61-35.38-14.61Zm0 375.38q-20.82 0-35.43 14.57Q250-293.17 250-272.35q0 20.81 14.57 35.43 14.57 14.61 35.38 14.61 20.82 0 35.43-14.57Q350-251.45 350-272.26q0-20.82-14.57-35.43-14.57-14.62-35.38-14.62ZM175.39-807.69h609.22q15.04 0 25.22 10.15Q820-787.4 820-772.4v247.01q0 16.24-10.17 26.97-10.18 10.73-25.22 10.73H175.39q-15.04 0-25.22-10.73Q140-509.15 140-525.39V-772.4q0-15 10.17-25.14 10.18-10.15 25.22-10.15Zm24.61 60v200h560v-200H200Zm-24.61 315.38h608.45q15.85 0 26 10.62Q820-411.08 820-395.38v244.61q0 17-10.16 27.73-10.15 10.73-26 10.73H176.16q-15.85 0-26-10.73Q140-133.77 140-150.77v-244.61q0-15.7 9.77-26.31 9.77-10.62 25.62-10.62Zm24.61 60v200h560v-200H200Zm0-375.38v200-200Zm0 375.38v200-200Z"/></svg>
					</div>
					<span class="item-label">Server</span>
				</button>

				<button
					class="sidebar-item"
					class:selected={activeTab === 'updates'}
					onclick={() => (activeTab = 'updates')}
				>
					<div class="item-icon">
						<svg xmlns="http://www.w3.org/2000/svg" height="24px" viewBox="0 -960 960 960" width="24px" fill="currentColor"><path d="m720-93.08 110.77-110.77-24.92-24.92-68.16 68.15v-156.3h-35.38v156.3l-68.16-68.15-24.92 24.92L720-93.08ZM473.85-796.54 233-657.23l247 142.77 247-142.77-240.85-139.31q-3.07-1.92-6.15-1.92-3.08 0-6.15 1.92ZM140-328.31v-303.38q0-19.69 9.49-36.07t26.67-26.39l267.69-154.08q9.23-5 17.75-7.42 8.52-2.43 18.38-2.43 9.87 0 18.9 2.43 9.04 2.42 17.27 7.42l267.69 154.08q17.18 10.01 26.67 26.39 9.49 16.38 9.49 36.07v148.61h-60v-124.46l-281 162-279-162v278.62q0 3.07 1.54 5.77 1.54 2.69 4.61 4.61l246.16 142.85v68.53L176.16-265.85q-17.18-10.01-26.67-26.39-9.49-16.38-9.49-36.07ZM592.54-77.54Q540-130.08 540-205q0-74.92 52.54-127.46Q645.08-385 720-385q74.92 0 127.46 52.54Q900-279.92 900-205q0 74.92-52.54 127.46Q794.92-25 720-25q-74.92 0-127.46-52.54ZM480-486.38Z"/></svg>					</div>
					<span class="item-label">Updates</span>
				</button>

				<button
					class="sidebar-item"
					class:selected={activeTab === 'about'}
					onclick={() => (activeTab = 'about')}
				>
					<div class="item-icon">
						<svg xmlns="http://www.w3.org/2000/svg" height="24px" viewBox="0 -960 960 960" width="24px" fill="currentColor"><path d="M450-290h60v-230h-60v230Zm52.92-307.75q9.39-9.29 9.39-23.02t-9.29-23.02q-9.29-9.28-23.02-9.28t-23.02 9.28q-9.29 9.29-9.29 23.02t9.39 23.02q9.38 9.29 22.92 9.29 13.54 0 22.92-9.29ZM480.07-100q-78.84 0-148.21-29.92t-120.68-81.21q-51.31-51.29-81.25-120.63Q100-401.1 100-479.93q0-78.84 29.92-148.21t81.21-120.68q51.29-51.31 120.63-81.25Q401.1-860 479.93-860q78.84 0 148.21 29.92t120.68 81.21q51.31 51.29 81.25 120.63Q860-558.9 860-480.07q0 78.84-29.92 148.21t-81.21 120.68q-51.29 51.31-120.63 81.25Q558.9-100 480.07-100Zm-.07-60q134 0 227-93t93-227q0-134-93-227t-227-93q-134 0-227 93t-93 227q0 134 93 227t227 93Zm0-320Z"/></svg>
					</div>
					<span class="item-label">About</span>
				</button>
			</nav>
		</aside>

		<!-- ── Main area ── -->
		<main class="editor-main">

			<!-- Import & Export tab -->
			{#if activeTab === 'import-export'}
				<div class="editor-panel">
					<div class="panel-header">
						<div class="panel-title-area">
							<h1>Import &amp; Export</h1>
						</div>
					</div>

					<div class="form-body">
						<p>
							Export your programs, screens, pop-ups, and studio settings as a ZIP archive.
							Or import a previous backup to fully restore your data.
						</p>

						<div class="action-row">
							<button
								class="action-btn"
								class:disabled={!canExport}
								onclick={doExport}
								disabled={!canExport || exporting}
							>
								<div class="action-icon export-icon">
									<svg xmlns="http://www.w3.org/2000/svg" height="24px" viewBox="0 -960 960 960" width="24px" fill="currentColor"><path d="M450-328.46v-336l-98.61 98.61-42.16-43.38L480-780l170.77 170.77-42.16 43.38L510-664.46v336h-60ZM252.31-180Q222-180 201-201q-21-21-21-51.31v-108.46h60v108.46q0 4.62 3.85 8.46 3.84 3.85 8.46 3.85h455.38q4.62 0 8.46-3.85 3.85-3.84 3.85-8.46v-108.46h60v108.46Q780-222 759-201q-21 21-51.31 21H252.31Z"/></svg>
								</div>
								<div class="action-text">
									<span class="action-title">{exporting ? 'Exporting…' : 'Export Database'}</span>
									<span class="action-sub">Download a .zip backup file</span>
								</div>
							</button>

							<button class="action-btn" onclick={triggerImport} disabled={importing}>
								<div class="action-icon import-icon">
									<svg xmlns="http://www.w3.org/2000/svg" height="24px" viewBox="0 -960 960 960" width="24px" fill="currentColor"><path d="M480-328.46 309.23-499.23l42.16-43.38L450-444v-336h60v336l98.61-98.61 42.16 43.38L480-328.46ZM252.31-180Q222-180 201-201q-21-21-21-51.31v-108.46h60v108.46q0 4.62 3.85 8.46 3.84 3.85 8.46 3.85h455.38q4.62 0 8.46-3.85 3.85-3.84 3.85-8.46v-108.46h60v108.46Q780-222 759-201q-21 21-51.31 21H252.31Z"/></svg>
								</div>
								<div class="action-text">
									<span class="action-title">{importing ? 'Importing…' : 'Import Database'}</span>
									<span class="action-sub">Restore from a .zip backup file</span>
								</div>
							</button>
						</div>

						{#if !canExport}
							<p class="warn-note">
								<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
									<circle cx="12" cy="12" r="10"/><path d="M12 8v4M12 16h.01"/>
								</svg>
								Export is disabled — no data in database yet.
							</p>
						{/if}

						<div class="danger-note">
							<svg xmlns="http://www.w3.org/2000/svg" height="24px" viewBox="0 -960 960 960" width="24px" fill="currentColor"><path d="M74.62-140 480-840l405.38 700H74.62ZM178-200h604L480-720 178-200Zm324.92-57.08q9.39-9.38 9.39-22.92 0-13.54-9.39-22.92-9.38-9.39-22.92-9.39-13.54 0-22.92 9.39-9.39 9.38-9.39 22.92 0 13.54 9.39 22.92 9.38 9.39 22.92 9.39 13.54 0 22.92-9.39ZM450-352.31h60v-200h-60v200ZM480-460Z"/></svg>
							Importing will permanently replace all existing data.
						</div>
					</div>
				</div>

			<!-- Server tab -->
			{:else if activeTab === 'server'}
				<div class="editor-panel">
					<div class="panel-header">
						<div class="panel-title-area">
							<h1>Server</h1>
						</div>
					</div>

					<div class="form-body">
						<p>
							Configure which port the backend server listens on. Leave blank to let the
							app automatically find an available port on each launch.
						</p>

						<!-- Current port status -->
						<div class="port-status-card">
							<div class="port-status-label">Currently running on port</div>
							<div class="port-status-value">{currentPort > 0 ? currentPort : '—'}</div>
						</div>

						<!-- Port input -->
						<div class="field-group">
							<label class="field-label" for="port-input">Preferred port</label>
							<p class="field-hint">
								Enter a port between 1024 and 65535, or leave empty for automatic selection
								(tries 5000, 5174, 3000, 8080, 8000 in order).
							</p>
							<div class="port-input-row">
								<input
									id="port-input"
									class="port-input"
									type="text"
									inputmode="numeric"
									pattern="[0-9]*"
									placeholder="Automatic"
									bind:value={preferredPortInput}
								/>
								{#if String(preferredPortInput).trim() !== ''}
									<button class="reset-btn" onclick={resetPort} title="Clear — use automatic">
										<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
											<line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
										</svg>
									</button>
								{/if}
							</div>
						</div>

						<!-- Save button -->
						<div class="save-row">
							<button
								class="save-btn"
								class:saved={portSaved}
								onclick={savePreferredPort}
								disabled={portSaving}
							>
								{#if portSaved}
									<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
										<polyline points="20 6 9 17 4 12"/>
									</svg>
									Saved
								{:else if portSaving}
									Saving…
								{:else}
									Save
								{/if}
							</button>
						</div>

						<!-- Restart notice -->
						<div class="info-note">
							<svg xmlns="http://www.w3.org/2000/svg" height="24px" viewBox="0 -960 960 960" width="24px" fill="currentColor"><path d="M450-290h60v-230h-60v230Zm52.92-307.75q9.39-9.29 9.39-23.02t-9.29-23.02q-9.29-9.28-23.02-9.28t-23.02 9.28q-9.29 9.29-9.29 23.02t9.39 23.02q9.38 9.29 22.92 9.29 13.54 0 22.92-9.29ZM480.07-100q-78.84 0-148.21-29.92t-120.68-81.21q-51.31-51.29-81.25-120.63Q100-401.1 100-479.93q0-78.84 29.92-148.21t81.21-120.68q51.29-51.31 120.63-81.25Q401.1-860 479.93-860q78.84 0 148.21 29.92t120.68 81.21q51.31 51.29 81.25 120.63Q860-558.9 860-480.07q0 78.84-29.92 148.21t-81.21 120.68q-51.29 51.31-120.63 81.25Q558.9-100 480.07-100Zm-.07-60q134 0 227-93t93-227q0-134-93-227t-227-93q-134 0-227 93t-93 227q0 134 93 227t227 93Zm0-320Z"/></svg>
							Port changes take effect after restarting the application.
						</div>
					</div>
				</div>

			<!-- Updates tab -->
			{:else if activeTab === 'updates'}
				<div class="editor-panel">
					<div class="panel-header">
						<div class="panel-title-area">
							<h1>Updates</h1>
						</div>
					</div>

					<div class="form-body">
						<p>Check for the latest version of Open GFX Board.</p>

						<!-- Current version card -->
						<div class="port-status-card">
							<div class="port-status-label">Current version</div>
							<div class="port-status-value">{appVersion ? `v${appVersion}` : '—'}</div>
						</div>

						<!-- Status feedback -->
						{#if updateStatus === 'up-to-date'}
							<div class="info-note update-ok-note">
								<svg xmlns="http://www.w3.org/2000/svg" height="20px" viewBox="0 -960 960 960" width="20px" fill="currentColor"><path d="M382-240 154-468l57-57 171 171 367-367 57 57-424 424Z"/></svg>
								You're on the latest version.
							</div>
						{:else if updateStatus === 'available' && updateVersion}
							<div class="update-available-card">
								<div class="update-card-icon">
									<svg xmlns="http://www.w3.org/2000/svg" height="22px" viewBox="0 -960 960 960" width="22px" fill="currentColor"><path d="M480-140q-148.77 0-252.5-98.5T122.77-480q0-145.15 98.15-248.08Q319.08-831 463-840l-50.31-50.31 42.16-42.15L600-786.92 454.85-641.77l-42.16-42.15L462-733.08q-118.08 9.93-199.15 95.97Q181.77-551.08 181.77-480q0 124.15 87.04 211.19Q355.85-181.77 480-181.77q124.15 0 211.19-87.04Q778.23-355.85 778.23-480q0-69.54-31.42-129.85-31.43-60.3-84.73-99.84l42.69-42.7q64.46 50.39 100.85 123.19Q842.01-556.4 842.01-480q0 148.77-98.62 252.38Q544.77-124 480-140Zm-30-278.46v-211.85h60v211.85l76.92 76.92-42.15 42.16L450-418.46Z"/></svg>
								</div>
								<div class="update-card-text">
									<span class="update-card-title">Update available</span>
									<span class="update-card-sub">Version <strong>{updateVersion}</strong> is ready to install. The app will restart automatically.</span>
								</div>
							</div>
						{/if}

						<!-- Error -->
						{#if updateError}
							<p class="warn-note">
								<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
									<circle cx="12" cy="12" r="10"/><path d="M12 8v4M12 16h.01"/>
								</svg>
								{updateError}
							</p>
						{/if}

						<!-- Action buttons -->
						{#if !IS_TAURI}
							<div class="info-note">Updates are only available in the desktop app.</div>
						{:else if updateStatus === 'available'}
							<div class="save-row">
								{#if updateInstalling}
									<div class="update-progress-inline">
										<div class="spinner-sm"></div>
										<span>Downloading and installing&hellip;</span>
									</div>
								{:else}
									<button class="save-btn update-install-btn" onclick={installUpdate}>
										Install Update
									</button>
								{/if}
							</div>
						{:else}
							<div class="save-row">
								<button
									class="save-btn"
									onclick={checkForUpdates}
									disabled={updateStatus === 'checking'}
								>
									{#if updateStatus === 'checking'}
										<div class="spinner-sm"></div>
										Checking&hellip;
									{:else}
										Check for Updates
									{/if}
								</button>
							</div>
						{/if}
					</div>
				</div>

			<!-- About tab -->
			{:else if activeTab === 'about'}
				<div class="editor-panel">
					<div class="panel-header">
						<div class="panel-title-area">
							<img src="/icon.png" alt="Open GFX Board" class="about-icon" />
							<div class="about-app-info">
								<h1>Open GFX Board</h1>
								{#if appVersion}
									<span class="about-version">v{appVersion}</span>
								{/if}
							</div>
						</div>
					</div>

					<div class="form-body">
						<p>
							A graphics and overlay management tool for live broadcasting. Manage programs, screens,
							pop-up overlays, and studio presets — all in one place.
						</p>
						<p class="about-credit">
							Developed by <a href="https://github.com/alayshoo" target="_blank" rel="noopener noreferrer" class="about-link">Alayshoo</a>.
						</p>
					</div>
				</div>
			{/if}

		</main>
	</div>
</div>

<input
	bind:this={fileInput}
	type="file"
	accept=".zip"
	onchange={onFileChange}
	style="display:none"
/>

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
		width: 220px;
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
		flex-shrink: 0;
	}

	.sidebar-list {
		flex: 1;
		overflow-y: auto;
		padding: 6px 0;
	}

	.sidebar-item {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 9px 14px;
		width: 100%;
		text-align: left;
		background: transparent;
		border: none;
		border-left: 3px solid transparent;
		cursor: pointer;
		transition: background 0.1s;
		font-family: inherit;
	}

	.sidebar-item:hover {
		background: var(--surface-2);
	}

	.sidebar-item.selected {
		background: var(--accent-dim);
		border-left-color: var(--accent);
	}

	.item-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-3);
		flex-shrink: 0;
	}

	.sidebar-item.selected .item-icon {
		color: var(--accent);
	}

	.sidebar-item.selected .item-label {
		color: var(--text-1);
	}

	/* ── Main editor area ── */
	.editor-main {
		flex: 1;
		overflow-y: auto;
		padding: 32px 40px;
		background: var(--bg);
	}

	.editor-panel {
		max-width: 600px;
	}

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
		align-items: center;
		gap: 24px;
	}

	.form-body {
		display: flex;
		flex-direction: column;
		gap: 24px;
	}

	/* ── Import/Export actions ── */
	.action-row {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.action-btn {
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 14px 18px;
		background: var(--surface-2);
		border: 1px solid var(--border-1);
		border-radius: var(--r-lg);
		cursor: pointer;
		font-family: inherit;
		text-align: left;
		transition: all 0.15s;
	}

	.action-btn:hover:not(:disabled) {
		border-color: var(--border-2);
		background: var(--surface-3);
	}

	.action-btn:disabled,
	.action-btn.disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.action-icon {
		width: 40px;
		height: 40px;
		border-radius: var(--r);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.export-icon {
		background: var(--accent-dim);
		color: var(--accent);
		border: 1px solid rgba(56, 189, 248, 0.2);
	}

	.import-icon {
		background: var(--go-dim);
		color: var(--go);
		border: 1px solid rgba(34, 197, 94, 0.2);
	}

	.action-text {
		display: flex;
		flex-direction: column;
		gap: 3px;
	}

	.action-title {
		font-size: 0.875rem;
		font-weight: 600;
		color: var(--text-1);
	}

	.action-sub {
		font-size: 0.75rem;
		color: var(--text-3);
	}

	.warn-note {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 0.8125rem;
		color: var(--warn);
		background: var(--warn-dim);
		border: 1px solid rgba(245, 158, 11, 0.2);
		border-radius: var(--r-sm);
		padding: 8px 12px;
		margin: 0;
	}

	.danger-note {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 0.75rem;
		color: var(--text-3);
		background: var(--live-dim);
		border: 1px solid rgba(239, 68, 68, 0.15);
		border-radius: var(--r-sm);
		padding: 8px 12px;
	}

	/* ── Server tab ── */
	.port-status-card {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 14px 18px;
		background: var(--surface-2);
		border: 1px solid var(--border-1);
		border-radius: var(--r-lg);
	}

	.port-status-label {
		font-size: 0.875rem;
		color: var(--text-3);
	}

	.port-status-value {
		font-size: 1.375rem;
		font-weight: 700;
		font-variant-numeric: tabular-nums;
		color: var(--accent);
		letter-spacing: 0.02em;
	}

	.field-group {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.port-input-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.port-input {
		width: 160px;
		padding: 8px 12px;
		background: var(--surface-2);
		border: 1px solid var(--border-1);
		border-radius: var(--r);
		color: var(--text-1);
		font-size: 0.875rem;
		font-variant-numeric: tabular-nums;
		transition: border-color 0.15s;
		appearance: textfield;
	}

	.port-input::-webkit-inner-spin-button,
	.port-input::-webkit-outer-spin-button {
		-webkit-appearance: none;
	}

	.port-input:focus {
		outline: none;
		border-color: var(--accent);
	}

	.port-input::placeholder {
		color: var(--text-3);
		font-style: italic;
	}

	.reset-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		background: transparent;
		border: 1px solid var(--border-1);
		border-radius: var(--r-sm);
		color: var(--text-3);
		cursor: pointer;
		transition: all 0.15s;
		flex-shrink: 0;
	}

	.reset-btn:hover {
		background: var(--surface-2);
		color: var(--text-1);
		border-color: var(--border-2);
	}

	.save-row {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.save-btn {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 8px 20px;
		background: var(--accent-dim);
		border: 1px solid rgba(56, 189, 248, 0.25);
		border-radius: var(--r);
		color: var(--accent);
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.15s;
	}

	.save-btn:hover:not(:disabled) {
		background: rgba(56, 189, 248, 0.2);
		border-color: rgba(56, 189, 248, 0.4);
	}

	.save-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.save-btn.saved {
		background: var(--go-dim);
		border-color: rgba(34, 197, 94, 0.25);
		color: var(--go);
	}

	.info-note {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 0.8125rem;
		color: var(--text-3);
		background: var(--surface-2);
		border: 1px solid var(--border-1);
		border-radius: var(--r-sm);
		padding: 8px 12px;
	}

	/* ── About tab ── */
	.about-icon {
		width: 96px;
		height: 96px;
		border-radius: var(--r-lg);
		flex-shrink: 0;
		filter: drop-shadow(0 0 4px rgba(248, 248, 248, 0.18));
	}

	.about-app-info {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.about-app-info h1 {
		font-size: 2rem;
	}

	.about-version {
		font-size: 0.875rem;
		color: var(--text-3);
	}

	.about-credit {
		font-size: 0.875rem;
		color: var(--text-3);
		margin: 0;
	}

	.about-link {
		color: var(--accent);
		text-decoration: none;
	}

	.about-link:hover {
		text-decoration: underline;
	}

	/* ── Updates tab ── */
	.update-ok-note {
		color: var(--go);
		background: var(--go-dim);
		border-color: rgba(34, 197, 94, 0.2);
	}

	.update-available-card {
		display: flex;
		align-items: flex-start;
		gap: 14px;
		padding: 14px 18px;
		background: var(--accent-dim);
		border: 1px solid rgba(56, 189, 248, 0.25);
		border-radius: var(--r-lg);
	}

	.update-card-icon {
		width: 40px;
		height: 40px;
		border-radius: var(--r);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		background: rgba(56, 189, 248, 0.15);
		color: var(--accent);
	}

	.update-card-text {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding-top: 2px;
	}

	.update-card-title {
		font-size: 0.875rem;
		font-weight: 600;
		color: var(--text-1);
	}

	.update-card-sub {
		font-size: 0.8125rem;
		color: var(--text-2);
		line-height: 1.5;
	}

	.update-card-sub strong {
		color: var(--text-1);
		font-weight: 600;
	}

	.update-install-btn {
		background: var(--accent-dim);
		border-color: rgba(56, 189, 248, 0.35);
		color: var(--accent);
	}

	.update-install-btn:hover:not(:disabled) {
		background: rgba(56, 189, 248, 0.2);
		border-color: rgba(56, 189, 248, 0.5);
	}

	.update-progress-inline {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 0.875rem;
		color: var(--text-2);
	}

	.spinner-sm {
		width: 14px;
		height: 14px;
		border: 2px solid var(--border-2);
		border-top-color: var(--accent);
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
		flex-shrink: 0;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}
</style>
