//! Focus Desktop spike entry (Tauri 2). Creates four windows (desktop / pet /
//! panel / music), a Rust event bus, a Mock Agent, SQLite storage and a
//! foreground probe. No real agent, no lock, no shell replacement, no private
//! virtual-desktop API (v0.2 plan, hard boundaries).

mod activity;
mod agents;
mod event_bus;
mod storage;

use std::sync::{Arc, Mutex};

use tauri::{Listener, Manager};

use event_bus::CoreEvent;

fn create_windows(app: &mut tauri::App) -> tauri::Result<()> {
    let url = tauri::WebviewUrl::App("index.html".into());

    tauri::WebviewWindowBuilder::new(app, "desktop", url.clone())
        .title("Focus Desktop")
        .fullscreen(true)
        .decorations(false)
        .build()?;

    tauri::WebviewWindowBuilder::new(app, "pet", url.clone())
        .title("Focus Pet")
        .inner_size(200.0, 200.0)
        .position(1280.0, 700.0)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .resizable(false)
        .build()?;

    tauri::WebviewWindowBuilder::new(app, "panel", url.clone())
        .title("Focus Panel")
        .inner_size(440.0, 680.0)
        .position(1040.0, 60.0)
        .resizable(true)
        .build()?;

    tauri::WebviewWindowBuilder::new(app, "music", url.clone())
        .title("Focus Music")
        .inner_size(340.0, 110.0)
        .position(1150.0, 20.0)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .resizable(false)
        .build()?;

    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Core event bus (broadcast). One relay task forwards to all windows.
            let (tx, rx) = tokio::sync::broadcast::channel::<CoreEvent>(256);

            create_windows(app)?;

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(event_bus::relay_task(app_handle, rx));

            // Mock Agent: publishes schema-valid AgentEvent envelopes + pet/bubble events.
            agents::mock::spawn(tx.clone());

            // SQLite storage + foreground probe.
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let store = storage::Store::open(&data_dir.join("spike.db"))?;
            store.migrate()?;
            let store = Arc::new(Mutex::new(store));
            app.manage(store.clone());

            let tx_probe = tx.clone();
            activity::spawn_probe(tx_probe, store);

            // Frontend -> core -> all windows: panel mode change.
            let tx_panel = tx.clone();
            let app_handle2 = app.handle().clone();
            app_handle2.listen("ui:panel_mode_changed", move |event| {
                let mode = serde_json::from_str::<serde_json::Value>(event.payload())
                    .ok()
                    .and_then(|v| {
                        v.get("mode")
                            .and_then(|m| m.as_str())
                            .map(|s| s.to_string())
                    })
                    .unwrap_or_else(|| "closed".to_string());
                let _ = tx_panel.send(CoreEvent::PanelModeChanged { mode });
            });

            // Frontend -> core: pet click toggles the panel window.
            let app_handle3 = app.handle().clone();
            app_handle3.clone().listen("ui:toggle_panel", move |_event| {
                if let Some(panel) = app_handle3.get_webview_window("panel") {
                    let visible = panel.is_visible().unwrap_or(true);
                    if visible {
                        let _ = panel.hide();
                    } else {
                        let _ = panel.show();
                        let _ = panel.set_focus();
                    }
                }
            });

            // Frontend -> core: music playback tick, routed back onto the bus so
            // the full frontend -> core -> bus -> windows path is exercised.
            let tx_music = tx.clone();
            let app_handle4 = app.handle().clone();
            app_handle4.listen("music:playback_tick", move |event| {
                let v = serde_json::from_str::<serde_json::Value>(event.payload()).unwrap_or_default();
                let position_ms = v.get("positionMs").and_then(|x| x.as_u64()).unwrap_or(0);
                let duration_ms = v.get("durationMs").and_then(|x| x.as_u64()).unwrap_or(0);
                let _ = tx_music.send(CoreEvent::MusicTick { position_ms, duration_ms });
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}