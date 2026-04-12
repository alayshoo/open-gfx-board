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
                                            let mut ctx = build_template_context(Some(pid), &db);

                                            // Check if this is a plugin screen
                                            let plugin_id: Option<String> = db.query_row(
                                                "SELECT plugin_id FROM screens WHERE id = ?1",
                                                [s.id],
                                                |r| r.get(0),
                                            ).ok().flatten();

                                            if let Some(ref plid) = plugin_id {
                                                if let Ok(ps) = state_c.plugin_states.try_lock() {
                                                    if let Some(pstate) = ps.get(plid) {
                                                        let mut vars = std::collections::HashMap::new();
                                                        for (k, v) in pstate {
                                                            vars.insert(k.clone(), match v {
                                                                serde_json::Value::String(sv) => sv.clone(),
                                                                serde_json::Value::Null => String::new(),
                                                                other => other.to_string(),
                                                            });
                                                        }
                                                        ctx.plugin_contexts.insert(plid.clone(), vars);
                                                    }
                                                }
                                            }

                                            let mut processed = html_template::process_template(raw, &ctx, Some(&db));

                                            // Inject live plugin SDK for plugin screens
                                            if let Some(ref plid) = plugin_id {
                                                processed = crate::plugins::sdk_injection::inject_sdk(&processed, plid, true);
                                            }

                                            processed
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
                                        let mut ctx = build_template_context(program_id, &db);

                                        // If this is a plugin screen, populate plugin state in the
                                        // template context so {{plugin:…}} expressions resolve.
                                        let plugin_id: Option<String> = db.query_row(
                                            "SELECT plugin_id FROM screens WHERE id = ?1",
                                            [gid],
                                            |r| r.get(0),
                                        ).ok().flatten();

                                        if let Some(ref pid) = plugin_id {
                                            if let Ok(ps) = state_c.plugin_states.try_lock() {
                                                if let Some(pstate) = ps.get(pid) {
                                                    let mut vars = std::collections::HashMap::new();
                                                    for (k, v) in pstate {
                                                        vars.insert(k.clone(), match v {
                                                            serde_json::Value::String(s) => s.clone(),
                                                            serde_json::Value::Null => String::new(),
                                                            other => other.to_string(),
                                                        });
                                                    }
                                                    ctx.plugin_contexts.insert(pid.clone(), vars);
                                                }
                                            }
                                        }

                                        let mut processed = html_template::process_template(raw, &ctx, Some(&db));

                                        // Inject the live plugin SDK for plugin screens
                                        if let Some(ref pid) = plugin_id {
                                            processed = crate::plugins::sdk_injection::inject_sdk(&processed, pid, true);
                                        }

                                        processed
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
                                        let mut ctx = build_template_context(program_id, &db);

                                        // Check if this is a plugin popup
                                        let plugin_id: Option<String> = db.query_row(
                                            "SELECT plugin_id FROM popups WHERE id = ?1",
                                            [popup_id],
                                            |r| r.get(0),
                                        ).ok().flatten();

                                        if let Some(ref pid) = plugin_id {
                                            if let Ok(ps) = state_c.plugin_states.try_lock() {
                                                if let Some(pstate) = ps.get(pid) {
                                                    let mut vars = std::collections::HashMap::new();
                                                    for (k, v) in pstate {
                                                        vars.insert(k.clone(), match v {
                                                            serde_json::Value::String(s) => s.clone(),
                                                            serde_json::Value::Null => String::new(),
                                                            other => other.to_string(),
                                                        });
                                                    }
                                                    ctx.plugin_contexts.insert(pid.clone(), vars);
                                                }
                                            }
                                        }

                                        let mut processed = html_template::process_template(raw, &ctx, Some(&db));

                                        // Inject SDK for plugin popups
                                        if let Some(ref pid) = plugin_id {
                                            processed = crate::plugins::sdk_injection::inject_sdk(&processed, pid, false);
                                        }

                                        processed
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

        // ── plugin-set-state ────────────────────────────────────────────
        {
            let state_c = state.clone();
            let io_cc = io_c.clone();
            socket.on("plugin-set-state", move |_socket: SocketRef, Data(data): Data<Value>| {
                let state_c = state_c.clone();
                let io_cc = io_cc.clone();
                async move {
                    let plugin_id = match data.get("pluginId").and_then(|v| v.as_str()) {
                        Some(id) => id.to_string(),
                        None => return,
                    };
                    let updates = match data.get("updates").and_then(|v| v.as_object()) {
                        Some(obj) => obj.clone(),
                        None => return,
                    };

                    let updates_map: std::collections::HashMap<String, serde_json::Value> =
                        updates.into_iter().collect();

                    // Write to DB
                    {
                        let db = state_c.db.lock().await;
                        if let Err(e) = crate::plugins::state::set_state_batch(&db, &plugin_id, &updates_map) {
                            eprintln!("plugin-set-state DB error: {e}");
                            return;
                        }
                    }

                    // Update in-memory cache
                    let full_state = {
                        let mut states = state_c.plugin_states.lock().await;
                        let entry = states.entry(plugin_id.clone()).or_default();
                        for (k, v) in &updates_map {
                            entry.insert(k.clone(), v.clone());
                        }
                        entry.clone()
                    };

                    let payload = serde_json::json!({
                        "pluginId": plugin_id,
                        "state": full_state,
                    });
                    let event = format!("plugin-state-updated:{}", plugin_id);
                    let _ = io_cc.within(ROOM).emit(&event, &payload).await;
                }
            });
        }

        // ── plugin-fire-event ────────────────────────────────────────────
        {
            let state_c = state.clone();
            let io_cc = io_c.clone();
            socket.on("plugin-fire-event", move |_socket: SocketRef, Data(data): Data<Value>| {
                let state_c = state_c.clone();
                let io_cc = io_cc.clone();
                async move {
                    let plugin_id = match data.get("pluginId").and_then(|v| v.as_str()) {
                        Some(id) => id.to_string(),
                        None => return,
                    };
                    let event_name = match data.get("event").and_then(|v| v.as_str()) {
                        Some(e) => e.to_string(),
                        None => return,
                    };
                    let event_data = data.get("data").cloned().unwrap_or(Value::Null);

                    // Validate event is declared in manifest
                    let valid = {
                        let manifests = state_c.plugin_manifests.lock().await;
                        manifests
                            .get(&plugin_id)
                            .map(|m| m.events.contains(&event_name))
                            .unwrap_or(false)
                    };
                    if !valid {
                        return;
                    }

                    let payload = serde_json::json!({
                        "pluginId": plugin_id,
                        "event": event_name,
                        "data": event_data,
                    });
                    let event = format!("plugin-event:{}:{}", plugin_id, event_name);
                    let _ = io_cc.within(ROOM).emit(&event, &payload).await;
                }
            });
        }

        // ── plugin-trigger-popup ─────────────────────────────────────────
        {
            let state_c = state.clone();
            let io_cc = io_c.clone();
            socket.on("plugin-trigger-popup", move |_socket: SocketRef, Data(data): Data<Value>| {
                let state_c = state_c.clone();
                let io_cc = io_cc.clone();
                async move {
                    let plugin_id = match data.get("pluginId").and_then(|v| v.as_str()) {
                        Some(id) => id.to_string(),
                        None => return,
                    };
                    let template_id = match data.get("templateId").and_then(|v| v.as_str()) {
                        Some(t) => t.to_string(),
                        None => return,
                    };
                    let context: std::collections::HashMap<String, String> = data
                        .get("context")
                        .and_then(|v| v.as_object())
                        .map(|obj| {
                            obj.iter()
                                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                                .collect()
                        })
                        .unwrap_or_default();
                    let duration_override = data.get("duration").and_then(|v| v.as_i64());

                    // Find popup definition in manifest
                    let popup_def = {
                        let manifests = state_c.plugin_manifests.lock().await;
                        manifests
                            .get(&plugin_id)
                            .and_then(|m| m.popups.iter().find(|p| p.template_id == template_id).cloned())
                    };
                    let popup_def = match popup_def {
                        Some(p) => p,
                        None => return,
                    };

                    let duration = duration_override.unwrap_or(popup_def.duration);

                    // Read template from disk
                    let template_path = state_c
                        .app_data_dir
                        .join("plugins")
                        .join(&plugin_id)
                        .join(&popup_def.template);
                    let template_html = tokio::fs::read_to_string(&template_path)
                        .await
                        .unwrap_or_default();

                    // Build template context
                    let plugin_states = state_c.plugin_states.lock().await;
                    let plugin_st = plugin_states.get(&plugin_id).cloned().unwrap_or_default();
                    drop(plugin_states);

                    let mut ctx = crate::html_template::TemplateContext::new();
                    let mut plugin_vars = std::collections::HashMap::new();
                    for (key, val) in &plugin_st {
                        plugin_vars.insert(key.clone(), match val {
                            Value::String(s) => s.clone(),
                            Value::Null => String::new(),
                            other => other.to_string(),
                        });
                    }
                    ctx.plugin_contexts.insert(plugin_id.clone(), plugin_vars);
                    ctx.popup_context = context;
                    ctx.popup_plugin_id = Some(plugin_id.clone());

                    // Process template and inject SDK
                    let db = state_c.db.lock().await;
                    let processed_html = {
                        let processed = crate::html_template::process_template(&template_html, &ctx, Some(&db));
                        // Inject SDK (non-live: popups are short-lived but may
                        // still need fetch-based helpers like getData)
                        crate::plugins::sdk_injection::inject_sdk(&processed, &plugin_id, false)
                    };

                    // Find popup DB id
                    let popup_db_id: Option<i64> = db
                        .query_row(
                            "SELECT id FROM popups WHERE plugin_id = ?1 AND plugin_template_id = ?2",
                            rusqlite::params![plugin_id, template_id],
                            |r| r.get(0),
                        )
                        .ok();
                    drop(db);

                    let popup_id = popup_db_id.unwrap_or(0);

                    // Update runtime state
                    {
                        let mut studio_states = state_c.studio_states.lock().await;
                        let ss = studio_states.entry(STUDIO_ID).or_default();
                        ss.active_popup_id = Some(popup_id);
                        ss.active_popup_path = None;
                        ss.active_popup_duration = duration;
                        ss.active_popup_direction = Some(popup_def.direction.clone());
                        ss.active_popup_position = Some(popup_def.position);
                        ss.active_popup_media_type = Some("html".to_string());
                        ss.active_popup_html_content = Some(processed_html.clone());
                        ss.active_popup_width = popup_def.width;
                        ss.active_popup_height = popup_def.height;
                    }

                    let payload = serde_json::json!({
                        "studioId": STUDIO_ID,
                        "popupId": popup_id,
                        "imagePath": null,
                        "duration": duration,
                        "direction": popup_def.direction,
                        "position": popup_def.position,
                        "mediaType": "html",
                        "htmlContent": processed_html,
                        "width": popup_def.width,
                        "height": popup_def.height,
                    });
                    let _ = io_cc.within(ROOM).emit("popup-started", &payload).await;
                }
            });
        }

    }});
}
