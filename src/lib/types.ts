export interface Screen {
	id: number;
	graphics_name: string;
	graphics_path: string | null;
	/** Vertical-alt media path (portrait/9:16 sources). Null = no vertical alt. */
	graphics_path_vertical: string | null;
	media_type: string;
	allow_popups: boolean;
	/** Raw HTML template (only when media_type is "html"). */
	html_content: string | null;
	comments: string;
	programs: { id: number; name: string }[];
	created_at: string;
	/** Set when this screen was installed by a plugin; null for user-created screens. */
	plugin_id: string | null;
	plugin_template_id: string | null;
	/**
	 * Layer assignment within a program (1 = top, 2 = middle, 3 = bottom).
	 * Only present when the screen is part of a Program's `graphics` array;
	 * omitted for standalone screens returned by /screens.
	 */
	layer?: number;
}

// Graphic is an alias for Screen for backward compat
export type Graphic = Screen;

export interface ProgramPopUp {
	id: number;
	popup_id: number;
	program_id: number;
	popup_launch_type: 'automatic' | 'manual' | 'both' | 'filler' | 'hidden';
	duration: number;
	frequency: number;
	/** Layer assignment within the program (1 = top, 2 = middle, 3 = bottom). */
	layer: number;
	popup: PopUp;
}

export interface Program {
	id: number;
	name: string;
	logo_path: string | null;
	background_graphics_path: string | null;
	graphics: Screen[];
	program_popups: ProgramPopUp[];
	created_at: string;
}

export interface PopUp {
	id: number;
	name: string;
	sponsor_name: string;
	comments: string;
	image_path: string | null;
	media_type: string;
	/** Raw HTML template (only when media_type is "html"). */
	html_content: string | null;
	direction: 'top' | 'bottom' | 'left' | 'right';
	position: number;
	/** Explicit popup width in pixels. Null = use natural media size / client default. */
	width: number | null;
	/** Explicit popup height in pixels. Null = use natural media size / client default. */
	height: number | null;
	/** Vertical-alt slide-in direction override. Null = use default direction. */
	direction_vertical: 'top' | 'bottom' | 'left' | 'right' | null;
	/** Vertical-alt position override (0–100 %). Null = use default position. */
	position_vertical: number | null;
	programs: { id: number; name: string }[];
	created_at: string;
	/** Set when this pop-up was installed by a plugin; null for user-created pop-ups. */
	plugin_id: string | null;
	plugin_template_id: string | null;
}

export interface ObsCommand {
	id: number | null;
	studio_id?: number;
	preset_id?: number;
	obs_command_name: string;
	obs_command_color: string;
	obs_command_description: string;
	obs_command_shortcut: string;
}

export interface Preset {
	id: number | null;
	studio_id?: number;
	name: string;
	commands: ObsCommand[];
}

export interface Studio {
	id: number;
	name: string;
	obs_browser_source_address: string;
	presets: Preset[];
	commands: ObsCommand[];
	created_at: string;
}

export interface ActiveOverlay {
	/** Layer number (1 = top, 2 = middle, 3 = bottom). */
	layer: number;
	graphicId: number;
	graphicPath: string | null;
	/** Vertical-alt media path (portrait/9:16 sources). Null = no vertical alt. */
	graphicPathVertical: string | null;
	allowPopUps: boolean;
	mediaType: string;
	/** Processed HTML (template variables already resolved). Only present when mediaType is "html". */
	htmlContent: string | null;
}

export interface ActivePopUp {
	/** Layer number (1 = top, 2 = middle, 3 = bottom). */
	layer: number;
	popupId: number;
	imagePath: string | null;
	duration: number;
	direction: string;
	position: number;
	/** Vertical-alt direction override. Null = use default direction. */
	directionVertical: string | null;
	/** Vertical-alt position override (0–100 %). Null = use default position. */
	positionVertical: number | null;
	mediaType: string;
	/** Processed HTML (template variables already resolved). Only present when mediaType is "html". */
	htmlContent: string | null;
	/** Explicit popup width in pixels. Null = use natural media size / client default. */
	width: number | null;
	/** Explicit popup height in pixels. */
	height: number | null;
}

export interface StudioState {
	studioId: number;
	programId: number | null;
	program: Program | null;
	/** All currently active screen overlays, one per layer at most. */
	activeOverlays: ActiveOverlay[];
	/** All currently active pop-ups, one per layer at most. */
	activePopUps: ActivePopUp[];
}

export interface Toast {
	id: number;
	type: 'success' | 'error' | 'info';
	message: string;
}

// ── Plugin types ─────────────────────────────────────────────────────────────

export interface PluginInfo {
	id: string;
	name: string;
	version: string;
	description: string;
	author: string;
	enabled: boolean;
	has_control: boolean;
	has_editor: boolean;
	is_bundled: boolean;
}

export interface PluginManifest {
	id: string;
	name: string;
	version: string;
	description: string;
	author: string;
	database: Record<string, { columns: Record<string, string> }>;
	state: Record<string, { type: string; default: any }>;
	events: string[];
	control?: { component: string };
	editor?: { component: string };
	screens: PluginScreenDef[];
	popups: PluginPopupDef[];
}

export interface PluginScreenDef {
	template_id: string;
	name: string;
	template: string;
	allow_popups: boolean;
}

export interface PluginPopupDef {
	template_id: string;
	name: string;
	template: string;
	direction: string;
	position: number;
	width: number | null;
	height: number | null;
	duration: number;
}

// ── Layer helpers ─────────────────────────────────────────────────────────────

export type LayerNumber = 1 | 2 | 3;
export const LAYERS: LayerNumber[] = [1, 2, 3];

/** Per-layer colour palette for screens (light blue → darker blue → purple). */
export const SCREEN_LAYER_COLORS: Record<LayerNumber, { solid: string; dim: string }> = {
	1: { solid: '#38bdf8', dim: 'rgba(56,189,248,0.15)' },
	2: { solid: '#3b82f6', dim: 'rgba(59,130,246,0.15)' },
	3: { solid: '#a855f7', dim: 'rgba(168,85,247,0.15)' },
};

/** Per-layer colour palette for popups (amber → lime → teal). */
export const POPUP_LAYER_COLORS: Record<LayerNumber, { solid: string; dim: string }> = {
	1: { solid: '#f59e0b', dim: 'rgba(245,158,11,0.15)' },
	2: { solid: '#84cc16', dim: 'rgba(132,204,22,0.15)' },
	3: { solid: '#14b8a6', dim: 'rgba(20, 184, 102, 0.15)' },
};
