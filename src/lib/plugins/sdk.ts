import type { Socket } from 'socket.io-client';
import {
	fetchPluginState,
	updatePluginState,
	fetchPluginData,
	fetchPluginRow,
	insertPluginData,
	updatePluginData,
	deletePluginData,
	queryPluginData,
	triggerPluginPopup,
	pluginAssetUrl,
} from '$lib/api/plugins';

/**
 * Plugin SDK – exposed to plugin web components via the `sdk` property.
 * Provides state management, database CRUD, event handling, popup triggers, and asset URLs.
 */
export class PluginSDK {
	private pluginId: string;
	private socket: Socket;
	private listeners: Array<{ event: string; handler: (...args: any[]) => void }> = [];

	constructor(pluginId: string, socket: Socket) {
		this.pluginId = pluginId;
		this.socket = socket;
	}

	// ── State ────────────────────────────────────────────────────────────

	async getState(): Promise<Record<string, any>> {
		return fetchPluginState(this.pluginId);
	}

	async setState(updates: Record<string, any>): Promise<Record<string, any>> {
		return updatePluginState(this.pluginId, updates);
	}

	/**
	 * Subscribe to state changes.  The callback receives the full state snapshot.
	 * Returns an unsubscribe function.
	 */
	onStateChanged(callback: (state: Record<string, any>) => void): () => void {
		const event = `plugin-state-updated:${this.pluginId}`;
		const handler = (data: any) => callback(data.state ?? data);
		this.socket.on(event, handler);
		this.listeners.push({ event, handler });
		return () => {
			this.socket.off(event, handler);
			this.listeners = this.listeners.filter((l) => l.handler !== handler);
		};
	}

	// ── Database ─────────────────────────────────────────────────────────

	async getData(table: string): Promise<any[]> {
		return fetchPluginData(this.pluginId, table);
	}

	async getRow(table: string, id: number): Promise<any> {
		return fetchPluginRow(this.pluginId, table, id);
	}

	async insertRow(table: string, data: any): Promise<any> {
		return insertPluginData(this.pluginId, table, data);
	}

	async updateRow(table: string, id: number, data: any): Promise<any> {
		return updatePluginData(this.pluginId, table, id, data);
	}

	async deleteRow(table: string, id: number): Promise<void> {
		return deletePluginData(this.pluginId, table, id);
	}

	async queryData(table: string, filters: Record<string, any>): Promise<any[]> {
		return queryPluginData(this.pluginId, table, filters);
	}

	/**
	 * Subscribe to data changes on a specific table.
	 * Returns an unsubscribe function.
	 */
	onDataChanged(
		table: string,
		callback: (event: { action: string; rowId: number }) => void,
	): () => void {
		const event = `plugin-data-changed:${this.pluginId}:${table}`;
		const handler = (data: any) => callback(data);
		this.socket.on(event, handler);
		this.listeners.push({ event, handler });
		return () => {
			this.socket.off(event, handler);
			this.listeners = this.listeners.filter((l) => l.handler !== handler);
		};
	}

	// ── Events ───────────────────────────────────────────────────────────

	fireEvent(eventName: string, data?: any): void {
		this.socket.emit('plugin-fire-event', {
			pluginId: this.pluginId,
			event: eventName,
			data: data ?? null,
		});
	}

	onEvent(eventName: string, callback: (data: any) => void): () => void {
		const event = `plugin-event:${this.pluginId}:${eventName}`;
		const handler = (data: any) => callback(data.data ?? data);
		this.socket.on(event, handler);
		this.listeners.push({ event, handler });
		return () => {
			this.socket.off(event, handler);
			this.listeners = this.listeners.filter((l) => l.handler !== handler);
		};
	}

	// ── Popups ───────────────────────────────────────────────────────────

	async triggerPopup(
		templateId: string,
		context?: Record<string, string>,
		duration?: number,
	): Promise<void> {
		return triggerPluginPopup(this.pluginId, templateId, context, duration);
	}

	// ── Assets ───────────────────────────────────────────────────────────

	assetUrl(path: string): string {
		return pluginAssetUrl(this.pluginId, path);
	}

	// ── Cleanup ──────────────────────────────────────────────────────────

	destroy(): void {
		for (const { event, handler } of this.listeners) {
			this.socket.off(event, handler);
		}
		this.listeners = [];
	}
}
