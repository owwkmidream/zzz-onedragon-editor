use std::path::Path;

use serde::Deserialize;

use crate::error::{AppError, AppResult};

#[derive(Debug, Deserialize, Clone)]
pub struct TeamEntry {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub auto_battle: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct TeamYaml {
    #[serde(default)]
    team_list: Vec<TeamEntry>,
}

#[derive(Debug, Clone)]
pub struct TeamInfo {
    pub idx: i32,
    pub name: String,
    pub auto_battle: String,
}

#[derive(Debug, Clone)]
pub struct TeamConfig {
    pub team_list: Vec<TeamInfo>,
}

pub fn load_team_config(project_root: &Path, instance_idx: u32) -> AppResult<TeamConfig> {
    let path = project_root
        .join("config")
        .join(format!("{instance_idx:02}"))
        .join("team.yml");

    let text = if path.exists() {
        std::fs::read_to_string(&path)
            .map_err(|e| AppError::read_file_failed(path.display().to_string(), e))?
    } else {
        String::new()
    };

    let yaml: TeamYaml = if text.trim().is_empty() {
        TeamYaml { team_list: vec![] }
    } else {
        serde_yaml::from_str(&text)
            .map_err(|e| AppError::parse_yaml_failed(path.display().to_string(), e))?
    };

    let mut team_list: Vec<TeamInfo> = Vec::new();
    for (i, item) in yaml.team_list.iter().enumerate() {
        team_list.push(TeamInfo {
            idx: i as i32,
            name: item
                .name
                .clone()
                .unwrap_or_else(|| format!("编队{}", i + 1)),
            auto_battle: item.auto_battle.clone().unwrap_or_else(|| "全配队通用".into()),
        });
    }

    let max_cnt = 10;
    while team_list.len() < max_cnt {
        let i = team_list.len();
        team_list.push(TeamInfo {
            idx: i as i32,
            name: format!("编队{}", i + 1),
            auto_battle: "全配队通用".into(),
        });
    }

    Ok(TeamConfig { team_list })
}
