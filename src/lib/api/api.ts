import { getBackendUrl } from '$lib/bridge';
import type { Program, Advertisement, Studio, Screen } from '../types';

export function getBaseUrl() {
	return getBackendUrl();
}

export const BACKEND_URL = getBackendUrl();

export async function fetchPrograms(): Promise<Program[]> {
	const res = await fetch(`${getBaseUrl()}/programs`);
	const data = await res.json();
	return data.programs ?? [];
}

export async function fetchStudios(): Promise<Studio[]> {
	const res = await fetch(`${getBaseUrl()}/studios`);
	const data = await res.json();
	return data.studios ?? [];
}

export async function fetchAdvertisements(): Promise<Advertisement[]> {
	const res = await fetch(`${getBaseUrl()}/advertisements`);
	const data = await res.json();
	return data.ads ?? [];
}

export async function fetchScreens(): Promise<Screen[]> {
	const res = await fetch(`${getBaseUrl()}/screens`);
	const data = await res.json();
	return data.screens ?? [];
}

export async function hasData(): Promise<boolean> {
	const res = await fetch(`${getBaseUrl()}/has-data`);
	const data = await res.json();
	return data.has_data ?? false;
}

export async function uploadImage(
	endpoint: string,
	file: File,
	id: number
): Promise<{ success: boolean; imagePath: string }> {
	const formData = new FormData();
	formData.append('image', file);
	formData.append('id', String(id));
	const res = await fetch(`${getBaseUrl()}${endpoint}`, {
		method: 'POST',
		body: formData,
	});
	return res.json();
}

export async function uploadProgramImage(
	file: File,
	programId: number,
	uploadType: 'logo' | 'background'
): Promise<{ success: boolean; imagePath: string }> {
	const formData = new FormData();
	formData.append('image', file);
	formData.append('upload_type', uploadType);
	const res = await fetch(`${getBaseUrl()}/programs/${programId}/upload-image`, {
		method: 'POST',
		body: formData,
	});
	return res.json();
}

export function imgUrl(path: string | null | undefined): string | null {
	if (!path) return null;
	return `${getBaseUrl()}/${path}`;
}
