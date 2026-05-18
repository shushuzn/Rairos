use std::fs;
use std::path::PathBuf;

use crate::state::OrchestratorState;

pub fn get_state_path() -> PathBuf {
    let path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_research_os")
        .join("autonomous");
    fs::create_dir_all(&path).ok();
    path.join("orchestrator_state.json")
}

pub fn load_state() -> OrchestratorState {
    let path = get_state_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(state) = serde_json::from_str(&content) {
                return state;
            }
        }
    }
    OrchestratorState::default()
}

pub fn save_state(state: &OrchestratorState) -> std::io::Result<()> {
    let path = get_state_path();
    let content = serde_json::to_string_pretty(state)?;
    fs::write(path, content)
}