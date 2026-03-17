use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::domain::compendium::CompendiumData;
use crate::error::{AppError, AppResult};

const TAB_NAME: &str = "训练";
const CATEGORY_NAME: &str = "恶名狩猎";
const MISSION_TYPE_AGENT_PLAN: &str = "代理人方案培养";

const LEVEL_ALLOWED: [&str; 6] = [
    "默认等级",
    "等级Lv.65",
    "等级Lv.60",
    "等级Lv.50",
    "等级Lv.40",
    "等级Lv.30",
];

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidationResult {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NotoriousHuntConfigModel {
    pub plan_list: Vec<NotoriousHuntItemModel>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NotoriousHuntItemModel {
    pub mission_type_name: String,
    pub level: String,
    pub predefined_team_idx: i64,
    pub auto_battle_config: String,
    pub run_times: i64,
    pub plan_times: i64,
    pub notorious_hunt_buff_num: i64,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct NotoriousHuntYaml {
    #[serde(default)]
    pub plan_list: Vec<NotoriousHuntItemYaml>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NotoriousHuntItemYaml {
    #[serde(default)]
    pub tab_name: Option<String>,
    #[serde(default)]
    pub category_name: Option<String>,
    #[serde(default)]
    pub mission_type_name: Option<String>,
    #[serde(default)]
    pub mission_name: Option<String>,
    #[serde(default)]
    pub level: Option<String>,
    #[serde(default)]
    pub predefined_team_idx: Option<i64>,
    #[serde(default)]
    pub auto_battle_config: Option<String>,
    #[serde(default)]
    pub run_times: Option<i64>,
    #[serde(default)]
    pub plan_times: Option<i64>,
    #[serde(default)]
    pub notorious_hunt_buff_num: Option<i64>,

    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NotoriousHuntItemToSave {
    pub tab_name: String,
    pub category_name: String,
    pub mission_type_name: String,
    pub mission_name: Option<String>,
    pub level: String,
    pub predefined_team_idx: i64,
    pub auto_battle_config: String,
    pub run_times: i64,
    pub plan_times: i64,
    pub notorious_hunt_buff_num: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct NotoriousHuntToSave {
    pub plan_list: Vec<NotoriousHuntItemToSave>,
}

pub fn load_notorious_hunt_yaml(path: &Path) -> AppResult<NotoriousHuntYaml> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| AppError::read_file_failed(path.display().to_string(), e))?;
    serde_yaml::from_str(&text).map_err(|e| AppError::parse_yaml_failed(path.display().to_string(), e))
}

pub fn dump_notorious_hunt_yaml(yaml: &NotoriousHuntYaml) -> AppResult<String> {
    let to_save = NotoriousHuntToSave {
        plan_list: yaml.plan_list.iter().map(item_to_save).collect(),
    };
    serde_yaml::to_string(&to_save).map_err(|e| AppError::write_file_failed("<memory>", e))
}

fn clamp_non_negative(n: i64) -> i64 {
    if n < 0 { 0 } else { n }
}

fn clamp_team_idx(n: i64) -> i64 {
    if n == -1 || (0..=9).contains(&n) { n } else { -1 }
}

fn clamp_buff_num(n: i64) -> i64 {
    if (1..=3).contains(&n) { n } else { 1 }
}

fn clamp_level(level: &str) -> &str {
    if LEVEL_ALLOWED.contains(&level) {
        level
    } else {
        "默认等级"
    }
}

fn item_to_save(item: &NotoriousHuntItemYaml) -> NotoriousHuntItemToSave {
    let level = item.level.as_deref().unwrap_or("默认等级");
    NotoriousHuntItemToSave {
        tab_name: TAB_NAME.into(),
        category_name: CATEGORY_NAME.into(),
        mission_type_name: item.mission_type_name.clone().unwrap_or_default(),
        mission_name: None,
        level: clamp_level(level).to_string(),
        predefined_team_idx: clamp_team_idx(item.predefined_team_idx.unwrap_or(-1)),
        auto_battle_config: item
            .auto_battle_config
            .clone()
            .unwrap_or_else(|| "全配队通用".into()),
        run_times: clamp_non_negative(item.run_times.unwrap_or(0)),
        plan_times: clamp_non_negative(item.plan_times.unwrap_or(1)),
        notorious_hunt_buff_num: clamp_buff_num(item.notorious_hunt_buff_num.unwrap_or(1)),
    }
}

pub fn build_boss_list(comp: &CompendiumData) -> AppResult<Vec<String>> {
    let training = comp
        .find_tab(TAB_NAME)
        .ok_or_else(|| AppError::ValidationFailed("compendium_data.yml 中找不到 tab_name=训练".into()))?;

    let category = training
        .category_list
        .iter()
        .find(|c| c.category_name == CATEGORY_NAME)
        .ok_or_else(|| {
            AppError::ValidationFailed("compendium_data.yml 中找不到 category_name=恶名狩猎".into())
        })?;

    let mut list: Vec<String> = Vec::new();
    for mt in &category.mission_type_list {
        if mt.mission_type_name == MISSION_TYPE_AGENT_PLAN {
            continue;
        }
        list.push(mt.mission_type_name.clone());
    }

    if list.is_empty() {
        return Err(AppError::ValidationFailed(
            "compendium_data.yml 中恶名狩猎 BOSS 列表为空".into(),
        ));
    }

    Ok(list)
}

pub fn normalize_yaml(mut yaml: NotoriousHuntYaml, boss_list: &[String], warnings: &mut Vec<String>) -> NotoriousHuntYaml {
    let boss_set: BTreeSet<&str> = boss_list.iter().map(|s| s.as_str()).collect();

    let mut ordered: Vec<NotoriousHuntItemYaml> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();

    // 1) 保留原有顺序：先遍历原列表
    for mut item in yaml.plan_list.drain(..) {
        let boss = item.mission_type_name.clone().unwrap_or_default();
        let boss = boss.trim();
        if boss.is_empty() {
            warnings.push("plan_list 中存在 mission_type_name 为空的条目，已忽略。".into());
            continue;
        }
        if !boss_set.contains(boss) {
            warnings.push(format!("配置包含已不存在的 BOSS：{boss}，已忽略。"));
            continue;
        }
        if seen.contains(boss) {
            warnings.push(format!(
                "配置中存在重复 BOSS：{boss}，已保留第一个，其余已忽略。"
            ));
            continue;
        }
        seen.insert(boss.to_string());

        // 2) 规范化固定字段
        item.tab_name = Some(TAB_NAME.into());
        item.category_name = Some(CATEGORY_NAME.into());
        item.mission_type_name = Some(boss.to_string());
        if item.mission_name.as_deref().unwrap_or("") != "" {
            warnings.push(format!(
                "BOSS {boss} 的 mission_name 非 null（当前：{}），将规范化为 null。",
                item.mission_name.clone().unwrap_or_default()
            ));
        }
        item.mission_name = None;

        // 3) 规范化可编辑字段到安全默认（保证 UI 可渲染）
        let level = item.level.clone().unwrap_or_else(|| "默认等级".into());
        let fixed_level = clamp_level(&level).to_string();
        if fixed_level != level {
            warnings.push(format!(
                "BOSS {boss} 的 level 非法（当前：{level}），将规范化为 {fixed_level}。"
            ));
        }
        item.level = Some(fixed_level);

        let team = item.predefined_team_idx.unwrap_or(-1);
        let fixed_team = clamp_team_idx(team);
        if fixed_team != team {
            warnings.push(format!(
                "BOSS {boss} 的 predefined_team_idx 非法（当前：{team}），将规范化为 {fixed_team}。"
            ));
        }
        item.predefined_team_idx = Some(fixed_team);

        item.auto_battle_config = Some(
            item.auto_battle_config
                .clone()
                .unwrap_or_else(|| "全配队通用".into()),
        );

        let run_times = item.run_times.unwrap_or(0);
        let fixed_run = clamp_non_negative(run_times);
        if fixed_run != run_times {
            warnings.push(format!(
                "BOSS {boss} 的 run_times 为负数（当前：{run_times}），将规范化为 {fixed_run}。"
            ));
        }
        item.run_times = Some(fixed_run);

        let plan_times = item.plan_times.unwrap_or(1);
        let fixed_plan = clamp_non_negative(plan_times);
        if fixed_plan != plan_times {
            warnings.push(format!(
                "BOSS {boss} 的 plan_times 为负数（当前：{plan_times}），将规范化为 {fixed_plan}。"
            ));
        }
        item.plan_times = Some(fixed_plan);

        let buff = item.notorious_hunt_buff_num.unwrap_or(1);
        let fixed_buff = clamp_buff_num(buff);
        if fixed_buff != buff {
            warnings.push(format!(
                "BOSS {boss} 的 notorious_hunt_buff_num 非法（当前：{buff}），将规范化为 {fixed_buff}。"
            ));
        }
        item.notorious_hunt_buff_num = Some(fixed_buff);

        // extra 不保留（工具严格输出 10 键）
        item.extra = BTreeMap::new();

        ordered.push(item);
    }

    // 4) 补齐缺失的 BOSS 到末尾
    for boss in boss_list {
        if seen.contains(boss) {
            continue;
        }
        warnings.push(format!("缺失 BOSS {boss}，已按默认值补齐到列表末尾。"));
        ordered.push(default_item_yaml(boss));
    }

    NotoriousHuntYaml { plan_list: ordered }
}

fn default_item_yaml(boss: &str) -> NotoriousHuntItemYaml {
    NotoriousHuntItemYaml {
        tab_name: Some(TAB_NAME.into()),
        category_name: Some(CATEGORY_NAME.into()),
        mission_type_name: Some(boss.into()),
        mission_name: None,
        level: Some("默认等级".into()),
        predefined_team_idx: Some(-1),
        auto_battle_config: Some("全配队通用".into()),
        run_times: Some(0),
        plan_times: Some(1),
        notorious_hunt_buff_num: Some(1),
        extra: BTreeMap::new(),
    }
}

pub fn to_model(yaml: NotoriousHuntYaml, boss_list: &[String], warnings: &mut Vec<String>) -> NotoriousHuntConfigModel {
    let normalized = normalize_yaml(yaml, boss_list, warnings);
    NotoriousHuntConfigModel {
        plan_list: normalized.plan_list.into_iter().map(item_yaml_to_model).collect(),
    }
}

fn item_yaml_to_model(item: NotoriousHuntItemYaml) -> NotoriousHuntItemModel {
    NotoriousHuntItemModel {
        mission_type_name: item.mission_type_name.unwrap_or_default(),
        level: item.level.unwrap_or_else(|| "默认等级".into()),
        predefined_team_idx: item.predefined_team_idx.unwrap_or(-1),
        auto_battle_config: item
            .auto_battle_config
            .unwrap_or_else(|| "全配队通用".into()),
        run_times: item.run_times.unwrap_or(0),
        plan_times: item.plan_times.unwrap_or(1),
        notorious_hunt_buff_num: item.notorious_hunt_buff_num.unwrap_or(1),
    }
}

pub fn from_model(model: NotoriousHuntConfigModel) -> AppResult<NotoriousHuntYaml> {
    Ok(NotoriousHuntYaml {
        plan_list: model.plan_list.into_iter().map(item_model_to_yaml).collect(),
    })
}

fn item_model_to_yaml(item: NotoriousHuntItemModel) -> NotoriousHuntItemYaml {
    NotoriousHuntItemYaml {
        tab_name: Some(TAB_NAME.into()),
        category_name: Some(CATEGORY_NAME.into()),
        mission_type_name: Some(item.mission_type_name),
        mission_name: None,
        level: Some(item.level),
        predefined_team_idx: Some(item.predefined_team_idx),
        auto_battle_config: Some(item.auto_battle_config),
        run_times: Some(item.run_times),
        plan_times: Some(item.plan_times),
        notorious_hunt_buff_num: Some(item.notorious_hunt_buff_num),
        extra: BTreeMap::new(),
    }
}

pub fn validate_yaml(boss_list: &[String], yaml: &NotoriousHuntYaml) -> AppResult<ValidationResult> {
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    let boss_set: BTreeSet<&str> = boss_list.iter().map(|s| s.as_str()).collect();

    let mut seen: BTreeSet<String> = BTreeSet::new();
    for (idx, item) in yaml.plan_list.iter().enumerate() {
        if !item.extra.is_empty() {
            let keys = item
                .extra
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            errors.push(format!("plan_list[{idx}] 存在未知键：{keys}"));
        }

        let boss = item.mission_type_name.as_deref().unwrap_or("").trim();
        if boss.is_empty() {
            errors.push(format!("plan_list[{idx}] mission_type_name 不能为空"));
            continue;
        }
        if !boss_set.contains(boss) {
            errors.push(format!(
                "plan_list[{idx}] mission_type_name 非法：{boss}（已不在当前 BOSS 列表中）"
            ));
            continue;
        }
        if seen.contains(boss) {
            errors.push(format!("plan_list[{idx}] mission_type_name 重复：{boss}"));
        }
        seen.insert(boss.to_string());

        let mission_name = item.mission_name.as_deref();
        if mission_name.is_some() && mission_name != Some("") {
            errors.push(format!(
                "plan_list[{idx}] mission_name 必须为 null（当前：{}）",
                mission_name.unwrap_or("")
            ));
        }

        let level = item.level.as_deref().unwrap_or("默认等级");
        if !LEVEL_ALLOWED.contains(&level) {
            errors.push(format!(
                "plan_list[{idx}] level 非法：{level}，允许值：{}",
                LEVEL_ALLOWED.join(" / ")
            ));
        }

        let predefined_team_idx = item.predefined_team_idx.unwrap_or(-1);
        if predefined_team_idx != -1 && !(0..=9).contains(&predefined_team_idx) {
            errors.push(format!(
                "plan_list[{idx}] predefined_team_idx 非法：{predefined_team_idx}，允许 -1 或 0..9"
            ));
        }

        let run_times = item.run_times.unwrap_or(0);
        let plan_times = item.plan_times.unwrap_or(1);
        if run_times < 0 || plan_times < 0 {
            errors.push(format!("plan_list[{idx}] run_times/plan_times 必须 >= 0"));
        }

        let buff = item.notorious_hunt_buff_num.unwrap_or(1);
        if !(1..=3).contains(&buff) {
            errors.push(format!(
                "plan_list[{idx}] notorious_hunt_buff_num 非法：{buff}，允许 1..3"
            ));
        }
    }

    if yaml.plan_list.is_empty() {
        errors.push("plan_list 不能为空".into());
    }

    // 列表完整性：必须覆盖全部 boss_list（固定长度）
    let missing = boss_list
        .iter()
        .filter(|b| !seen.contains(*b))
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        warnings.push(format!("缺失 BOSS 条目：{}（读取时会自动补齐）", missing.join("、")));
    }

    // 全局危险校验：禁止全部 plan_times 为 0（避免主程序 reset_plans 死循环）
    let all_zero = yaml
        .plan_list
        .iter()
        .all(|i| i.plan_times.unwrap_or(1) == 0);
    if all_zero {
        errors.push("所有条目的 plan_times 全部为 0：主程序会在 reset_plans() 进入死循环，请至少保留一个 BOSS 的 plan_times > 0。".into());
    }

    Ok(ValidationResult { errors, warnings })
}

pub fn validate_existing_file_for_unknown_keys(path: &Path) -> AppResult<()> {
    if !path.exists() {
        return Ok(());
    }
    let yaml = load_notorious_hunt_yaml(path)?;
    let mut errors: Vec<String> = Vec::new();
    for (idx, item) in yaml.plan_list.iter().enumerate() {
        if !item.extra.is_empty() {
            let keys = item
                .extra
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            errors.push(format!("plan_list[{idx}] 存在未知键：{keys}"));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(AppError::ValidationFailed(format!(
            "检测到现有配置包含未知键，工具拒绝保存。\n{}",
            errors.join("\n")
        )))
    }
}

