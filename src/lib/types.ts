export interface Screen {
	id: number;
	graphics_name: string;
	graphics_path: string | null;
	media_type: string;
	allow_popups: boolean;
	comments: string;
	programs: { id: number; name: string }[];
	created_at: string;
}

// Graphic is an alias for Screen for backward compat
export type Graphic = Screen;

export interface ProgramPopUp {
	id: number;
	popup_id: number;
	program_id: number;
	popup_launch_type: 'automatic' | 'manual' | 'both' | 'filler';
	duration: number;
	frequency: number;
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
	direction: 'top' | 'bottom' | 'left' | 'right';
	position: number;
	programs: { id: number; name: string }[];
	created_at: string;
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
	graphicId: number;
	graphicPath: string | null;
	allowPopUps: boolean;
}

export interface ActivePopUp {
	popupId: number;
	imagePath: string | null;
	duration: number;
	direction: string;
	position: number;
}

export interface StudioState {
	studioId: number;
	programId: number | null;
	program: Program | null;
	activeOverlay: ActiveOverlay | null;
	activePopUp: ActivePopUp | null;
}

export interface Toast {
	id: number;
	type: 'success' | 'error' | 'info';
	message: string;
}
