use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::domain::{compendium, notorious_hunt};
use crate::domain::notorious_hunt::{NotoriousHuntConfigModel, ValidationResult};
use crate::error::{AppError, AppResult};
use crate::infra::fsx;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct NotoriousHuntPaths {
    pub main_path: String,
    pub legacy_path: String,
    pub main_exists: bool,
    pub legacy_exists: bool,
}

#[derive(Debug, Serialize)]
pub struct ReadNotoriousHuntResult {
    pub source: String,
    pub paths: NotoriousHuntPaths,
    pub config: NotoriousHuntConfigModel,
    pub validation: ValidationResult,
}

#[derive(Debug, Serialize)]
pub struct SaveResult {
    pub written_path: String,
    pub backup_path: Option<String>,
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
        .join("notorious_hunt.yml");
    let legacy = root
        .join("config")
        .join(instance_dir_name(instance_idx))
        .join("notorious_hunt.yml");
    (main, legacy)
}

#[tauri::command]
pub fn get_notorious_hunt_paths(
    state: State<'_, AppState>,
    instance_idx: u32,
    group_id: Option<String>,
) -> Result<NotoriousHuntPaths, String> {
    get_notorious_hunt_paths_impl(&state, instance_idx, group_id.as_deref().unwrap_or("one_dragon"))
        .map_err(|e| e.to_string())
}

fn get_notorious_hunt_paths_impl(
    state: &AppState,
    instance_idx: u32,
    group_id: &str,
) -> AppResult<NotoriousHuntPaths> {
    let root = state.project_root().ok_or(AppError::ProjectRootNotSet)?;
    let (main, legacy) = build_paths(&root, instance_idx, group_id);

    Ok(NotoriousHuntPaths {
        main_path: fsx::to_rel_string(&root, &main),
        legacy_path: fsx::to_rel_string(&root, &legacy),
        main_exists: main.exists(),
        legacy_exists: legacy.exists(),
    })
}

#[tauri::command]
pub fn read_notorious_hunt(
    state: State<'_, AppState>,
    instance_idx: u32,
    group_id: Option<String>,
) -> Result<ReadNotoriousHuntResult, String> {
    read_notorious_hunt_impl(&state, instance_idx, group_id.as_deref().unwrap_or("one_dragon"))
        .map_err(|e| e.to_string())
}

fn read_notorious_hunt_impl(
    state: &AppState,
    instance_idx: u32,
    group_id: &str,
) -> AppResult<ReadNotoriousHuntResult> {
    let root = state.project_root().ok_or(AppError::ProjectRootNotSet)?;
    let (main, legacy) = build_paths(&root, instance_idx, group_id);
    let paths = get_notorious_hunt_paths_impl(state, instance_idx, group_id)?;

    let comp = compendium::load_compendium(&root)?;
    let boss_list = notorious_hunt::build_boss_list(&comp)?;

    let (source, config_yaml) = if main.exists() {
        ("main".to_string(), notorious_hunt::load_notorious_hunt_yaml(&main)?)
    } else if legacy.exists() {
        ("legacy".to_string(), notorious_hunt::load_notorious_hunt_yaml(&legacy)?)
    } else {
        ("none".to_string(), notorious_hunt::NotoriousHuntYaml::default())
    };

    let mut validation = notorious_hunt::validate_yaml(&boss_list, &config_yaml)?;

    if source == "legacy" {
        validation.warnings.push(
            "当前读取自历史路径（legacy）。建议迁移到 config/{实例}/one_dragon/notorious_hunt.yml。".into(),
        );
    }
    validation
        .warnings
        .push("主程序运行时可能缓存配置；保存后如果未生效，建议重启主程序。".into());

    // 读取后进行规范化：补齐缺失 BOSS、修正固定字段、以及把明显非法值回收到安全默认，保证 UI 可渲染。
    let config = notorious_hunt::to_model(config_yaml, &boss_list, &mut validation.warnings);

    Ok(ReadNotoriousHuntResult {
        source,
        paths,
        config,
        validation,
    })
}

