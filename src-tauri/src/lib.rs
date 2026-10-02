mod auth;
mod commands;
mod config;
mod error;
mod library;
mod model;
mod presence;
mod rules;
mod store;
mod youtube;

use commands::AppState;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};
use tokio::sync::Mutex;

fn show(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show(app)))
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            if app.config().plugins.0.contains_key("updater") {
                app.handle()
                    .plugin(tauri_plugin_updater::Builder::new().build())?;
            }
            let directory = app.path().app_data_dir()?;
            #[cfg(debug_assertions)]
            let directory = std::env::var_os("RESONANCE_TEST_DATA_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or(directory);
            std::fs::create_dir_all(&directory)?;
            let store = store::Store::open(&directory.join("library.sqlite"))?;
            let mut library = store.load()?;
            let config = config::Config::from_env();
            config.apply_defaults(&mut library.settings);
            let close_to_tray = std::sync::atomic::AtomicBool::new(library.settings.close_to_tray);
            app.manage(AppState {
                config,
                core: Mutex::new(library::Core {
                    library,
                    store,
                    sync_error: None,
                }),
                discord: Mutex::new(presence::DiscordClient::default()),
                signing_in: Mutex::new(()),
                playback: Mutex::new(None),
                close_to_tray,
            });
            let show_item = MenuItem::with_id(app, "show", "Show Resonance", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().expect("application icon").clone())
                .tooltip("Resonance")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show(app),
                    "quit" => app.exit(0),
                    _ => (),
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    ) {
                        show(tray.app_handle());
                    }
                })
                .build(app)?;
            commands::start_background_sync(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                if state
                    .close_to_tray
                    .load(std::sync::atomic::Ordering::Relaxed)
                {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::save_settings,
            commands::sign_in,
            commands::disconnect,
            commands::create_playlist,
            commands::delete_playlist,
            commands::add_track,
            commands::remove_track,
            commands::save_rule,
            commands::delete_rule,
            commands::preview_rules,
            commands::sync_library,
            commands::playback_tick,
            commands::clear_history,
            commands::open_youtube,
        ])
        .run(tauri::generate_context!())
        .expect("Couldn't start Resonance");
}
