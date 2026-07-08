#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod gateway_proxy;
mod hermes_provider_adapter;
mod models;
mod paths;
mod policy;
mod provider_catalog;
mod provider_runtime;
mod provider_service;
mod provider_state;
mod runtime;
mod self_edit;
mod storage;

use gateway_proxy::GatewayProxy;
use hermes_provider_adapter::HermesProviderAdapter;
use models::{
    AgentProviderStatus, BootstrapStatus, ChangeRecord, InspectSelection, PapersSession,
    PolicyDecision,
};
use paths::PapersPaths;
use provider_catalog::ProviderCatalogEntry;
use provider_service::ProviderService;
use provider_state::{OfflineProviderHint, ProviderState, RuntimeTestResult};
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
    providers: ProviderService,
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
async fn agent_provider_status(
    state: State<'_, AppState>,
) -> Result<AgentProviderStatus, String> {
    Ok(state.runtime.provider_status().await)
}

#[tauri::command]
async fn set_agent_provider(
    state: State<'_, AppState>,
    provider: String,
    model: String,
) -> Result<AgentProviderStatus, String> {
    state.runtime.set_provider_model(provider, model).await
}

#[tauri::command]
async fn start_provider_login(
    state: State<'_, AppState>,
    provider: String,
) -> Result<String, String> {
    state.runtime.start_provider_login(provider).await
}

#[tauri::command]
async fn validate_agent_provider(
    state: State<'_, AppState>,
) -> Result<AgentProviderStatus, String> {
    Ok(state.runtime.validate_provider().await)
}

// --- Provider orchestration layer (new) -------------------------------------
// These commands speak Papers concepts only; Hermes quirks live in the adapter.

#[tauri::command]
fn list_providers(state: State<'_, AppState>) -> Vec<ProviderCatalogEntry> {
    state.providers.list_providers()
}

