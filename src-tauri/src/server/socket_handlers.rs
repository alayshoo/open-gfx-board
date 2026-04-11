use socketioxide::extract::{Data, SocketRef, State};
use socketioxide::SocketIo;
use serde_json::Value;
use crate::state::AppState;
use crate::models::{ActiveOverlay, ActivePopup, StudioState};
use crate::html_template::{self, TemplateContext};

/// Fixed studio ID – there is only one studio now.
const STUDIO_ID: i64 = 1;
/// Fixed Socket.IO room – all clients share a single room.
const ROOM: &str = "studio";

/// Build a [`TemplateContext`] from the current runtime state, resolving the
/// program name (if any) by querying the database.
fn build_template_context(
    program_id: Option<i64>,
    conn: &rusqlite::Connection,
) -> TemplateContext {
    let program_name = program_id.and_then(|pid| {
        crate::db::programs::get_program(conn, pid)
            .ok()
            .flatten()
            .map(|p| p.name)
    });

    let studio_name: Option<String> = conn
        .query_row(
            "SELECT name FROM studios WHERE id = ?1",
            [STUDIO_ID],
            |row| row.get(0),
        )
        .ok();

    TemplateContext::from_runtime(
        program_name.as_deref(),
        studio_name.as_deref(),
    )
}

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
                        media_type: runtime.active_screen_media_type.clone().unwrap_or_else(|| "image".to_string()),
                        html_content: runtime.active_screen_html_content.clone(),
                    });

                    let active_popup = runtime.active_popup_id.map(|pid| ActivePopup {
                        popup_id: pid,
                        image_path: runtime.active_popup_path.clone(),
                        duration: runtime.active_popup_duration,
                        direction: runtime.active_popup_direction.clone().unwrap_or_else(|| "bottom".to_string()),
                        position: runtime.active_popup_position.unwrap_or(50),
                        media_type: runtime.active_popup_media_type.clone().unwrap_or_else(|| "image".to_string()),
                        html_content: runtime.active_popup_html_content.clone(),
                        width: runtime.active_popup_width,
                        height: runtime.active_popup_height,
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
                        s.active_screen_media_type = None;
                        s.active_screen_html_content = None;
                        s.active_popup_id = None;
                        s.active_popup_path = None;
                        s.active_popup_duration = 0;
                        s.active_popup_direction = None;
                        s.active_popup_position = None;
                        s.active_popup_media_type = None;
                        s.active_popup_html_content = None;
                        s.active_popup_width = None;
                        s.active_popup_height = None;
                    }

                    if let Some(pid) = program_id {
                        // Fetch program and build overlay inside a scoped db lock
                        let (program, first_overlay) = {
                            let db = state_c.db.lock().await;
                            let prog = tokio::task::block_in_place(|| {
                                crate::db::programs::get_program(&db, pid).ok().flatten()
                            });

                            // Automatically activate the first screen overlay for the new program.
                            // If the first screen is HTML, process the template now.
                            let overlay = prog.as_ref()
                                .and_then(|p| p.screens.first())
                                .map(|s| {
                                    let processed_html = if s.media_type == "html" {
                                        s.html_content.as_ref().map(|raw| {
                                            let ctx = build_template_context(Some(pid), &db);
                                            html_template::process_template(raw, &ctx, Some(&db))
                                        })
                                    } else {
                                        None
                                    };
                                    ActiveOverlay {
                                        graphic_id: s.id,
                                        graphic_path: s.media_path.clone(),
                                        allow_popups: s.allow_popups,
                                        media_type: s.media_type.clone(),
                                        html_content: processed_html,
                                    }
                                });

                            (prog, overlay)
                        }; // db lock released here

                        if let Some(ref overlay) = first_overlay {
                            let mut states = state_c.studio_states.lock().await;
                            let s = states.entry(STUDIO_ID).or_default();
                            s.active_screen_id = Some(overlay.graphic_id);
                            s.active_screen_path = overlay.graphic_path.clone();
                            s.active_screen_allow_popups = overlay.allow_popups;
                            s.active_screen_media_type = Some(overlay.media_type.clone());
                            s.active_screen_html_content = overlay.html_content.clone();
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
                                "mediaType": overlay.media_type,
                                "htmlContent": overlay.html_content,
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
        //
        // Always fetches fresh screen data from the database (like trigger-popup)
        // so that HTML content and other fields are up-to-date.
        {
            let state_c = state.clone();
            let io_cc = io_c.clone();
            socket.on("trigger-overlay", move |_socket: SocketRef, Data(data): Data<Value>| {
                let state_c = state_c.clone();
                let io_cc = io_cc.clone();
                async move {
                    let graphic_id = data.get("graphicId").and_then(|v| v.as_i64());
                    let allow_popups = data.get("allowPopUps").and_then(|v| v.as_bool()).unwrap_or(false);

                    // Fetch fresh screen data from DB inside a scoped lock, then release before
                    // acquiring studio_states to avoid potential deadlocks.
                    let (graphic_path, media_type, processed_html) = if let Some(gid) = graphic_id {
                        let db = state_c.db.lock().await;
                        let screen = tokio::task::block_in_place(|| {
                            crate::db::screens::get_screen(&db, gid).ok().flatten()
                        });
                        match screen {
                            Some(s) => {
                                let html = if s.media_type == "html" {
                                    s.html_content.as_ref().map(|raw| {
                                        // Use try_lock for studio_states to get program_id without
                                        // risking a deadlock (we already hold db).
                                        let program_id = state_c.studio_states.try_lock()
                                            .ok()
                                            .and_then(|states| states.get(&STUDIO_ID).and_then(|r| r.program_id));
                                        let ctx = build_template_context(program_id, &db);
                                        html_template::process_template(raw, &ctx, Some(&db))
                                    })
                                } else {
                                    None
                                };
                                (s.media_path, s.media_type, html)
                            }
                            None => {
                                // Fallback: use data from the client
                                let gp = data.get("graphicPath").and_then(|v| v.as_str()).map(String::from);
                                (gp, "image".to_string(), None)
                            }
                        }
                    } else {
                        let gp = data.get("graphicPath").and_then(|v| v.as_str()).map(String::from);
                        (gp, "image".to_string(), None)
                    };
                    // db lock is now released

                    {
                        let mut states = state_c.studio_states.lock().await;
                        let s = states.entry(STUDIO_ID).or_default();
                        s.active_screen_id = graphic_id;
                        s.active_screen_path = graphic_path.clone();
                        s.active_screen_allow_popups = allow_popups;
                        s.active_screen_media_type = Some(media_type.clone());
                        s.active_screen_html_content = processed_html.clone();
                    }

                    let payload = serde_json::json!({
                        "studioId": STUDIO_ID,
                        "graphicId": graphic_id,
                        "graphicPath": graphic_path,
                        "allowPopUps": allow_popups,
                        "mediaType": media_type,
                        "htmlContent": processed_html,
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
                        s.active_screen_media_type = None;
                        s.active_screen_html_content = None;
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

                    // Fetch fresh popup data and process any HTML template inside a
                    // scoped db lock, then release before acquiring studio_states.
                    let result = {
                        let db = state_c.db.lock().await;
                        let popup = tokio::task::block_in_place(|| {
                            crate::db::popups::get_popup(&db, popup_id).ok().flatten()
                        });
                        match popup {
                            Some(p) => {
                                let mt = p.media_type.clone();
                                let html = if mt == "html" {
                                    p.html_content.as_ref().map(|raw| {
                                        let program_id = state_c.studio_states.try_lock()
                                            .ok()
                                            .and_then(|states| states.get(&STUDIO_ID).and_then(|r| r.program_id));
                                        let ctx = build_template_context(program_id, &db);
                                        html_template::process_template(raw, &ctx, Some(&db))
                                    })
                                } else {
                                    None
                                };
                                Some((p.media_path.clone(), p.direction.clone(), p.position, mt, html, p.width, p.height))
                            }
                            None => None,
                        }
                    }; // db lock released here

                    let Some((image_path, direction, position, media_type, processed_html, width, height)) = result else { return; };

                    {
                        let mut states = state_c.studio_states.lock().await;
                        let s = states.entry(STUDIO_ID).or_default();
                        s.active_popup_id = Some(popup_id);
                        s.active_popup_path = image_path.clone();
                        s.active_popup_duration = duration;
                        s.active_popup_direction = Some(direction.clone());
                        s.active_popup_position = Some(position);
                        s.active_popup_media_type = Some(media_type.clone());
                        s.active_popup_html_content = processed_html.clone();
                        s.active_popup_width = width;
                        s.active_popup_height = height;
                    }

                    let payload = serde_json::json!({
                        "studioId": STUDIO_ID,
                        "popupId": popup_id,
                        "imagePath": image_path,
                        "duration": duration,
                        "direction": direction,
                        "position": position,
                        "mediaType": media_type,
                        "htmlContent": processed_html,
                        "width": width,
                        "height": height,
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
                        s.active_popup_media_type = None;
                        s.active_popup_html_content = None;
                        s.active_popup_width = None;
                        s.active_popup_height = None;
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
