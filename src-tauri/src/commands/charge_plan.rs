use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::domain::{charge_plan, compendium};
use crate::domain::charge_plan::{ChargePlanConfigModel, ValidationResult};
use crate::error::{AppError, AppResult};
use crate::infra::fsx;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct ChargePlanPaths {
    pub main_path: String,
    pub legacy_path: String,
    pub main_exists: bool,
    pub legacy_exists: bool,
}

#[derive(Debug, Serialize)]
pub struct ReadChargePlanResult {
    pub source: String,
    pub paths: ChargePlanPaths,
    pub config: ChargePlanConfigModel,
    pub validation: ValidationResult,
}

#[derive(Debug, Serialize)]
pub struct SaveResult {
    pub written_path: String,
    pub backup_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveOptions {
    pub update_history_list: bool,
}

impl Default for SaveOptions {
    fn default() -> Self {
        Self {
            update_history_list: true,
        }
    }
}

fn instance_dir_name(instance_idx: u32) -> String {
    format!("{instance_idx:02}")
}

fn build_paths(root: &Path, instance_idx: u32, group_id: &str) -> (PathBuf, PathBuf) {
    let instance = instance_dir_name(instance_idx);
    let main = root
        .join("config")
        .join(instance)
        .join(group_id)
        .join("charge_plan.yml");
    let legacy = root
        .join("config")
        .join(instance_dir_name(instance_idx))
        .join("charge_plan.yml");
    (main, legacy)
}

#[tauri::command]
pub fn get_charge_plan_paths(
    state: State<'_, AppState>,
    instance_idx: u32,
    group_id: Option<String>,
) -> Result<ChargePlanPaths, String> {
    get_charge_plan_paths_impl(&state, instance_idx, group_id.as_deref().unwrap_or("one_dragon"))
        .map_err(|e| e.to_string())
}

fn get_charge_plan_paths_impl(
    state: &AppState,
    instance_idx: u32,
    group_id: &str,
) -> AppResult<ChargePlanPaths> {
    let root = state.project_root().ok_or(AppError::ProjectRootNotSet)?;
    let (main, legacy) = build_paths(&root, instance_idx, group_id);

    Ok(ChargePlanPaths {
        main_path: fsx::to_rel_string(&root, &main),
        legacy_path: fsx::to_rel_string(&root, &legacy),
        main_exists: main.exists(),
        legacy_exists: legacy.exists(),
    })
}

#[tauri::command]
pub fn read_charge_plan(
    state: State<'_, AppState>,
    instance_idx: u32,
    group_id: Option<String>,
) -> Result<ReadChargePlanResult, String> {
    read_charge_plan_impl(&state, instance_idx, group_id.as_deref().unwrap_or("one_dragon"))
        .map_err(|e| e.to_string())
}

fn read_charge_plan_impl(
    state: &AppState,
    instance_idx: u32,
    group_id: &str,
) -> AppResult<ReadChargePlanResult> {
    let root = state.project_root().ok_or(AppError::ProjectRootNotSet)?;
    let (main, legacy) = build_paths(&root, instance_idx, group_id);
    let paths = get_charge_plan_paths_impl(state, instance_idx, group_id)?;

    let (source, config_yaml) = if main.exists() {
        ("main".to_string(), charge_plan::load_charge_plan_yaml(&main)?)
    } else if legacy.exists() {
        ("legacy".to_string(), charge_plan::load_charge_plan_yaml(&legacy)?)
    } else {
        ("none".to_string(), charge_plan::ChargePlanYaml::default())
    };

    let mut extra_warnings: Vec<String> = Vec::new();
    if source == "legacy" {
        extra_warnings.push(
            "当前读取自历史路径（legacy）。建议迁移到 config/{实例}/one_dragon/charge_plan.yml。".into(),
        );
    }
    extra_warnings.push("主程序运行时可能缓存配置；保存后如果未生效，建议重启主程序。".into());
    extra_warnings.push(
        "提示：level 字段即使写入，也可能在主程序保存时被擦除；本工具 v0 不提供 level 编辑入口。".into(),
    );

    let comp = compendium::load_compendium(&root)?;
    let mut validation = charge_plan::validate_config(&comp, &config_yaml)?;
    validation.warnings.extend(extra_warnings);

    let config = charge_plan::to_model(config_yaml, &mut validation.warnings);

    Ok(ReadChargePlanResult {
        source,
        paths,
        config,
        validation,
    })
}

#[tauri::command]
pub fn validate_charge_plan(
    state: State<'_, AppState>,
    _instance_idx: u32,
    config: ChargePlanConfigModel,
) -> Result<ValidationResult, String> {
    validate_charge_plan_impl(&state, config).map_err(|e| e.to_string())
}

fn validate_charge_plan_impl(state: &AppState, config: ChargePlanConfigModel) -> AppResult<ValidationResult> {
    let root = state.project_root().ok_or(AppError::ProjectRootNotSet)?;
    let comp = compendium::load_compendium(&root)?;
    let yaml = charge_plan::from_model(config)?;
    charge_plan::validate_config(&comp, &yaml)
}

#[tauri::command]
pub fn save_charge_plan(
    state: State<'_, AppState>,
    instance_idx: u32,
    group_id: Option<String>,
    config: ChargePlanConfigModel,
    options: Option<SaveOptions>,
) -> Result<SaveResult, String> {
    save_charge_plan_impl(
        &state,
        instance_idx,
        group_id.as_deref().unwrap_or("one_dragon"),
        config,
        options.unwrap_or_default(),
    )
    .map_err(|e| e.to_string())
}

fn save_charge_plan_impl(
    state: &AppState,
    instance_idx: u32,
    group_id: &str,
    config: ChargePlanConfigModel,
    options: SaveOptions,
) -> AppResult<SaveResult> {
    let root = state.project_root().ok_or(AppError::ProjectRootNotSet)?;
    let (main, _legacy) = build_paths(&root, instance_idx, group_id);

    if let Some(parent) = main.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::write_file_failed(parent.display().to_string(), e))?;
    }

    let comp = compendium::load_compendium(&root)?;
    let mut yaml = charge_plan::from_model(config)?;
    let validation = charge_plan::validate_config(&comp, &yaml)?;
    if !validation.errors.is_empty() {
        return Err(AppError::ValidationFailed(validation.errors.join("\n")));
    }

    if options.update_history_list {
        yaml.history_list = charge_plan::build_new_history_list(&yaml.plan_list, &yaml.history_list);
    }

    let backup_path = fsx::backup_if_exists(&main)?;
    let text = charge_plan::dump_charge_plan_yaml(&yaml)?;
    fsx::atomic_write_text(&main, &text)?;

    Ok(SaveResult {
        written_path: fsx::to_rel_string(&root, &main),
        backup_path: backup_path.map(|p| fsx::to_rel_string(&root, &p)),
    })
}