#[tauri::command]
async fn get_provider_state(
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<ProviderState, String> {
    let (paths, lock, client, port, token) = state.runtime.provider_adapter_inputs();
    let adapter = HermesProviderAdapter::new(paths, lock, client, port, token);
    state.providers.state(&adapter, &provider_id).await
}

#[tauri::command]
async fn offline_provider_hint(
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<OfflineProviderHint, String> {
    let (paths, lock, client, port, token) = state.runtime.provider_adapter_inputs();
    let adapter = HermesProviderAdapter::new(paths, lock, client, port, token);
    state.providers.offline_hint(&adapter, &provider_id)
}

#[tauri::command]
async fn begin_provider_auth(
    app: AppHandle,
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<Value, String> {
    let (paths, lock, client, port, token) = state.runtime.provider_adapter_inputs();
    let adapter = HermesProviderAdapter::new(paths, lock, client, port, token);
    let event = state.providers.begin_auth(&app, &adapter, &provider_id).await?;
    Ok(serde_json::to_value(&event).map_err(|error| error.to_string())?)
}

#[tauri::command]
async fn poll_provider_auth(
    app: AppHandle,
    state: State<'_, AppState>,
    provider_id: String,
    session_id: String,
) -> Result<Value, String> {
    let (paths, lock, client, port, token) = state.runtime.provider_adapter_inputs();
    let adapter = HermesProviderAdapter::new(paths, lock, client, port, token);
    let event = state
        .providers
        .poll_auth(&app, &adapter, &provider_id, &session_id)
        .await?;
    Ok(serde_json::to_value(&event).map_err(|error| error.to_string())?)
}

#[tauri::command]
async fn submit_provider_auth_code(
    app: AppHandle,
    state: State<'_, AppState>,
    provider_id: String,
    session_id: String,
    code: String,
) -> Result<Value, String> {
    let (paths, lock, client, port, token) = state.runtime.provider_adapter_inputs();
    let adapter = HermesProviderAdapter::new(paths, lock, client, port, token);
    let event = state
        .providers
        .submit_auth_code(&app, &adapter, &provider_id, &session_id, &code)
        .await?;
    Ok(serde_json::to_value(&event).map_err(|error| error.to_string())?)
}

#[tauri::command]
async fn save_provider_secret(
    app: AppHandle,
    state: State<'_, AppState>,
    provider_id: String,
    secret: String,
) -> Result<Value, String> {
    let (paths, lock, client, port, token) = state.runtime.provider_adapter_inputs();
    let adapter = HermesProviderAdapter::new(paths, lock, client, port, token);
    let result = state
        .providers
        .save_secret(&app, &adapter, &provider_id, &secret)
        .await?;
    Ok(serde_json::to_value(&result).map_err(|error| error.to_string())?)
}

#[tauri::command]
async fn list_provider_models(
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<Vec<String>, String> {
    let (paths, lock, client, port, token) = state.runtime.provider_adapter_inputs();
    let adapter = HermesProviderAdapter::new(paths, lock, client, port, token);
    state.providers.list_models(&adapter, &provider_id).await
}

#[tauri::command]
async fn set_provider_model_v2(
    app: AppHandle,
    state: State<'_, AppState>,
    provider_id: String,
    model: String,
) -> Result<Value, String> {
    let (paths, lock, client, port, token) = state.runtime.provider_adapter_inputs();
    let adapter = HermesProviderAdapter::new(paths, lock, client, port, token);
    let event = state
        .providers
        .set_model(&app, &adapter, &provider_id, &model)
        .await?;
    Ok(serde_json::to_value(&event).map_err(|error| error.to_string())?)
}

#[tauri::command]
async fn disconnect_provider(
    app: AppHandle,
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<(), String> {
    let (paths, lock, client, port, token) = state.runtime.provider_adapter_inputs();
    let adapter = HermesProviderAdapter::new(paths, lock, client, port, token);
    state
        .providers
        .disconnect(&app, &adapter, &provider_id)
        .await
}

#[tauri::command]
async fn set_active_provider(
    app: AppHandle,
    state: State<'_, AppState>,
    provider_id: String,
    model: String,
    force: bool,
) -> Result<Value, String> {
    let (paths, lock, client, port, token) = state.runtime.provider_adapter_inputs();
    let adapter = HermesProviderAdapter::new(paths, lock, client, port, token);
    let last_test = state
        .database
        .last_provider_test(&provider_id)
        .ok()
        .flatten()
        .map(|(passed, reason, at)| RuntimeTestResult {
            passed,
            marker: provider_runtime::TEST_MARKER.to_string(),
            reason,
            at,
        });
    let event = state
        .providers
        .activate(&app, &adapter, &provider_id, &model, force, last_test.as_ref())
        .await?;
    Ok(serde_json::to_value(&event).map_err(|error| error.to_string())?)
}

#[tauri::command]
fn record_provider_test_result(
    state: State<'_, AppState>,
    provider_id: String,
    model: String,
    echo: Option<String>,
    error: Option<String>,
) -> RuntimeTestResult {
    let result = state
        .providers
        .record_runtime_test(&provider_id, &model, echo.as_deref(), error.as_deref());
    let _ = state.database.upsert_provider_test(
        &provider_id,
        &model,
        result.passed,
        result.reason.as_deref(),
    );
    result
}

#[tauri::command]
async fn open_api_key_window(app: AppHandle, provider_id: String) -> Result<(), String> {
    open_credential_window(&app, provider_id, ApiKeyPurpose::Entry)
}

/// Latest-known runtime-test truth, not a live health layer. Returns the
/// persisted test outcome for the given provider. Honest: this is NOT a live
/// provider-health subsystem — it is a replay of what the last test recorded.
#[tauri::command]
fn latest_runtime_test_state(
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<Option<RuntimeTestResult>, String> {
    state
        .database
        .last_provider_test(&provider_id)
        .map(|opt| {
            opt.map(|(passed, reason, at)| RuntimeTestResult {
                passed,
                marker: provider_runtime::TEST_MARKER.to_string(),
                reason,
                at,
            })
        })
        .map_err(|error| error.to_string())
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
    let providers = ProviderService::new(database.clone());
    let state = AppState {
        database,
        runtime,
        self_edit,
        gateway: GatewayProxy::default(),
        providers,
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
            agent_provider_status,
            set_agent_provider,
            start_provider_login,
            validate_agent_provider,
            list_providers,
            get_provider_state,
            offline_provider_hint,
            begin_provider_auth,
            poll_provider_auth,
            submit_provider_auth_code,
            save_provider_secret,
            list_provider_models,
            set_provider_model_v2,
            disconnect_provider,
            set_active_provider,
            record_provider_test_result,
            open_api_key_window,
            latest_runtime_test_state,
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

    // Resume any interrupted OAuth sign-ins on relaunch. Hermes is the
    // source of truth; the persisted record only hints that a poll is
    // worth restarting. Best-effort: never blocks app startup.
    let handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        let state = handle.state::<AppState>();
        let (paths, lock, client, port, token) = state.runtime.provider_adapter_inputs();
        let adapter = HermesProviderAdapter::new(paths, lock, client, port, token);
        let _ = state.providers.resume_pending(&handle, &adapter).await;
    });

    app.run(|app_handle, event| {
        if matches!(event, RunEvent::Exit) {
            let _ = app_handle.state::<AppState>().gateway.disconnect();
            let _ = app_handle.state::<AppState>().runtime.stop();
        }
    });
}

/// Purpose of the isolated credential window, so the frontend route knows
/// which minimal form to render. Kept an enum (not a bool) so a future
/// "reveal/confirm" flow slots in without changing the command signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApiKeyPurpose {
    Entry,
}

/// Opens a small isolated window for entering a provider API key. The window
/// loads a dedicated Vite entry (`index-key-entry.html`) so the main app's
/// React state never sees the typed secret. The window posts the key straight
/// to `save_provider_secret` (main app emit) and self-closes on success.
fn open_credential_window(
    app: &AppHandle,
    provider_id: String,
    purpose: ApiKeyPurpose,
) -> Result<(), String> {
    let label = format!("key-entry/{provider_id}");
    if app.get_webview_window(&label).is_some() {
        return Err("A key entry window is already open. Finish or cancel it first.".into());
    }
    let url = format!(
        "index-key-entry.html?provider={provider_id}&purpose={}",
        match purpose {
            ApiKeyPurpose::Entry => "entry",
        }
    );
    tauri::WebviewWindowBuilder::new(app, &label, tauri::WebviewUrl::App(url.into()))
        .title("Enter provider key — Papers")
        .inner_size(460.0, 280.0)
        .resizable(false)
        .center()
        .build()
        .map_err(|error| format!("Could not open the key entry window: {error}"))?;
    Ok(())
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
