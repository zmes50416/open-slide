mod agents;

use agents::DetectedAgent;

#[tauri::command]
fn detect_agents() -> Vec<DetectedAgent> {
    agents::detect()
}

#[tauri::command]
fn get_project_folder() -> Result<String, String> {
    agents::project_folder().map(|dir| dir.to_string_lossy().into_owned())
}

#[tauri::command]
fn launch_agent(agent_id: String) -> Result<(), String> {
    let agent = agents::find_agent(&agent_id)
        .ok_or_else(|| format!("Agent `{agent_id}` is no longer available on this machine."))?;
    let folder = agents::project_folder()?;
    agents::launch_in_terminal(std::path::Path::new(&agent.path), &folder)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            detect_agents,
            get_project_folder,
            launch_agent
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
