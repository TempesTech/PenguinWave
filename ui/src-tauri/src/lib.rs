//! Penguin Wave desktop UI.
//!
//! Holds no audio logic. Every request is forwarded to `penguinwave-daemon`
//! over its socket; this layer owns the window, the tray, and the connection.

mod daemon;

use daemon::{DaemonClient, EventRelay, CONNECTION_EVENT};
use penguinwave_proto::{EventFrame, Request, Response};
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
#[cfg(not(target_os = "linux"))]
use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WindowEvent};

/// The one command the webview calls. Everything else is the daemon's.
#[tauri::command]
async fn daemon_request(
    client: tauri::State<'_, Arc<DaemonClient>>,
    request: Request,
) -> Result<Response, penguinwave_proto::PwError> {
    let client = Arc::clone(&client);
    // The socket call blocks; keeping it off the async runtime's thread stops
    // a slow daemon from stalling every other command.
    tauri::async_runtime::spawn_blocking(move || client.call(request))
        .await
        .map_err(|e| {
            penguinwave_proto::PwError::new(penguinwave_proto::ErrorKind::Internal, e.to_string())
        })?
}

/// Whether the socket is up, for the disconnected banner.
#[tauri::command]
fn daemon_connected(client: tauri::State<'_, Arc<DaemonClient>>) -> bool {
    client.is_connected()
}

/// Relays daemon pushes to the webview under their protocol names.
struct WebviewRelay(AppHandle);

impl EventRelay for WebviewRelay {
    fn event(&self, frame: &EventFrame) {
        let _ = self.0.emit(frame.event.name(), &frame.event);
    }

    fn connection(&self, connected: bool) {
        let _ = self.0.emit(CONNECTION_EVENT, connected);
    }
}

// Reveal + focus the main window (used by tray click and single-instance).
fn show_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    match app.get_webview_window("main") {
        Some(window) => {
            let visible = window.is_visible().unwrap_or(false);
            let minimized = window.is_minimized().unwrap_or(false);
            eprintln!(
                "[tray] show_main_window: found window (visible={visible}, minimized={minimized})"
            );
            let r1 = window.unminimize();
            let r2 = window.show();
            let r3 = window.set_focus();
            eprintln!("[tray] unminimize={r1:?} show={r2:?} set_focus={r3:?}");
        }
        None => eprintln!("[tray] show_main_window: NO window with label 'main'"),
    }
}

// Build the system tray icon + menu.
fn build_tray<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "Show PenguinWave", true, None::<&str>)?;
    let hide_item = MenuItem::with_id(app, "hide", "Hide to tray", true, None::<&str>)?;
    // Quitting closes the UI only: the daemon keeps ChatMix and the sinks
    // running, so the old bare "Quit" now claims more than it does.
    let quit_item = MenuItem::with_id(
        app,
        "quit",
        "Quit (audio keeps running)",
        true,
        None::<&str>,
    )?;
    let menu = Menu::with_items(app, &[&show_item, &hide_item, &quit_item])?;

    // Load the tray icon explicitly from the bundled PNG instead of relying on
    // the compile-time-embedded default window icon (which cargo does not
    // re-embed when only the icon files change).
    let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/128x128.png"))?;

    // NOTE: Linux tray (libappindicator / StatusNotifier) does NOT deliver
    // `on_tray_icon_event` clicks — the menu is the only interaction channel,
    // so the menu must stay enabled on left click there. On Windows/macOS the
    // click event fires, so we additionally open the window on left click.
    let builder = TrayIconBuilder::with_id("main")
        .icon(tray_icon)
        .tooltip("PenguinWave")
        .menu(&menu)
        .on_menu_event(|app, event| {
            eprintln!("[tray] menu event id={:?}", event.id.as_ref());
            match event.id.as_ref() {
                "show" => show_main_window(app),
                "hide" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.hide();
                    }
                }
                "quit" => app.exit(0),
                _ => {}
            }
        });

    // Suppress the menu on left click only where direct click events work,
    // so a single left click reveals the window instead of opening the menu.
    #[cfg(not(target_os = "linux"))]
    let builder = builder
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    builder.build(app)?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // On Wayland, webkitgtk's DMABUF renderer can fail to re-realize the GL
    // surface after a hide()/show() cycle (close-to-tray), leaving the window
    // invisible even though show() returns Ok. Must be set before GTK
    // initializes.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }

    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init());
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }));
        builder = builder.plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ));
    }

    builder
        .setup(|app| {
            build_tray(app.handle())?;

            let client = DaemonClient::new();
            client.spawn(WebviewRelay(app.handle().clone()));
            app.manage(client);
            Ok(())
        })
        .on_window_event(|window, event| {
            // Close-to-tray. ChatMix keeps running either way now: it belongs
            // to the daemon, not to this window.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![daemon_request, daemon_connected])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
