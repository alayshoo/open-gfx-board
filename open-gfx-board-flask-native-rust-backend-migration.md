# open-gfx-board: Flask → Native Rust Backend Migration

## Context

The project had a working UI built on SvelteKit 5 targeting a Flask backend on `localhost:PORT`. The architecture is being migrated to be fully self-contained in a Tauri 2 desktop app, with all backend logic implemented in Rust. The Tauri shell is a bare skeleton. This plan implements the complete Rust backend and updates the frontend to talk to it.

The design principle: an **Axum HTTP server runs inside the Tauri process** on an available port, serving the same REST + Socket.IO API the frontend already expects. This means:

- Minimal frontend changes (the existing fetch/socket calls still work)
- Remote browser clients (tablets) connect to the Axum server over the network
- The Tauri desktop app also connects to `localhost:5000`, no dual transport needed
- `bridge.ts` only needs to handle file uploads (native file picker in Tauri vs multipart in browser)
- `enigo` key presses run on the Rust server side, triggered by Socket.IO events — so remote clients CAN fire OBS shortcuts

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│  Tauri Process                                      │
│                                                     │
│  ┌─────────────────┐         ┌──────────────────┐   │
│  │  Webview        │         │  Axum Server     │   │
│  │  (SvelteKit SPA)│◄─HTTP──►│  :5000           │   │
│  │                 │◄─WS────►│  REST + Socket.IO│   │
│  └─────────────────┘         └────────┬─────────┘   │
│                                       │             │
│  Remote Browser ──────HTTP/WS────────►│             │
│  (tablet, OBS browser source)         │             │
│                                       │             │
│                               ┌───────▼──────────┐  │
│                               │  SQLite DB       │  │
│                               │  + media files   │  │
│                               └──────────────────┘  │
└─────────────────────────────────────────────────────┘
```

---

## Data Model (confirmed by user)

```
Studio
  id, name, obs_browser_source_address, created_at
  └── Presets[]        ← NEW layer (was: commands directly on studio)
        id, studio_id, name
        └── ObsCommands[]
              id, preset_id, name, color, shortcut (F13–F24), description

Program
  id, name, logo_path, bg_path, created_at
  └── Screens[]        ← DB name "screens", JSON uses existing "graphics" field names via serde rename
        id, program_id, name, comments, media_path, media_type, allow_ads
  └── ProgramAds[]
        id, program_id, ad_id, trigger_type (manual|automatic|both|filler), duration, frequency

Advertisement
  id, name, sponsor_name, comments, media_path, media_type, created_at
