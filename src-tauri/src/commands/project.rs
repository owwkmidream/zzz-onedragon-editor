use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct DetectRootResult {
    pub found: bool,
    pub reason: String,
    pub path: Option<String>,
}

fn is_valid_project_root(path: &Path) -> AppResult<()> {
    let one_dragon = path.join("config").join("one_dragon.yml");
    if !one_dragon.exists() {
        return Err(AppError::ProjectRootInvalid("config/one_dragon.yml".into()));
    }

    let compendium = path
        .join("assets")
        .join("game_data")
        .join("compendium_data.yml");
    if !compendium.exists() {
        return Err(AppError::ProjectRootInvalid(
            "assets/game_data/compendium_data.yml".into(),
        ));
    }

    Ok(())
}

#[tauri::command]
pub fn detect_project_root() -> DetectRootResult {
    let cwd = std::env::current_dir().ok();
    let Some(cwd) = cwd else {
        return DetectRootResult {
            found: false,
            reason: "无法获取当前工作目录".into(),
            path: None,
        };
    };

    match is_valid_project_root(&cwd) {
        Ok(()) => DetectRootResult {
            found: true,
            reason: format!("已检测到项目根目录"),
            path: Some(cwd.display().to_string()),
        },
        Err(e) => DetectRootResult {
            found: false,
            reason: e.to_string(),
            path: None,
        },
    }
}

#[tauri::command]
pub fn set_project_root(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let root = PathBuf::from(path);
    is_valid_project_root(&root).map_err(|e| e.to_string())?;
    state.set_project_root(root);
    Ok(())
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct InstanceInfo {
    pub idx: u32,
    pub name: String,
    pub active: Option<bool>,
    pub active_in_od: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct OneDragonConfig {
    instance_list: Vec<InstanceInfo>,
}

#[tauri::command]
pub fn list_instances(state: State<'_, AppState>) -> Result<Vec<InstanceInfo>, String> {
    list_instances_impl(&state).map_err(|e| e.to_string())
}

fn list_instances_impl(state: &AppState) -> AppResult<Vec<InstanceInfo>> {
    let root = state.project_root().ok_or(AppError::ProjectRootNotSet)?;
    let path = root.join("config").join("one_dragon.yml");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| AppError::read_file_failed(path.display().to_string(), e))?;
    let data: OneDragonConfig = serde_yaml::from_str(&text)
        .map_err(|e| AppError::parse_yaml_failed(path.display().to_string(), e))?;
    Ok(data.instance_list)
}
