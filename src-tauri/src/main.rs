#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod gateway_proxy;
mod models;
mod paths;
mod policy;
mod runtime;
mod self_edit;
mod storage;

use gateway_proxy::GatewayProxy;
use models::{BootstrapStatus, ChangeRecord, InspectSelection, PapersSession, PolicyDecision};
use paths::PapersPaths;
use runtime::RuntimeManager;
use self_edit::SelfEditService;
use serde_json::Value;
use std::sync::Mutex;
use storage::Database;
use tauri::{AppHandle, Manager, RunEvent, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

struct AppState {
    database: Database,
    runtime: RuntimeManager,
    self_edit: SelfEditService,
    gateway: GatewayProxy,
    last_foreground: Mutex<String>,
}

const PAPERS_GLOBAL_SHORTCUT: &str = "Ctrl+Alt+Q";

#[tauri::command]
async fn bootstrap_status(state: State<'_, AppState>) -> Result<BootstrapStatus, String> {
    Ok(state.runtime.status().await)
}

#[tauri::command]
async fn install_hermes(state: State<'_, AppState>) -> Result<BootstrapStatus, String> {
    state.runtime.install().await
}

#[tauri::command]
async fn start_hermes(state: State<'_, AppState>) -> Result<BootstrapStatus, String> {
    state.runtime.start().await
}

#[tauri::command]
fn stop_hermes(state: State<'_, AppState>) -> Result<(), String> {
    state.runtime.stop()
}

#[tauri::command]
async fn start_nous_login(state: State<'_, AppState>) -> Result<String, String> {
    state.runtime.start_nous_login().await
}

#[tauri::command]
async fn gateway_connect(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let url = state.runtime.gateway_url()?;
    state.gateway.connect(app, url).await
}

#[tauri::command]
fn gateway_send(state: State<'_, AppState>, frame: String) -> Result<(), String> {
    state.gateway.send(frame)
}

#[tauri::command]
fn gateway_disconnect(state: State<'_, AppState>) -> Result<(), String> {
    state.gateway.disconnect()
}

#[tauri::command]
fn show_companion(app: tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("companion")
        .ok_or_else(|| "The Papers companion window is unavailable.".to_string())?;
    position_companion(&window);
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())
}

#[tauri::command]
fn hide_companion(app: tauri::AppHandle) -> Result<(), String> {
    app.get_webview_window("companion")
        .ok_or_else(|| "The Papers companion window is unavailable.".to_string())?
        .hide()
        .map_err(|error| error.to_string())
}

fn toggle_companion(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("companion") else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        return;
    }
    position_companion(&window);
    let _ = window.show();
    let _ = window.set_focus();
}

#[tauri::command]
fn show_main(app: tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "The main Papers window is unavailable.".to_string())?;
    window.show().map_err(|error| error.to_string())?;
    window.unminimize().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())
}

#[tauri::command]
fn foreground_app(state: State<'_, AppState>) -> String {
    let current = foreground_window_title().unwrap_or_else(|| "your current app".to_string());
    let papers_window = current == "Papers Agent"
        || current == "PAPERS ARE PAPERS"
        || current == "PAPERS PREVIEW — TEMPORARY VERSION";
    if let Ok(mut remembered) = state.last_foreground.lock() {
        if papers_window && !remembered.is_empty() {
            return remembered.clone();
        }
        if !papers_window {
            *remembered = current.clone();
        }
    }
    current
}

#[tauri::command]
fn list_sessions(state: State<'_, AppState>) -> Result<Vec<PapersSession>, String> {
    state.database.list_sessions()
}

#[tauri::command]
fn create_session(
    state: State<'_, AppState>,
    title: String,
    mode: String,
) -> Result<PapersSession, String> {
    state.database.create_session(&title, &mode)
}

#[tauri::command]
fn rename_session(
    state: State<'_, AppState>,
    id: String,
    title: String,
) -> Result<PapersSession, String> {
    state.database.rename_session(&id, &title)
}

#[tauri::command]
fn delete_session(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.database.delete_session(&id)
}

#[tauri::command]
fn bind_hermes_session(
    state: State<'_, AppState>,
    id: String,
    hermes_session_id: String,
) -> Result<(), String> {
    state.database.bind_hermes_session(&id, &hermes_session_id)
}

#[tauri::command]
fn update_session_state(
    state: State<'_, AppState>,
    id: String,
    state_name: String,
) -> Result<(), String> {
    state.database.update_session_state(&id, &state_name)
}

#[tauri::command]
fn record_agent_event(
    state: State<'_, AppState>,
    session_id: String,
    event: Value,
) -> Result<(), String> {
    state.database.record_event(&session_id, &event)
}

#[tauri::command]
fn classify_action(kind: String, target: String, payload: String) -> PolicyDecision {
    policy::classify(&kind, &target, &payload)
}

