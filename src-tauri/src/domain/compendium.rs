use std::path::Path;

use serde::Deserialize;

use crate::error::{AppError, AppResult};

#[derive(Debug, Deserialize, Clone)]
pub struct CompendiumTab {
    pub tab_name: String,
    #[serde(default)]
    pub category_list: Vec<CompendiumCategory>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CompendiumCategory {
    pub category_name: String,
    #[serde(default)]
    pub mission_type_list: Vec<CompendiumMissionType>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CompendiumMissionType {
    pub mission_type_name: String,
    #[serde(default)]
    pub mission_type_name_display: Option<String>,
    #[serde(default)]
    pub mission_list: Vec<CompendiumMission>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CompendiumMission {
    pub mission_name: String,
    #[serde(default)]
    pub mission_name_display: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CompendiumData {
    #[serde(default)]
    pub tab_list: Vec<CompendiumTab>,
}

impl CompendiumData {
    pub fn find_tab(&self, tab_name: &str) -> Option<&CompendiumTab> {
        self.tab_list.iter().find(|t| t.tab_name == tab_name)
    }
}

pub fn load_compendium(project_root: &Path) -> AppResult<CompendiumData> {
    let path = project_root
        .join("assets")
        .join("game_data")
        .join("compendium_data.yml");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| AppError::read_file_failed(path.display().to_string(), e))?;

    let tab_list: Vec<CompendiumTab> = serde_yaml::from_str(&text)
        .map_err(|e| AppError::parse_yaml_failed(path.display().to_string(), e))?;

    Ok(CompendiumData { tab_list })
}
