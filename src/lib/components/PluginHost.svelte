<script lang="ts">
	import { onMount } from 'svelte';
	import { socket } from '$lib/api/socket';
	import { pluginAssetUrl } from '$lib/api/plugins';
	import { PluginSDK } from '$lib/plugins/sdk';
	import type { PluginManifest } from '$lib/types';

	interface Props {
		pluginId: string;
		/** Which component to load: 'control' or 'editor' */
		componentType: 'control' | 'editor';
		manifest: PluginManifest;
	}

	let { pluginId, componentType, manifest }: Props = $props();
	let hostEl: HTMLDivElement;
	let sdk: PluginSDK | null = null;
	let error: string | null = $state(null);

	onMount(() => {
		let destroyed = false;
		let element: HTMLElement | null = null;

		async function loadComponent() {
			const compDef = componentType === 'control' ? manifest.control : manifest.editor;
			if (!compDef) {
				error = `Plugin "${pluginId}" does not define a ${componentType} component`;
				return;
			}

			const jsUrl = pluginAssetUrl(pluginId, compDef.component);

			try {
				const module = await import(/* @vite-ignore */ jsUrl);
				if (destroyed) return;

				// The module should register a custom element.
				// Convention: the tag name is `{pluginId}-{componentType}` (e.g. football-control)
				const tagName = `${pluginId}-${componentType}`;

				// Wait a tick for customElements.define to take effect
				await new Promise((r) => setTimeout(r, 0));

				if (!customElements.get(tagName)) {
					// Try to auto-register if the module exports a default class
					if (module.default && typeof module.default === 'function') {
						try {
							customElements.define(tagName, module.default);
						} catch {
							// Already defined or invalid
						}
					}
				}

				if (!customElements.get(tagName)) {
					error = `Plugin "${pluginId}" did not register custom element <${tagName}>`;
					return;
				}

				sdk = new PluginSDK(pluginId, socket);
				element = document.createElement(tagName);
				(element as any).sdk = sdk;
				(element as any).manifest = manifest;

				if (destroyed) {
					sdk.destroy();
					return;
				}

				hostEl.appendChild(element);
			} catch (e: any) {
				if (!destroyed) {
					error = `Failed to load plugin component: ${e.message}`;
				}
			}
		}

		loadComponent();

		return () => {
			destroyed = true;
			if (sdk) {
				sdk.destroy();
				sdk = null;
			}
			if (element && hostEl?.contains(element)) {
				hostEl.removeChild(element);
			}
		};
	});
</script>

{#if error}
	<div class="plugin-error">
		<p>{error}</p>
	</div>
{/if}
<div class="plugin-host" bind:this={hostEl}></div>

<style>
	.plugin-host {
		width: 100%;
		min-height: 0;
	}

	.plugin-error {
		color: var(--warn, #f97316);
		font-size: 0.85rem;
		padding: 8px 12px;
		background: rgba(249, 115, 22, 0.1);
		border-radius: var(--r, 8px);
		border: 1px solid rgba(249, 115, 22, 0.25);
	}
</style>