#[tauri::command]
fn create_change(
    state: State<'_, AppState>,
    title: String,
    request: String,
    selection: Option<InspectSelection>,
) -> Result<ChangeRecord, String> {
    state.self_edit.create(&title, &request, selection)
}

#[tauri::command]
fn list_changes(state: State<'_, AppState>) -> Result<Vec<ChangeRecord>, String> {
    state.self_edit.list()
}

#[tauri::command]
async fn build_change(state: State<'_, AppState>, id: String) -> Result<ChangeRecord, String> {
    let service = state.self_edit.clone();
    tokio::task::spawn_blocking(move || service.build(&id))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
fn launch_change_preview(state: State<'_, AppState>, id: String) -> Result<ChangeRecord, String> {
    state.self_edit.launch_preview(&id)
}

#[tauri::command]
async fn accept_change(state: State<'_, AppState>, id: String) -> Result<ChangeRecord, String> {
    let service = state.self_edit.clone();
    tokio::task::spawn_blocking(move || service.accept(&id))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
fn reject_change(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.self_edit.reject(&id)
}

#[tauri::command]
async fn rollback_last(state: State<'_, AppState>) -> Result<String, String> {
    let service = state.self_edit.clone();
    tokio::task::spawn_blocking(move || service.rollback_last())
        .await
        .map_err(|error| error.to_string())?
}

fn main() {
    let paths = PapersPaths::discover().expect("Papers could not prepare its local data directory");
    let database =
        Database::open(&paths.database()).expect("Papers could not open its local state");
    let runtime =
        RuntimeManager::new(paths.clone()).expect("Papers could not load the Hermes lock");
    let self_edit = SelfEditService::new(paths, database.clone());
    let state = AppState {
        database,
        runtime,
        self_edit,
        gateway: GatewayProxy::default(),
        last_foreground: Mutex::new(String::new()),
    };

    let app = tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    if let Some(target) = foreground_window_title() {
                        if target != "Papers Agent" {
                            if let Ok(mut remembered) =
                                app.state::<AppState>().last_foreground.lock()
                            {
                                *remembered = target;
                            }
                        }
                    }
                    toggle_companion(app);
                })
                .build(),
        )
        .manage(state)
        .setup(|app| {
            if let Err(error) = app.global_shortcut().register(PAPERS_GLOBAL_SHORTCUT) {
                eprintln!(
                    "Papers could not register {PAPERS_GLOBAL_SHORTCUT} because another app already owns it: {error}"
                );
            }

            if let Ok(path) = std::env::var("PAPERS_HEALTH_FILE") {
                std::fs::write(path, b"ready")?;
            }
            if std::env::var_os("PAPERS_PREVIEW").is_some() {
                if let Some(window) = app.get_webview_window("main") {
                    window.set_title("PAPERS PREVIEW — TEMPORARY VERSION")?;
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap_status,
            install_hermes,
            start_hermes,
            stop_hermes,
            start_nous_login,
            gateway_connect,
            gateway_send,
            gateway_disconnect,
            show_companion,
            hide_companion,
            show_main,
            foreground_app,
            list_sessions,
            create_session,
            rename_session,
            delete_session,
            bind_hermes_session,
            update_session_state,
            record_agent_event,
            classify_action,
            create_change,
            list_changes,
            build_change,
            launch_change_preview,
            accept_change,
            reject_change,
            rollback_last
        ])
        .build(tauri::generate_context!())
        .expect("Papers could not start");

    app.run(|app_handle, event| {
        if matches!(event, RunEvent::Exit) {
            let _ = app_handle.state::<AppState>().gateway.disconnect();
            let _ = app_handle.state::<AppState>().runtime.stop();
        }
    });
}

fn position_companion(window: &tauri::WebviewWindow) {
    if let Ok(Some(monitor)) = window.current_monitor() {
        let size = monitor.size();
        let scale = monitor.scale_factor();
        let window_size = window
            .outer_size()
            .unwrap_or(tauri::PhysicalSize::new(430, 112));
        let margin = (18.0 * scale) as i32;
        let x = size.width as i32 - window_size.width as i32 - margin;
        let y = size.height as i32 - window_size.height as i32 - (56.0 * scale) as i32;
        let _ = window.set_position(tauri::PhysicalPosition::new(x.max(0), y.max(0)));
    }
}

#[cfg(windows)]
fn foreground_window_title() -> Option<String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
    };
    unsafe {
        let window = GetForegroundWindow();
        if window.is_null() {
            return None;
        }
        let length = GetWindowTextLengthW(window);
        if length <= 0 {
            return None;
        }
        let mut buffer = vec![0u16; length as usize + 1];
        let written = GetWindowTextW(window, buffer.as_mut_ptr(), buffer.len() as i32);
        if written <= 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&buffer[..written as usize]))
    }
}

#[cfg(not(windows))]
fn foreground_window_title() -> Option<String> {
    None
}
