mod agents;
mod project;
mod toolchain;

use std::path::PathBuf;
use std::sync::Mutex;

use agents::DetectedAgent;
use serde::Serialize;
use tauri::{AppHandle, Manager, RunEvent};
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
fn detect_agents() -> Vec<DetectedAgent> {
    agents::detect()
}

#[tauri::command]
fn get_project_folder(app: AppHandle) -> Option<String> {
    project::resolve_project_folder(&app).map(|dir| dir.to_string_lossy().into_owned())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChosenFolder {
    path: String,
    is_project: bool,
}

#[tauri::command]
async fn choose_project_folder(app: AppHandle) -> Result<Option<ChosenFolder>, String> {
    let dialog = app
        .dialog()
        .file()
        .set_title("Choose where your slides live");
    let picked = tauri::async_runtime::spawn_blocking(move || dialog.blocking_pick_folder())
        .await
        .map_err(|err| err.to_string())?;
    let Some(picked) = picked else {
        return Ok(None);
    };
    let dir: PathBuf = picked
        .into_path()
        .map_err(|err| format!("Unsupported folder selection: {err}"))?;
    project::store_project_folder(&app, &dir)?;
    Ok(Some(ChosenFolder {
        is_project: project::is_project(&dir),
        path: dir.to_string_lossy().into_owned(),
    }))
}

#[tauri::command]
async fn init_project(app: AppHandle) -> Result<(), String> {
    let dir = project::resolve_project_folder(&app)
        .ok_or_else(|| "No project folder selected yet.".to_string())?;
    tauri::async_runtime::spawn_blocking(move || project::init_project(&app, &dir))
        .await
        .map_err(|err| err.to_string())?
}

#[tauri::command]
async fn start_dev_server(app: AppHandle) -> Result<u16, String> {
    let dir = project::resolve_project_folder(&app)
        .ok_or_else(|| "No project folder selected yet.".to_string())?;
    tauri::async_runtime::spawn_blocking(move || project::start_dev_server(&app, &dir))
        .await
        .map_err(|err| err.to_string())?
}

#[tauri::command]
fn launch_agent(app: AppHandle, agent_id: String) -> Result<(), String> {
    let agent = agents::find_agent(&agent_id)
        .ok_or_else(|| format!("Agent `{agent_id}` is no longer available on this machine."))?;
    let folder = project::resolve_project_folder(&app)
        .ok_or_else(|| "Choose a project folder before launching an agent.".to_string())?;
    agents::launch_in_terminal(std::path::Path::new(&agent.path), &folder)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(project::DevServer(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            detect_agents,
            get_project_folder,
            choose_project_folder,
            init_project,
            start_dev_server,
            launch_agent
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let RunEvent::Exit = event {
            project::stop_dev_server(&app_handle.state::<project::DevServer>());
        }
    });
}
