import { getBaseUrl } from './api';
import type { PluginInfo, PluginManifest } from '../types';

export async function fetchPlugins(): Promise<PluginInfo[]> {
	const res = await fetch(`${getBaseUrl()}/plugins`);
	return res.json();
}

export async function installPlugin(path: string): Promise<{ success: boolean; pluginId?: string; error?: string }> {
	const res = await fetch(`${getBaseUrl()}/plugins/install`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ path }),
	});
	return res.json();
}

export async function uninstallPlugin(id: string): Promise<{ success: boolean; error?: string }> {
	const res = await fetch(`${getBaseUrl()}/plugins/${id}`, { method: 'DELETE' });
	return res.json();
}

export async function enablePlugin(id: string): Promise<void> {
	await fetch(`${getBaseUrl()}/plugins/${id}/enable`, { method: 'PUT' });
}

export async function disablePlugin(id: string): Promise<void> {
	await fetch(`${getBaseUrl()}/plugins/${id}/disable`, { method: 'PUT' });
}

export async function refreshPlugin(id: string): Promise<{ success: boolean; error?: string }> {
	const res = await fetch(`${getBaseUrl()}/plugins/${id}/refresh`, { method: 'PUT' });
	return res.json();
}

export async function fetchPluginManifest(id: string): Promise<PluginManifest> {
	const res = await fetch(`${getBaseUrl()}/plugins/${id}/manifest`);
	return res.json();
}

export async function fetchPluginState(id: string): Promise<Record<string, any>> {
	const res = await fetch(`${getBaseUrl()}/plugins/${id}/state`);
	return res.json();
}

export async function updatePluginState(id: string, updates: Record<string, any>): Promise<Record<string, any>> {
	const res = await fetch(`${getBaseUrl()}/plugins/${id}/state`, {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(updates),
	});
	return res.json();
}

export async function fetchPluginData(id: string, table: string): Promise<any[]> {
	const res = await fetch(`${getBaseUrl()}/plugins/${id}/data/${table}`);
	return res.json();
}

export async function fetchPluginRow(id: string, table: string, rowId: number): Promise<any> {
	const res = await fetch(`${getBaseUrl()}/plugins/${id}/data/${table}/${rowId}`);
	return res.json();
}

export async function insertPluginData(id: string, table: string, data: any): Promise<any> {
	const res = await fetch(`${getBaseUrl()}/plugins/${id}/data/${table}`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(data),
	});
	return res.json();
}

export async function updatePluginData(id: string, table: string, rowId: number, data: any): Promise<any> {
	const res = await fetch(`${getBaseUrl()}/plugins/${id}/data/${table}/${rowId}`, {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(data),
	});
	return res.json();
}

export async function deletePluginData(id: string, table: string, rowId: number): Promise<void> {
	await fetch(`${getBaseUrl()}/plugins/${id}/data/${table}/${rowId}`, { method: 'DELETE' });
}

export async function queryPluginData(id: string, table: string, filters: Record<string, any>): Promise<any[]> {
	const params = new URLSearchParams();
	for (const [k, v] of Object.entries(filters)) {
		params.set(k, String(v));
	}
	const res = await fetch(`${getBaseUrl()}/plugins/${id}/data/${table}/query?${params}`);
	return res.json();
}

export async function triggerPluginPopup(
	id: string,
	templateId: string,
	context?: Record<string, string>,
	duration?: number,
): Promise<void> {
	await fetch(`${getBaseUrl()}/plugins/${id}/trigger-popup`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ template_id: templateId, context: context ?? {}, duration }),
	});
}

export function pluginAssetUrl(id: string, path: string): string {
	return `${getBaseUrl()}/plugins/${id}/assets/${path}`;
}
