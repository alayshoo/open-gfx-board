use socketioxide::extract::{Data, SocketRef, State};
use socketioxide::SocketIo;
use serde_json::Value;
use crate::state::AppState;
use crate::models::{ActiveOverlay, StudioState};

pub fn register_handlers(io: &SocketIo, state: AppState) {
    let io_c = io.clone();
    io.ns("/", move |socket: SocketRef, State(state): State<AppState>| {
        let io_c = io_c.clone();
        async move {
        // join-studio-room
        {
            let state_c = state.clone();
            socket.on("join-studio-room", move |socket: SocketRef, Data(data): Data<Value>| {
                let state_c = state_c.clone();
                async move {
                    if let Some(studio_id) = data.get("studioId").and_then(|v| v.as_i64()) {
                        let room = format!("studio:{studio_id}");
                        let _ = socket.join(room);
                        // ensure runtime state entry exists
                        let mut states = state_c.studio_states.lock().await;
                        states.entry(studio_id).or_default();
                    }
                }
            });
        }

        // leave-studio-room
        {
            socket.on("leave-studio-room", move |socket: SocketRef, Data(data): Data<Value>| async move {
                if let Some(studio_id) = data.get("studioId").and_then(|v| v.as_i64()) {
                    let room = format!("studio:{studio_id}");
                    let _ = socket.leave(room);
                }
            });
        }

        // get-studio-state
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
                    let program = if let Some(pid) = runtime.program_id {
                        let db = state_c.db.lock().await;
                        tokio::task::block_in_place(|| {
                            crate::db::programs::get_program(&db, pid).ok().flatten()
                        })
                    } else {
                        None
                    };
                    let active_overlay = runtime.active_screen_id.map(|gid| {
                        let path = program.as_ref()
                            .and_then(|p| p.screens.iter().find(|s| s.id == gid))
                            .and_then(|s| s.media_path.clone());
                        ActiveOverlay { graphic_id: gid, graphic_path: path }
                    });
                    let studio_state = StudioState {
                        studio_id,
                        program_id: runtime.program_id,
                        program,
                        active_overlay,
                    };
                    let _ = socket.emit("studio-state", &studio_state);
                }
            });
        }

        // select-program
        {
            let state_c = state.clone();
            let io_cc = io_c.clone();
            socket.on("select-program", move |_socket: SocketRef, Data(data): Data<Value>| {
                let state_c = state_c.clone();
                let io_cc = io_cc.clone();
                async move {
                    let Some(studio_id) = data.get("studioId").and_then(|v| v.as_i64()) else { return; };
                    let program_id = data.get("programId").and_then(|v| v.as_i64());
                    {
                        let mut states = state_c.studio_states.lock().await;
                        let s = states.entry(studio_id).or_default();
                        s.program_id = program_id;
                        s.active_screen_id = None;
                        s.active_ad_id = None;
                    }
                    let program = if let Some(pid) = program_id {
                        let db = state_c.db.lock().await;
                        tokio::task::block_in_place(|| {
                            crate::db::programs::get_program(&db, pid).ok().flatten()
                        })
                    } else {
                        None
                    };
                    let room = format!("studio:{studio_id}");
                    let payload = serde_json::json!({
                        "studioId": studio_id,
                        "programId": program_id,
                        "program": program,
                        "activeOverlay": null,
                    });
                    let _ = io_cc.within(room).emit("program-selected", &payload).await;
                }
            });
        }

        // trigger-overlay
        {
            let state_c = state.clone();
            socket.on("trigger-overlay", move |socket: SocketRef, Data(data): Data<Value>| {
                let state_c = state_c.clone();
                async move {
                    let Some(studio_id) = data.get("studioId").and_then(|v| v.as_i64()) else { return; };
                    let graphic_id = data.get("graphicId").and_then(|v| v.as_i64());
                    let graphic_path = data.get("graphicPath").and_then(|v| v.as_str()).map(String::from);
                    let allow_ads = data.get("allowAds").and_then(|v| v.as_bool()).unwrap_or(false);
                    {
                        let mut states = state_c.studio_states.lock().await;
                        let s = states.entry(studio_id).or_default();
                        s.active_screen_id = graphic_id;
                    }
                    let room = format!("studio:{studio_id}");
                    let payload = serde_json::json!({
                        "studioId": studio_id,
                        "graphicId": graphic_id,
                        "graphicPath": graphic_path,
                        "allowAds": allow_ads,
                    });
                    let _ = socket.within(room).emit("overlay-activated", &payload).await;
                }
            });
        }

        // trigger-ad
        {
            let state_c = state.clone();
            socket.on("trigger-ad", move |socket: SocketRef, Data(data): Data<Value>| {
                let state_c = state_c.clone();
                async move {
                    let Some(studio_id) = data.get("studioId").and_then(|v| v.as_i64()) else { return; };
                    let ad_id = data.get("adId").and_then(|v| v.as_i64());
                    let image_path = data.get("imagePath").and_then(|v| v.as_str()).map(String::from);
                    let duration = data.get("duration").and_then(|v| v.as_i64()).unwrap_or(10);
                    {
                        let mut states = state_c.studio_states.lock().await;
                        let s = states.entry(studio_id).or_default();
                        s.active_ad_id = ad_id;
                    }
                    let room = format!("studio:{studio_id}");
                    let payload = serde_json::json!({
                        "studioId": studio_id,
                        "adId": ad_id,
                        "imagePath": image_path,
                        "duration": duration,
                    });
                    let _ = socket.within(room).emit("ad-started", &payload).await;
                }
            });
        }

        // trigger-obs-command
        {
            socket.on("trigger-obs-command", move |socket: SocketRef, Data(data): Data<Value>| async move {
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
                let _ = socket.within(room).emit("obs-command-fired", &payload).await;
            });
        }
    }});
}