```

---

## Phase 1: Cargo.toml Dependencies

Add to `src-tauri/Cargo.toml`:

```toml
axum          = { version = "0.7", features = ["multipart", "macros"] }
tower-http    = { version = "0.5", features = ["fs", "cors"] }
socketioxide  = { version = "0.14", features = ["state"] }   # Socket.IO for axum 0.7
rusqlite      = { version = "0.31", features = ["bundled"] }  # bundled = no system sqlite needed
tokio         = { version = "1", features = ["full"] }
enigo         = "0.2"
dirs          = "5"
uuid          = { version = "1", features = ["v4"] }
anyhow        = "1"
chrono        = { version = "0.4", features = ["serde"] }
zip           = "2"
```

⚠️ `socketioxide 0.14` targets Axum 0.7 — keep versions pinned together.
⚠️ Port 5000 conflicts with macOS AirPlay on Monterey+. Use 5174 as fallback or make it configurable.

---

## Phase 2: Rust Module Structure

```
src-tauri/src/
├── main.rs                     (unchanged)
├── lib.rs                      (REWRITE: wires everything, starts Axum)
├── state.rs                    (AppState, StudioRuntimeState)
├── db/
│   ├── mod.rs
│   ├── schema.rs               (run_migrations — CREATE TABLE IF NOT EXISTS)
│   ├── studios.rs              (DB fns: studios, presets, commands)
│   ├── programs.rs             (DB fns: programs, screens, program_ads)
│   └── advertisements.rs      (DB fns: advertisements)
├── models/
│   └── mod.rs                  (all serde structs; use #[serde(rename)] to keep frontend field names)
├── commands/
│   └── mod.rs                  (all #[tauri::command] fns — used for Tauri IPC if needed)
├── server/
│   ├── mod.rs                  (build_router())
│   ├── routes/
│   │   ├── studios.rs
│   │   ├── programs.rs
│   │   ├── advertisements.rs
│   │   └── system.rs           (has-data, export, import, media serving)
│   └── socket_handlers.rs      (socketioxide event handlers)
└── keyboard.rs                 (enigo wrapper — fire_shortcut(key: &str))
```

### `state.rs`

```rust
pub struct AppState {
    pub db: Arc<Mutex<rusqlite::Connection>>,
    pub studio_states: Arc<Mutex<HashMap<i64, StudioRuntimeState>>>,
    pub app_data_dir: PathBuf,
    pub io: Arc<SocketIo>,      // set via OnceLock after socket layer is built
}
```

All DB calls from async handlers use `tokio::task::spawn_blocking`.

---

## Phase 3: Database Schema (`db/schema.rs`)

```sql
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS studios (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    obs_browser_source_address TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS presets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    studio_id INTEGER NOT NULL REFERENCES studios(id) ON DELETE CASCADE,
    name TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS obs_commands (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    preset_id INTEGER NOT NULL REFERENCES presets(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    color TEXT NOT NULL DEFAULT '#38bdf8',
    shortcut TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT ''
);
CREATE TABLE IF NOT EXISTS programs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    logo_path TEXT,
    bg_path TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS screens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    program_id INTEGER NOT NULL REFERENCES programs(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    comments TEXT NOT NULL DEFAULT '',
    media_path TEXT,
    media_type TEXT NOT NULL DEFAULT 'image',
    allow_ads INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE IF NOT EXISTS advertisements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    sponsor_name TEXT NOT NULL DEFAULT '',
    comments TEXT NOT NULL DEFAULT '',
    media_path TEXT,
    media_type TEXT NOT NULL DEFAULT 'image',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS program_ads (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    program_id INTEGER NOT NULL REFERENCES programs(id) ON DELETE CASCADE,
    ad_id INTEGER NOT NULL REFERENCES advertisements(id) ON DELETE CASCADE,
    trigger_type TEXT NOT NULL DEFAULT 'manual'
        CHECK(trigger_type IN ('manual','automatic','both','filler')),
    duration INTEGER NOT NULL DEFAULT 10,
    frequency INTEGER NOT NULL DEFAULT 1,
    UNIQUE(program_id, ad_id)
);
CREATE TABLE IF NOT EXISTS studio_state (
    studio_id        INTEGER PRIMARY KEY REFERENCES studios(id) ON DELETE CASCADE,
    program_id       INTEGER REFERENCES programs(id) ON DELETE SET NULL,
    active_screen_id INTEGER REFERENCES screens(id) ON DELETE SET NULL,
    active_ad_id     INTEGER REFERENCES advertisements(id) ON DELETE SET NULL
);
```

---

## Phase 4: Models (`models/mod.rs`)

Use `#[serde(rename)]` to keep existing frontend field names without changing any Svelte files:

```rust
// Screen → serializes as "Graphic" shape the frontend expects
pub struct Screen {
    pub id: i64,
    pub program_id: i64,
    #[serde(rename = "graphics_name")]   pub name: String,
    pub comments: String,
    #[serde(rename = "graphics_path")]   pub media_path: Option<String>,
    pub media_type: String,
    pub allow_ads: bool,
}

// ProgramAd: serialize trigger_type as "ad_launch_type" for frontend compat
pub struct ProgramAd {
    pub id: i64,
    pub program_id: i64,
    pub ad_id: i64,
    #[serde(rename = "ad_launch_type")]  pub trigger_type: String,
    pub duration: i64,
    pub frequency: i64,
    pub ad: Option<Advertisement>,
}

// Studio: expose both presets AND flattened commands (from all presets)
pub struct Studio {
    pub id: i64,
    pub name: String,
    pub obs_browser_source_address: String,
    pub presets: Vec<Preset>,
    pub commands: Vec<ObsCommand>,   // flattened from all presets, for compat
    pub created_at: String,
}
```

---

## Phase 5: Axum Server (`server/`)

### `server/mod.rs` — `build_router()`

```rust
let (socket_layer, io) = SocketIo::builder()
    .with_state(app_state.clone())
    .build_layer();

let cors = CorsLayer::new()
    .allow_origin(["tauri://localhost", "http://localhost:1420", "http://localhost:5174"])
    .allow_methods(Any).allow_headers(Any);

let router = Router::new()
    .nest("/studios", studios_router())
    .nest("/programs", programs_router())
    .nest("/advertisements", advertisements_router())
    .route("/has-data", get(has_data_handler))
    .route("/export", get(export_handler))
    .route("/import", post(import_handler))
    .route("/media/*path", get(serve_media))
    .layer(socket_layer)
    .layer(cors)
    .with_state(app_state)
    .fallback_service(
        ServeDir::new(&build_dir)
            .not_found_service(ServeFile::new(build_dir.join("index.html")))
    );
```

### REST Endpoints

**Studios**

- `GET /studios` → `{ studios: Vec<Studio> }`
- `POST /studios` body `{ name }` → `{ success, studio }`
- `PUT /studios/:id` body `{ name, presets: [{ name, commands: [...] }] }` → `{ success, studio }`
- `DELETE /studios/:id` → `{ success, id }`

**Programs**

- `GET /programs` → `{ programs: Vec<Program> }`
- `POST /programs` → `{ success, program }`
- `PUT /programs/:id` → `{ success, program }`
- `DELETE /programs/:id` → `{ success, id }`
- `POST /programs/upload-image` multipart → `{ success, imagePath }`

**Advertisements**

- `GET /advertisements` → `{ ads: Vec<Advertisement> }`
- `POST /advertisements` → `{ success, ad }`
- `PUT /advertisements/:id` → `{ success, ad }`
- `DELETE /advertisements/:id` → `{ success, id }`
- `POST /advertisements/upload-image` multipart → `{ success, imagePath }`

**System**

- `GET /has-data` → `{ has_data: bool }`
- `GET /export` → ZIP binary stream
- `POST /import` multipart → `{ success }`
- `GET /media/*path` → serves files from `{app_data_dir}/media/`
- `GET /`* → SvelteKit static build (SPA fallback to `index.html`)

---

## Phase 6: Socket.IO Event Handlers (`server/socket_handlers.rs`)

All handlers registered in `io.ns("/", |socket, State(state)| { ... })`:


| Incoming Event                                                   | Action                                                                 |
| ---------------------------------------------------------------- | ---------------------------------------------------------------------- |
| `join-studio-room { studioId }`                                  | `socket.join("studio:{id}")`                                           |
| `leave-studio-room { studioId }`                                 | `socket.leave("studio:{id}")`                                          |
| `get-studio-state { studioId }`                                  | emit `studio-state` to this socket                                     |
| `select-program { studioId, programId }`                         | update DB + memory, broadcast `program-selected` to room               |
| `trigger-overlay { studioId, graphicId, graphicPath, allowAds }` | update state, broadcast `overlay-activated` to room                    |
| `trigger-ad { studioId, adId, imagePath, duration }`             | update state, broadcast `ad-started` to room                           |
| `trigger-obs-command { studioId, commandId, shortcut }`          | call `keyboard::fire_shortcut()`, broadcast `obs-command-fired`        |
| `create-studio / edit-studio / delete-studio`                    | DB op, broadcast `studio-created/updated/deleted` + `update-studios`   |
| `create-program / edit-program / delete-program`                 | DB op, broadcast `program-created/updated/deleted` + `update-programs` |
| `create-ad / edit-ad / delete-ad`                                | DB op, broadcast `ad-created/updated/deleted` + `update-ads`           |


⚠️ `keyboard::fire_shortcut` must be called inside `tokio::task::spawn_blocking` (Enigo is not async-safe).

---

## Phase 7: Keyboard Handler (`keyboard.rs`)

```rust
pub fn fire_shortcut(shortcut: &str) -> anyhow::Result<()> {
    let key = match shortcut {
        "F13" => Key::Other(0x7C), "F14" => Key::Other(0x7D),
        "F15" => Key::Other(0x7E), "F16" => Key::Other(0x7F),
        "F17" => Key::Other(0x80), "F18" => Key::Other(0x81),
        "F19" => Key::Other(0x82), "F20" => Key::Other(0x83),
        "F21" => Key::Other(0x84), "F22" => Key::Other(0x85),
        "F23" => Key::Other(0x86), "F24" => Key::Other(0x87),
        _ => return Err(anyhow::anyhow!("Unknown shortcut: {shortcut}")),
    };
    let mut enigo = Enigo::new(&Settings::default())?;
    enigo.key(key, Direction::Click)?;
    Ok(())
}
```

---

## Phase 8: `lib.rs` Rewrite

Start the Axum server as a Tokio task inside Tauri's `.setup()` hook — this runs before the webview loads:

```rust
.setup(move |_app| {
    let state = app_state.clone();
    tauri::async_runtime::spawn(async move {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:5000").await.unwrap();
        axum::serve(listener, router).await.unwrap();
    });
    Ok(())
})
```

DB is opened, `run_migrations()` called, and `AppState` initialized before `.setup()` is called.

---

## Phase 9: Frontend Changes

### Files to create

`**src/lib/stores/toasts.ts**` (currently missing — all pages import from it)

```typescript
import { writable } from 'svelte/store';
import type { Toast } from '$lib/api/types';

export const toasts = writable<Toast[]>([]);
let nextId = 0;

export function addToast(type: Toast['type'], message: string, durationMs = 3500) {
    const id = nextId++;
    toasts.update(t => [...t, { id, type, message }]);
    setTimeout(() => toasts.update(t => t.filter(x => x.id !== id)), durationMs);
}
```

`**src/lib/bridge.ts**` (new)

```typescript
export const IS_TAURI = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
// Desktop Tauri always uses localhost:5000 (Axum server is local)
// Remote browser uses its current origin in prod, or localhost:5000 in dev
export const BACKEND_URL = IS_TAURI
    ? 'http://localhost:5000'
    : (import.meta.env.DEV ? 'http://localhost:5000' : window.location.origin);
```

Note: Since Axum always runs at localhost:5000 even in the desktop app, the bridge does NOT need to route through Tauri IPC for CRUD. Both desktop and remote browser use the same HTTP/Socket.IO layer. `invoke()` is reserved for future native-only features.

### Files to update

`**src/lib/api/socket.ts**`

- Change `BACKEND_URL` import to come from `$lib/bridge` instead of being defined here
- Re-export `BACKEND_URL` so existing imports in other files don't break

`**src/lib/api/api.ts**`

- Change `import { BACKEND_URL } from './socket'` → `import { BACKEND_URL } from '$lib/bridge'`

`**src/lib/api/types.ts**`

- Add `Preset` interface `{ id, studio_id, name, commands: ObsCommand[] }`
- Add `preset_id?: number` to `ObsCommand`
- Add `presets: Preset[]` to `Studio`
- Add `'filler'` to `ProgramAd.ad_launch_type` union (rename to `trigger_type` internally but keep JSON name)
- Add `comments: string` and `media_type?: string` to `Advertisement`
- Add `comments?: string` and `media_type?: string` to `Graphic`

`**package.json**`

- Add `"socket.io-client": "^4.8.0"` (currently used but not listed)

**Routing fix** — `control/+page.svelte` line 33:

- Change `goto('/studio-selector')` → `goto('/')`
- Same fix in `program-selector/+page.svelte` if present

`**studio-editor/+page.svelte`**

- Extend studio create/edit modal to show a **Presets** section with tabs/accordion
- Each preset has a name and a list of commands (existing command rows)
- The socket emit for `edit-studio` needs to send `{ id, name, presets: [...] }`
- On load, `studio.presets` (from `fetchStudios()`) populates the Presets UI

### OBS Overlay Pages to create

`**src/routes/obs/+page.svelte`**

- Connects via Socket.IO (same `socket` import)
- Listens for `overlay-activated` / `overlay-deactivated` events
- Renders `<img>` or `<video>` based on `media_type`
- Full-screen, `background: transparent` (for OBS chroma key / transparent BG)
- OBS browser source URL: `http://{machine-ip}:5000/obs?studio={id}`

`**src/routes/obs-background/+page.svelte**`

- Listens for `program-selected` / `program-cleared`
- Shows `program.background_graphics_path` as full-screen background image/video

### SvelteKit config check

Ensure `svelte.config.js` has:

```javascript
adapter({ fallback: 'index.html' })
```

This enables the SPA fallback so Axum's `ServeDir` correctly serves `/obs`, `/control`, etc.

---

## Phase 10: Media File Handling

Upload handler writes files to `{app_data_dir}/media/{entity_type}/{entity_id}/{uuid}.{ext}`.
Returns relative path `media/programs/1/abc123.jpg`.
`imgUrl()` in `api.ts` prepends `BACKEND_URL` → `http://localhost:5000/media/programs/1/abc123.jpg`.
Axum serves `/media/*` via `ServeDir` pointing at `{app_data_dir}/media/`.

---

## Phase 11: Import/Export

**Export:** ZIP containing `app.db` + full `media/` tree → streamed as `application/zip`.
**Import:** Receive ZIP, acquire DB write lock, extract `app.db` + `media/` overwriting existing, re-run migrations, broadcast `update-studios/programs/ads`.

---

## Critical Files


| File                                      | Action                                           |
| ----------------------------------------- | ------------------------------------------------ |
| `src-tauri/Cargo.toml`                    | Add all new dependencies                         |
| `src-tauri/src/lib.rs`                    | Full rewrite — wires server, DB, state, commands |
| `src-tauri/src/db/schema.rs`              | Create — full SQL schema                         |
| `src-tauri/src/models/mod.rs`             | Create — all Rust structs with serde renames     |
| `src-tauri/src/server/mod.rs`             | Create — Axum router                             |
| `src-tauri/src/server/socket_handlers.rs` | Create — all Socket.IO events                    |
| `src-tauri/src/keyboard.rs`               | Create — Enigo wrapper                           |
| `src/lib/bridge.ts`                       | Create — BACKEND_URL, IS_TAURI                   |
| `src/lib/stores/toasts.ts`                | Create — missing store                           |
| `src/lib/api/types.ts`                    | Update — Preset, filler, comments                |
| `src/lib/api/socket.ts`                   | Update — import BACKEND_URL from bridge          |
| `src/routes/studio-editor/+page.svelte`   | Update — Presets layer UI                        |
| `src/routes/obs/+page.svelte`     | Create — OBS graphic overlay page                |
| `src/routes/obs-background/+page.svelte`  | Create — OBS background page                     |
| `src/routes/control/+page.svelte`         | Fix goto('/studio-selector') → goto('/')         |


---

## Verification

1. `cargo build` — confirms Rust compiles with all dependencies
2. `npm run tauri dev` — app launches, Axum starts on :5000, webview loads
3. Open `http://localhost:5000` in a browser — same UI, same data as desktop (remote parity)
4. Create a studio with a preset and commands → verify in DB with `sqlite3 app.db`
5. Open `http://localhost:5000/obs?studio=1` in browser / OBS — trigger overlay from control page → overlay updates in real time
6. Trigger an OBS command from a remote browser → verify F13-F24 keypress fires on host machine
7. Export → wipe DB → import → verify data restored