#[tauri::command]
pub fn migrate_legacy_to_main(
    state: State<'_, AppState>,
    instance_idx: u32,
    group_id: Option<String>,
    mode: String,
    config: Option<ChargePlanConfigModel>,
) -> Result<SaveResult, String> {
    migrate_legacy_to_main_impl(
        &state,
        instance_idx,
        group_id.as_deref().unwrap_or("one_dragon"),
        mode,
        config,
    )
    .map_err(|e| e.to_string())
}

fn migrate_legacy_to_main_impl(
    state: &AppState,
    instance_idx: u32,
    group_id: &str,
    mode: String,
    config: Option<ChargePlanConfigModel>,
) -> AppResult<SaveResult> {
    let root = state.project_root().ok_or(AppError::ProjectRootNotSet)?;
    let (main, legacy) = build_paths(&root, instance_idx, group_id);

    match mode.as_str() {
        "copy" => {
            if main.exists() {
                return Err(AppError::ValidationFailed(
                    "主路径已存在，copy 模式不会覆盖。".into(),
                ));
            }
            if !legacy.exists() {
                return Err(AppError::ValidationFailed("历史路径不存在，无法迁移。".into()));
            }
            if let Some(parent) = main.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| AppError::write_file_failed(parent.display().to_string(), e))?;
            }
            std::fs::copy(&legacy, &main)
                .map_err(|e| AppError::write_file_failed(main.display().to_string(), e))?;
            Ok(SaveResult {
                written_path: fsx::to_rel_string(&root, &main),
                backup_path: None,
            })
        }
        "write_current" => {
            let Some(config) = config else {
                return Err(AppError::ValidationFailed(
                    "write_current 模式需要传入当前配置".into(),
                ));
            };
            save_charge_plan_impl(state, instance_idx, group_id, config, SaveOptions::default())
        }
        _ => Err(AppError::ValidationFailed(
            "未知迁移模式，允许值：copy / write_current".into(),
        )),
    }
}
