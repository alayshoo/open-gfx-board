use socketioxide::extract::{Data, SocketRef, State};
use socketioxide::SocketIo;
use serde_json::Value;
use crate::state::AppState;
use crate::models::{ActiveOverlay, ActivePopup, StudioState};

/// Fixed studio ID – there is only one studio now.
const STUDIO_ID: i64 = 1;
/// Fixed Socket.IO room – all clients share a single room.
const ROOM: &str = "studio";

pub fn register_handlers(io: &SocketIo, state: AppState) {
    let io_c = io.clone();
    io.ns("/", move |socket: SocketRef, State(state): State<AppState>| {
        let io_c = io_c.clone();
        async move {

        // ── join-studio-room ──────────────────────────────────────────────
        {
            let state_c = state.clone();
            socket.on("join-studio-room", move |socket: SocketRef, _: Data<Value>| {
                let state_c = state_c.clone();
                async move {
                    let _ = socket.join(ROOM);
                    let mut states = state_c.studio_states.lock().await;
                    states.entry(STUDIO_ID).or_default();
                }
            });
        }

        // ── leave-studio-room ─────────────────────────────────────────────
        {
            socket.on("leave-studio-room", move |socket: SocketRef, _: Data<Value>| async move {
                let _ = socket.leave(ROOM);
            });
        }

        // ── get-studio-state ──────────────────────────────────────────────
        // Responds only to the requesting socket with the current studio snapshot.
        {
            let state_c = state.clone();
            socket.on("get-studio-state", move |socket: SocketRef, _: Data<Value>| {
                let state_c = state_c.clone();
                async move {
                    let runtime = {
                        let states = state_c.studio_states.lock().await;
                        states.get(&STUDIO_ID).cloned().unwrap_or_default()
                    };

                    // Load the full program object so clients can hydrate without an extra HTTP call
                    let program = if let Some(pid) = runtime.program_id {
                        let db = state_c.db.lock().await;
                        tokio::task::block_in_place(|| {
                            crate::db::programs::get_program(&db, pid).ok().flatten()
                        })
                    } else {
                        None
                    };

                    let active_overlay = runtime.active_screen_id.map(|gid| ActiveOverlay {
                        graphic_id: gid,
                        graphic_path: runtime.active_screen_path.clone(),
                        allow_popups: runtime.active_screen_allow_popups,
                    });

                    let active_popup = runtime.active_popup_id.map(|pid| ActivePopup {
                        popup_id: pid,
                        image_path: runtime.active_popup_path.clone(),
                        duration: runtime.active_popup_duration,
                        direction: runtime.active_popup_direction.clone().unwrap_or_else(|| "bottom".to_string()),
                        position: runtime.active_popup_position.unwrap_or(50),
                    });

                    let studio_state = StudioState {
                        studio_id: STUDIO_ID,
                        program_id: runtime.program_id,
                        program,
                        active_overlay,
                        active_popup,
                    };

                    let _ = socket.emit("studio-state", &studio_state);
                }
            });
        }

        // ── select-program ────────────────────────────────────────────────
        // Broadcasts program-selected (or program-cleared when programId is null)
        // to every client in the studio room, including the sender.
        {
            let state_c = state.clone();
            let io_cc = io_c.clone();
            socket.on("select-program", move |_socket: SocketRef, Data(data): Data<Value>| {
                let state_c = state_c.clone();
                let io_cc = io_cc.clone();
                async move {
                    let program_id = data.get("programId").and_then(|v| v.as_i64());

                    // Update runtime state; changing program always resets overlay / popup
                    {
                        let mut states = state_c.studio_states.lock().await;
                        let s = states.entry(STUDIO_ID).or_default();
                        s.program_id = program_id;
                        s.active_screen_id = None;
                        s.active_screen_path = None;
                        s.active_screen_allow_popups = false;
                        s.active_popup_id = None;
                        s.active_popup_path = None;
                        s.active_popup_duration = 0;
                        s.active_popup_direction = None;
                        s.active_popup_position = None;
                    }

                    if let Some(pid) = program_id {
                        let program = {
                            let db = state_c.db.lock().await;
                            tokio::task::block_in_place(|| {
                                crate::db::programs::get_program(&db, pid).ok().flatten()
                            })
                        };

                        // Automatically activate the first screen overlay for the new program
                        let first_overlay = program.as_ref()
                            .and_then(|p| p.screens.first())
                            .map(|s| ActiveOverlay {
                                graphic_id: s.id,
                                graphic_path: s.media_path.clone(),
                                allow_popups: s.allow_popups,
                            });

                        if let Some(ref overlay) = first_overlay {
                            let mut states = state_c.studio_states.lock().await;
                            let s = states.entry(STUDIO_ID).or_default();
                            s.active_screen_id = Some(overlay.graphic_id);
                            s.active_screen_path = overlay.graphic_path.clone();
                            s.active_screen_allow_popups = overlay.allow_popups;
                        }

                        let payload = serde_json::json!({
                            "studioId": STUDIO_ID,
                            "programId": pid,
                            "program": program,
                            "activeOverlay": first_overlay,
                            "activePopup": null,
                        });
                        let _ = io_cc.within(ROOM).emit("program-selected", &payload).await;

                        // Also emit overlay-activated so the OBS overlay page reacts immediately
                        if let Some(ref overlay) = first_overlay {
                            let overlay_payload = serde_json::json!({
                                "studioId": STUDIO_ID,
                                "graphicId": overlay.graphic_id,
                                "graphicPath": overlay.graphic_path,
                                "allowPopUps": overlay.allow_popups,
                            });
                            let _ = io_cc.within(ROOM).emit("overlay-activated", &overlay_payload).await;
                        }
                    } else {
                        // programId: null means the operator is clearing the active program
                        let payload = serde_json::json!({ "studioId": STUDIO_ID });
                        let _ = io_cc.within(ROOM).emit("program-cleared", &payload).await;
                    }
                }
            });
        }

        // ── trigger-overlay ───────────────────────────────────────────────
        // Uses io (not socket) so the triggering client also receives the event,
        // keeping its own UI in sync with every other client in the room.
        {
            let state_c = state.clone();
            let io_cc = io_c.clone();
            socket.on("trigger-overlay", move |_socket: SocketRef, Data(data): Data<Value>| {
                let state_c = state_c.clone();
                let io_cc = io_cc.clone();
                async move {
                    let graphic_id = data.get("graphicId").and_then(|v| v.as_i64());
                    let graphic_path = data.get("graphicPath").and_then(|v| v.as_str()).map(String::from);
                    let allow_popups = data.get("allowPopUps").and_then(|v| v.as_bool()).unwrap_or(false);

                    {
                        let mut states = state_c.studio_states.lock().await;
                        let s = states.entry(STUDIO_ID).or_default();
                        s.active_screen_id = graphic_id;
                        s.active_screen_path = graphic_path.clone();
                        s.active_screen_allow_popups = allow_popups;
                    }

                    let payload = serde_json::json!({
                        "studioId": STUDIO_ID,
                        "graphicId": graphic_id,
                        "graphicPath": graphic_path,
                        "allowPopUps": allow_popups,
                    });
                    let _ = io_cc.within(ROOM).emit("overlay-activated", &payload).await;
                }
            });
        }

        // ── deactivate-overlay ────────────────────────────────────────────
        {
            let state_c = state.clone();
            let io_cc = io_c.clone();
            socket.on("deactivate-overlay", move |_socket: SocketRef, _: Data<Value>| {
                let state_c = state_c.clone();
                let io_cc = io_cc.clone();
                async move {
                    {
                        let mut states = state_c.studio_states.lock().await;
                        let s = states.entry(STUDIO_ID).or_default();
                        s.active_screen_id = None;
                        s.active_screen_path = None;
                        s.active_screen_allow_popups = false;
                    }

                    let payload = serde_json::json!({ "studioId": STUDIO_ID });
                    let _ = io_cc.within(ROOM).emit("overlay-deactivated", &payload).await;
                }
            });
        }

        // ── trigger-popup ─────────────────────────────────────────────────
        // Uses io (not socket) so the triggering client also gets the event.
        // Only popupId + duration come from the client; image_path / direction /
        // position are always fetched fresh from the database so that stale
        // values held by any controller never reach the OBS overlay.
        {
            let state_c = state.clone();
            let io_cc = io_c.clone();
            socket.on("trigger-popup", move |_socket: SocketRef, Data(data): Data<Value>| {
                let state_c = state_c.clone();
                let io_cc = io_cc.clone();
                async move {
                    let Some(popup_id) = data.get("popupId").and_then(|v| v.as_i64()) else { return; };
                    let duration = data.get("duration").and_then(|v| v.as_i64()).unwrap_or(10);

                    // Fetch fresh popup data from the database so direction / position /
                    // image_path are always up-to-date regardless of which controller
                    // triggered the popup.
                    let popup = {
                        let db = state_c.db.lock().await;
                        tokio::task::block_in_place(|| {
                            crate::db::popups::get_popup(&db, popup_id).ok().flatten()
                        })
                    };
                    let Some(popup) = popup else { return; };

                    let image_path = popup.media_path.clone();
                    let direction = popup.direction.clone();
                    let position = popup.position;

                    {
                        let mut states = state_c.studio_states.lock().await;
                        let s = states.entry(STUDIO_ID).or_default();
                        s.active_popup_id = Some(popup_id);
                        s.active_popup_path = image_path.clone();
                        s.active_popup_duration = duration;
                        s.active_popup_direction = Some(direction.clone());
                        s.active_popup_position = Some(position);
                    }

                    let payload = serde_json::json!({
                        "studioId": STUDIO_ID,
                        "popupId": popup_id,
                        "imagePath": image_path,
                        "duration": duration,
                        "direction": direction,
                        "position": position,
                    });
                    let _ = io_cc.within(ROOM).emit("popup-started", &payload).await;
                }
            });
        }

        // ── end-popup ─────────────────────────────────────────────────────
        {
            let state_c = state.clone();
            let io_cc = io_c.clone();
            socket.on("end-popup", move |_socket: SocketRef, _: Data<Value>| {
                let state_c = state_c.clone();
                let io_cc = io_cc.clone();
                async move {
                    {
                        let mut states = state_c.studio_states.lock().await;
                        let s = states.entry(STUDIO_ID).or_default();
                        s.active_popup_id = None;
                        s.active_popup_path = None;
                        s.active_popup_duration = 0;
                        s.active_popup_direction = None;
                        s.active_popup_position = None;
                    }

                    let payload = serde_json::json!({ "studioId": STUDIO_ID });
                    let _ = io_cc.within(ROOM).emit("popup-ended", &payload).await;
                }
            });
        }

        // ── trigger-obs-command ───────────────────────────────────────────
        {
            let io_cc = io_c.clone();
            socket.on("trigger-obs-command", move |_socket: SocketRef, Data(data): Data<Value>| {
                let io_cc = io_cc.clone();
                async move {
                    let command_id = data.get("commandId").and_then(|v| v.as_i64());
                    let shortcut = data.get("shortcut").and_then(|v| v.as_str()).map(String::from);

                    if let Some(ref sc) = shortcut {
                        let sc_clone = sc.clone();
                        let result = tokio::task::spawn_blocking(move || {
                            crate::keyboard::fire_shortcut(&sc_clone)
                        }).await;
                        if let Err(e) = result {
                            eprintln!("Keyboard shortcut task failed: {e}");
                        }
                    }

                    let payload = serde_json::json!({
                        "studioId": STUDIO_ID,
                        "commandId": command_id,
                        "shortcut": shortcut,
                    });
                    let _ = io_cc.within(ROOM).emit("obs-command-fired", &payload).await;
                }
            });
        }

    }});
}