#[tauri::command]
pub fn validate_notorious_hunt(
    state: State<'_, AppState>,
    config: NotoriousHuntConfigModel,
) -> Result<ValidationResult, String> {
    validate_notorious_hunt_impl(&state, config).map_err(|e| e.to_string())
}

fn validate_notorious_hunt_impl(
    state: &AppState,
    config: NotoriousHuntConfigModel,
) -> AppResult<ValidationResult> {
    let root = state.project_root().ok_or(AppError::ProjectRootNotSet)?;
    let comp = compendium::load_compendium(&root)?;
    let boss_list = notorious_hunt::build_boss_list(&comp)?;

    let yaml = notorious_hunt::from_model(config)?;
    let mut warnings: Vec<String> = Vec::new();
    let normalized = notorious_hunt::normalize_yaml(yaml, &boss_list, &mut warnings);
    let mut validation = notorious_hunt::validate_yaml(&boss_list, &normalized)?;
    validation.warnings.extend(warnings);
    Ok(validation)
}

#[tauri::command]
pub fn save_notorious_hunt(
    state: State<'_, AppState>,
    instance_idx: u32,
    group_id: Option<String>,
    config: NotoriousHuntConfigModel,
) -> Result<SaveResult, String> {
    save_notorious_hunt_impl(&state, instance_idx, group_id.as_deref().unwrap_or("one_dragon"), config)
        .map_err(|e| e.to_string())
}

fn save_notorious_hunt_impl(
    state: &AppState,
    instance_idx: u32,
    group_id: &str,
    config: NotoriousHuntConfigModel,
) -> AppResult<SaveResult> {
    let root = state.project_root().ok_or(AppError::ProjectRootNotSet)?;
    let (main, legacy) = build_paths(&root, instance_idx, group_id);

    if let Some(parent) = main.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::write_file_failed(parent.display().to_string(), e))?;
    }

    // 严格规则：如果当前已有配置文件包含未知键，则拒绝保存（避免工具“悄悄清理”未知键）。
    let existing = if main.exists() {
        Some(main.as_path())
    } else if legacy.exists() {
        Some(legacy.as_path())
    } else {
        None
    };
    if let Some(path) = existing {
        notorious_hunt::validate_existing_file_for_unknown_keys(path)?;
    }

    let comp = compendium::load_compendium(&root)?;
    let boss_list = notorious_hunt::build_boss_list(&comp)?;

    let yaml = notorious_hunt::from_model(config)?;
    let mut normalization_warnings: Vec<String> = Vec::new();
    let normalized = notorious_hunt::normalize_yaml(yaml, &boss_list, &mut normalization_warnings);

    let validation = notorious_hunt::validate_yaml(&boss_list, &normalized)?;
    if !validation.errors.is_empty() {
        return Err(AppError::ValidationFailed(validation.errors.join("\n")));
    }

    let backup_path = fsx::backup_if_exists(&main)?;
    let text = notorious_hunt::dump_notorious_hunt_yaml(&normalized)?;
    fsx::atomic_write_text(&main, &text)?;

    Ok(SaveResult {
        written_path: fsx::to_rel_string(&root, &main),
        backup_path: backup_path.map(|p| fsx::to_rel_string(&root, &p)),
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MigrateResult {
    pub written_path: String,
}

#[tauri::command]
pub fn migrate_notorious_hunt_legacy_to_main(
    state: State<'_, AppState>,
    instance_idx: u32,
    group_id: Option<String>,
    mode: String,
) -> Result<MigrateResult, String> {
    migrate_notorious_hunt_legacy_to_main_impl(
        &state,
        instance_idx,
        group_id.as_deref().unwrap_or("one_dragon"),
        mode,
    )
    .map_err(|e| e.to_string())
}

fn migrate_notorious_hunt_legacy_to_main_impl(
    state: &AppState,
    instance_idx: u32,
    group_id: &str,
    mode: String,
) -> AppResult<MigrateResult> {
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

            Ok(MigrateResult {
                written_path: fsx::to_rel_string(&root, &main),
            })
        }
        other => Err(AppError::ValidationFailed(format!(
            "未知迁移模式：{other}（仅支持 copy）"
        ))),
    }
}

