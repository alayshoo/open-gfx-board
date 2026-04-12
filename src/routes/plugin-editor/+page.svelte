<script lang="ts">
	import { onMount } from 'svelte';
	import TopNav from '$lib/components/TitleBarWeb.svelte';
	import PluginHost from '$lib/components/PluginHost.svelte';
	import { fetchPlugins, fetchPluginManifest } from '$lib/api/plugins';
	import { IS_TAURI } from '$lib/bridge';
	import type { PluginInfo, PluginManifest } from '$lib/types';

	let plugins = $state<PluginInfo[]>([]);
	let manifests = $state<Record<string, PluginManifest>>({});
	let activePluginId = $state<string | null>(null);
	let loading = $state(true);

	const editorPlugins = $derived(
		plugins.filter((p) => p.enabled && p.has_editor),
	);

	onMount(async () => {
		try {
			plugins = await fetchPlugins();
			const editors = plugins.filter((p) => p.enabled && p.has_editor);

			// Load manifests for all editor plugins
			for (const plugin of editors) {
				try {
					manifests[plugin.id] = await fetchPluginManifest(plugin.id);
				} catch {
					// skip broken plugins
				}
			}

			if (editors.length > 0) {
				activePluginId = editors[0].id;
			}
		} catch {
			// ignore
		} finally {
			loading = false;
		}
	});
</script>

<div class="editor-wrap">
	{#if !IS_TAURI}
		<TopNav back={{ href: '/settings', label: 'Settings' }} />
	{/if}

	<div class="editor-body">
		{#if loading}
			<div class="loading">Loading plugins…</div>
		{:else if editorPlugins.length === 0}
			<div class="empty">
				<p>No plugins with editor pages are installed.</p>
				<a href="/settings" class="btn btn-secondary">Back to Settings</a>
			</div>
		{:else}
			<!-- ── Sidebar ── -->
			<aside class="sidebar">
				<div class="sidebar-header">
					<span class="sidebar-title">
						Editor Plugins
						<span class="badge">{editorPlugins.length}</span>
					</span>
				</div>

				<div class="sidebar-list">
					{#each editorPlugins as plugin (plugin.id)}
						<button
							class="sidebar-item"
							class:selected={activePluginId === plugin.id}
							onclick={() => (activePluginId = plugin.id)}
						>
							<span class="item-name">{plugin.name}</span>
							{#if manifests[plugin.id]?.description}
								<span class="item-meta">{manifests[plugin.id].description}</span>
							{/if}
						</button>
					{/each}
				</div>
			</aside>

			<!-- ── Main editor area ── -->
			<main class="editor-main">
				{#if activePluginId && manifests[activePluginId]}
					{#key activePluginId}
						<PluginHost
							pluginId={activePluginId}
							componentType="editor"
							manifest={manifests[activePluginId]}
						/>
					{/key}
				{:else if activePluginId}
					<div class="empty-state">
						<p>Loading plugin editor…</p>
					</div>
				{/if}
			</main>
		{/if}
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

	/* ── Full-page states ── */
	.loading,
	.empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 16px;
		width: 100%;
		color: var(--text-3);
		font-size: 0.9375rem;
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
		font-size: 0.8125rem;
		font-weight: 600;
		color: var(--text-2);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.badge {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: 18px;
		height: 18px;
		padding: 0 5px;
		background: var(--surface-3);
		border-radius: 9px;
		font-size: 0.6875rem;
		font-weight: 600;
		color: var(--text-2);
		letter-spacing: 0;
		text-transform: none;
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
		font-size: 0.875rem;
		font-weight: 500;
		color: var(--text-1);
	}

	.item-meta {
		font-size: 0.75rem;
		color: var(--text-3);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	/* ── Main editor area ── */
	.editor-main {
		flex: 1;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		background: var(--bg);
		padding: 32px 40px;
	}

	.empty-state {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: var(--text-3);
		font-size: 0.9375rem;
	}
</style>
