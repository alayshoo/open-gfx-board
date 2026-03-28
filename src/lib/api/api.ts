import { BACKEND_URL } from './socket';
import type { Program, Advertisement, Studio } from '../types';

export async function fetchPrograms(): Promise<Program[]> {
	const res = await fetch(`${BACKEND_URL}/programs`);
	const data = await res.json();
	return data.programs ?? [];
}

export async function fetchStudios(): Promise<Studio[]> {
	const res = await fetch(`${BACKEND_URL}/studios`);
	const data = await res.json();
	return data.studios ?? [];
}

export async function fetchAdvertisements(): Promise<Advertisement[]> {
	const res = await fetch(`${BACKEND_URL}/advertisements`);
	const data = await res.json();
	return data.ads ?? [];
}

export async function hasData(): Promise<boolean> {
	const res = await fetch(`${BACKEND_URL}/has-data`);
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
	const res = await fetch(`${BACKEND_URL}${endpoint}`, {
		method: 'POST',
		body: formData,
	});
	return res.json();
}

export async function uploadProgramImage(
	file: File,
	programId: number,
	uploadType: 'logo' | 'background' | 'graphic',
	graphicId?: number
): Promise<{ success: boolean; imagePath: string }> {
	const formData = new FormData();
	formData.append('image', file);
	formData.append('program_id', String(programId));
	formData.append('upload_type', uploadType);
	if (uploadType === 'graphic' && graphicId != null) {
		formData.append('graphic_id', String(graphicId));
	}
	const res = await fetch(`${BACKEND_URL}/programs/upload-image`, {
		method: 'POST',
		body: formData,
	});
	return res.json();
}

export function imgUrl(path: string | null | undefined): string | null {
	if (!path) return null;
	return `${BACKEND_URL}/${path}`;
}
