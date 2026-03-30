use socketioxide::extract::{Data, SocketRef, State};
use socketioxide::SocketIo;
use serde_json::Value;
use crate::state::AppState;
use crate::models::{ActiveOverlay, ActiveAd, StudioState};

pub fn register_handlers(io: &SocketIo, state: AppState) {
    let io_c = io.clone();
    io.ns("/", move |socket: SocketRef, State(state): State<AppState>| {
        let io_c = io_c.clone();
        async move {

        // ── join-studio-room ──────────────────────────────────────────────
        {
            let state_c = state.clone();
            socket.on("join-studio-room", move |socket: SocketRef, Data(data): Data<Value>| {
                let state_c = state_c.clone();
                async move {
                    if let Some(studio_id) = data.get("studioId").and_then(|v| v.as_i64()) {
                        let room = format!("studio:{studio_id}");
                        let _ = socket.join(room);
                        // Ensure a runtime state entry exists for this studio
                        let mut states = state_c.studio_states.lock().await;
                        states.entry(studio_id).or_default();
                    }
                }
            });
        }

        // ── leave-studio-room ─────────────────────────────────────────────
        {
            socket.on("leave-studio-room", move |socket: SocketRef, Data(data): Data<Value>| async move {
                if let Some(studio_id) = data.get("studioId").and_then(|v| v.as_i64()) {
                    let room = format!("studio:{studio_id}");
                    let _ = socket.leave(room);
                }
            });
        }

        // ── get-studio-state ──────────────────────────────────────────────
        // Responds only to the requesting socket with the current studio snapshot.
        {
            let state_c = state.clone();
            socket.on("get-studio-state", move |socket: SocketRef, Data(data): Data<Value>| {
                let state_c = state_c.clone();
                async move {
                    let Some(studio_id) = data.get("studioId").and_then(|v| v.as_i64()) else { return; };

                    let runtime = {
                        let states = state_c.studio_states.lock().await;
                        states.get(&studio_id).cloned().unwrap_or_default()
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
                        allow_ads: runtime.active_screen_allow_ads,
                    });

                    let active_ad = runtime.active_ad_id.map(|aid| ActiveAd {
                        ad_id: aid,
                        image_path: runtime.active_ad_path.clone(),
                        duration: runtime.active_ad_duration,
                    });

                    let studio_state = StudioState {
                        studio_id,
                        program_id: runtime.program_id,
                        program,
                        active_overlay,
                        active_ad,
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
                    let Some(studio_id) = data.get("studioId").and_then(|v| v.as_i64()) else { return; };
                    let program_id = data.get("programId").and_then(|v| v.as_i64());

                    // Update runtime state; changing program always resets overlay / ad
                    {
                        let mut states = state_c.studio_states.lock().await;
                        let s = states.entry(studio_id).or_default();
                        s.program_id = program_id;
                        s.active_screen_id = None;
                        s.active_screen_path = None;
                        s.active_screen_allow_ads = false;
                        s.active_ad_id = None;
                        s.active_ad_path = None;
                        s.active_ad_duration = 0;
                    }

                    let room = format!("studio:{studio_id}");

                    if let Some(pid) = program_id {
                        let program = {
                            let db = state_c.db.lock().await;
                            tokio::task::block_in_place(|| {
                                crate::db::programs::get_program(&db, pid).ok().flatten()
                            })
                        };
                        let payload = serde_json::json!({
                            "studioId": studio_id,
                            "programId": pid,
                            "program": program,
                            "activeOverlay": null,
                            "activeAd": null,
                        });
                        let _ = io_cc.within(room).emit("program-selected", &payload).await;
                    } else {
                        // programId: null means the operator is clearing the active program
                        let payload = serde_json::json!({ "studioId": studio_id });
                        let _ = io_cc.within(room).emit("program-cleared", &payload).await;
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
                    let Some(studio_id) = data.get("studioId").and_then(|v| v.as_i64()) else { return; };
                    let graphic_id = data.get("graphicId").and_then(|v| v.as_i64());
                    let graphic_path = data.get("graphicPath").and_then(|v| v.as_str()).map(String::from);
                    let allow_ads = data.get("allowAds").and_then(|v| v.as_bool()).unwrap_or(false);

                    {
                        let mut states = state_c.studio_states.lock().await;
                        let s = states.entry(studio_id).or_default();
                        s.active_screen_id = graphic_id;
                        s.active_screen_path = graphic_path.clone();
                        s.active_screen_allow_ads = allow_ads;
                    }

                    let room = format!("studio:{studio_id}");
                    let payload = serde_json::json!({
                        "studioId": studio_id,
                        "graphicId": graphic_id,
                        "graphicPath": graphic_path,
                        "allowAds": allow_ads,
                    });
                    let _ = io_cc.within(room).emit("overlay-activated", &payload).await;
                }
            });
        }

        // ── deactivate-overlay ────────────────────────────────────────────
        {
            let state_c = state.clone();
            let io_cc = io_c.clone();
            socket.on("deactivate-overlay", move |_socket: SocketRef, Data(data): Data<Value>| {
                let state_c = state_c.clone();
                let io_cc = io_cc.clone();
                async move {
                    let Some(studio_id) = data.get("studioId").and_then(|v| v.as_i64()) else { return; };

                    {
                        let mut states = state_c.studio_states.lock().await;
                        let s = states.entry(studio_id).or_default();
                        s.active_screen_id = None;
                        s.active_screen_path = None;
                        s.active_screen_allow_ads = false;
                    }

                    let room = format!("studio:{studio_id}");
                    let payload = serde_json::json!({ "studioId": studio_id });
                    let _ = io_cc.within(room).emit("overlay-deactivated", &payload).await;
                }
            });
        }

        // ── trigger-ad ────────────────────────────────────────────────────
        // Uses io (not socket) so the triggering client also gets the event.
        {
            let state_c = state.clone();
            let io_cc = io_c.clone();
            socket.on("trigger-ad", move |_socket: SocketRef, Data(data): Data<Value>| {
                let state_c = state_c.clone();
                let io_cc = io_cc.clone();
                async move {
                    let Some(studio_id) = data.get("studioId").and_then(|v| v.as_i64()) else { return; };
                    let ad_id = data.get("adId").and_then(|v| v.as_i64());
                    let image_path = data.get("imagePath").and_then(|v| v.as_str()).map(String::from);
                    let duration = data.get("duration").and_then(|v| v.as_i64()).unwrap_or(10);

                    {
                        let mut states = state_c.studio_states.lock().await;
                        let s = states.entry(studio_id).or_default();
                        s.active_ad_id = ad_id;
                        s.active_ad_path = image_path.clone();
                        s.active_ad_duration = duration;
                    }

                    let room = format!("studio:{studio_id}");
                    let payload = serde_json::json!({
                        "studioId": studio_id,
                        "adId": ad_id,
                        "imagePath": image_path,
                        "duration": duration,
                    });
                    let _ = io_cc.within(room).emit("ad-started", &payload).await;
                }
            });
        }

        // ── end-ad ────────────────────────────────────────────────────────
        {
            let state_c = state.clone();
            let io_cc = io_c.clone();
            socket.on("end-ad", move |_socket: SocketRef, Data(data): Data<Value>| {
                let state_c = state_c.clone();
                let io_cc = io_cc.clone();
                async move {
                    let Some(studio_id) = data.get("studioId").and_then(|v| v.as_i64()) else { return; };

                    {
                        let mut states = state_c.studio_states.lock().await;
                        let s = states.entry(studio_id).or_default();
                        s.active_ad_id = None;
                        s.active_ad_path = None;
                        s.active_ad_duration = 0;
                    }

                    let room = format!("studio:{studio_id}");
                    let payload = serde_json::json!({ "studioId": studio_id });
                    let _ = io_cc.within(room).emit("ad-ended", &payload).await;
                }
            });
        }

        // ── trigger-obs-command ───────────────────────────────────────────
        {
            let io_cc = io_c.clone();
            socket.on("trigger-obs-command", move |_socket: SocketRef, Data(data): Data<Value>| {
                let io_cc = io_cc.clone();
                async move {
                    let Some(studio_id) = data.get("studioId").and_then(|v| v.as_i64()) else { return; };
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

                    let room = format!("studio:{studio_id}");
                    let payload = serde_json::json!({
                        "studioId": studio_id,
                        "commandId": command_id,
                        "shortcut": shortcut,
                    });
                    let _ = io_cc.within(room).emit("obs-command-fired", &payload).await;
                }
            });
        }

    }});
}
