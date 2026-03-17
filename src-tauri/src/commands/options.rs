use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use tauri::State;

use crate::domain::{compendium, team};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Debug, Serialize, Clone)]
pub struct LabeledValue {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct CompendiumForChargePlan {
    pub categories: Vec<String>,
    pub mission_types_by_category: BTreeMap<String, Vec<LabeledValue>>,
    pub missions_by_category_and_type: BTreeMap<String, BTreeMap<String, Vec<LabeledValue>>>,
}

#[tauri::command]
pub fn load_compendium_for_charge_plan(
    state: State<'_, AppState>,
) -> Result<CompendiumForChargePlan, String> {
    load_compendium_for_charge_plan_impl(&state).map_err(|e| e.to_string())
}

fn load_compendium_for_charge_plan_impl(state: &AppState) -> AppResult<CompendiumForChargePlan> {
    let root = state.project_root().ok_or(AppError::ProjectRootNotSet)?;
    let comp = compendium::load_compendium(&root)?;

    let training = comp.find_tab("训练").ok_or_else(|| {
        AppError::ValidationFailed("compendium_data.yml 中找不到 tab_name=训练".into())
    })?;

    let mut categories = Vec::new();
    let mut mission_types_by_category: BTreeMap<String, Vec<LabeledValue>> = BTreeMap::new();
    let mut missions_by_category_and_type: BTreeMap<String, BTreeMap<String, Vec<LabeledValue>>> =
        BTreeMap::new();

    for category in &training.category_list {
        categories.push(category.category_name.clone());

        let mut types = Vec::new();
        let mut missions_by_type: BTreeMap<String, Vec<LabeledValue>> = BTreeMap::new();

        for mission_type in &category.mission_type_list {
            let label = mission_type
                .mission_type_name_display
                .clone()
                .unwrap_or_else(|| mission_type.mission_type_name.clone());
            types.push(LabeledValue {
                label,
                value: mission_type.mission_type_name.clone(),
            });

            let mission_list = mission_type
                .mission_list
                .iter()
                .map(|m| LabeledValue {
                    label: m
                        .mission_name_display
                        .clone()
                        .unwrap_or_else(|| m.mission_name.clone()),
                    value: m.mission_name.clone(),
                })
                .collect::<Vec<_>>();

            missions_by_type.insert(mission_type.mission_type_name.clone(), mission_list);
        }

        mission_types_by_category.insert(category.category_name.clone(), types);
        missions_by_category_and_type.insert(category.category_name.clone(), missions_by_type);
    }

    Ok(CompendiumForChargePlan {
        categories,
        mission_types_by_category,
        missions_by_category_and_type,
    })
}

#[derive(Debug, Serialize)]
pub struct TeamInfo {
    pub idx: i32,
    pub name: String,
    pub auto_battle: String,
}

#[tauri::command]
pub fn load_team_list(state: State<'_, AppState>, instance_idx: u32) -> Result<Vec<TeamInfo>, String> {
    load_team_list_impl(&state, instance_idx).map_err(|e| e.to_string())
}

fn load_team_list_impl(state: &AppState, instance_idx: u32) -> AppResult<Vec<TeamInfo>> {
    let root = state.project_root().ok_or(AppError::ProjectRootNotSet)?;
    let data = team::load_team_config(&root, instance_idx)?;
    Ok(data
        .team_list
        .into_iter()
        .map(|t| TeamInfo {
            idx: t.idx,
            name: t.name,
            auto_battle: t.auto_battle,
        })
        .collect())
}

#[tauri::command]
pub fn list_auto_battle_templates(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    list_auto_battle_templates_impl(&state).map_err(|e| e.to_string())
}

fn list_auto_battle_templates_impl(state: &AppState) -> AppResult<Vec<String>> {
    let root = state.project_root().ok_or(AppError::ProjectRootNotSet)?;
    let dir = root.join("config").join("auto_battle");
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut set: BTreeSet<String> = BTreeSet::new();
    for entry in std::fs::read_dir(&dir)
        .map_err(|e| AppError::read_file_failed(dir.display().to_string(), e))?
    {
        let entry = entry.map_err(|e| AppError::read_file_failed(dir.display().to_string(), e))?;
        let Some(name) = entry.file_name().to_str().map(|s| s.to_string()) else {
            continue;
        };

        let template = if name.ends_with(".sample.yml") {
            name.trim_end_matches(".sample.yml").to_string()
        } else if name.ends_with(".merged.yml") {
            name.trim_end_matches(".merged.yml").to_string()
        } else if name.ends_with(".yml") {
            name.trim_end_matches(".yml").to_string()
        } else {
            continue;
        };

        set.insert(template);
    }

    let mut result = set.into_iter().collect::<Vec<_>>();
    result.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    Ok(result)
}
