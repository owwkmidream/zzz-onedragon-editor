use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::compendium::CompendiumData;
use crate::error::{AppError, AppResult};

const RESTORE_CHARGE_ALLOWED: [&str; 4] = [
    "不使用",
    "使用储蓄电量",
    "使用以太电池",
    "同时使用储蓄电量和以太电池",
];

const CARD_NUM_ALLOWED: [&str; 6] = ["默认数量", "1", "2", "3", "4", "5"];

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidationResult {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChargePlanConfigModel {
    pub loop_enabled: bool,
    pub skip_plan: bool,
    pub use_coupon: bool,
    pub restore_charge: String,
    pub plan_list: Vec<ChargePlanItemModel>,
    pub history_list: Vec<ChargePlanItemModel>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChargePlanItemModel {
    pub tab_name: String,
    pub category_name: String,
    pub mission_type_name: String,
    pub mission_name: Option<String>,
    pub level: Option<String>,
    pub auto_battle_config: String,
    pub run_times: i64,
    pub plan_times: i64,
    pub card_num: String,
    pub predefined_team_idx: i64,
    pub notorious_hunt_buff_num: i64,
    pub plan_id: String,
}

fn de_opt_string_or_number<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v: Option<serde_yaml::Value> = Option::deserialize(deserializer)?;
    let Some(v) = v else {
        return Ok(None);
    };

    match v {
        serde_yaml::Value::Null => Ok(None),
        serde_yaml::Value::String(s) => Ok(Some(s)),
        serde_yaml::Value::Number(n) => Ok(Some(n.to_string())),
        other => {
            let dumped = serde_yaml::to_string(&other).unwrap_or_else(|_| String::new());
            Ok(Some(dumped.trim().to_string()))
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct ChargePlanYaml {
    #[serde(default, rename = "loop")]
    pub loop_: Option<bool>,
    #[serde(default)]
    pub skip_plan: Option<bool>,
    #[serde(default)]
    pub use_coupon: Option<bool>,
    #[serde(default)]
    pub restore_charge: Option<String>,
    #[serde(default)]
    pub plan_list: Vec<ChargePlanItemYaml>,
    #[serde(default)]
    pub history_list: Vec<ChargePlanItemYaml>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChargePlanItemYaml {
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
    pub auto_battle_config: Option<String>,
    #[serde(default)]
    pub run_times: Option<i64>,
    #[serde(default)]
    pub plan_times: Option<i64>,
    #[serde(default, deserialize_with = "de_opt_string_or_number")]
    pub card_num: Option<String>,
    #[serde(default)]
    pub predefined_team_idx: Option<i64>,
    #[serde(default)]
    pub notorious_hunt_buff_num: Option<i64>,
    #[serde(default)]
    pub plan_id: Option<String>,

    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChargePlanItemToSave {
    pub tab_name: String,
    pub category_name: String,
    pub mission_type_name: String,
    pub mission_name: Option<String>,
    pub level: String,
    pub auto_battle_config: String,
    pub run_times: i64,
    pub plan_times: i64,
    pub card_num: String,
    pub predefined_team_idx: i64,
    pub notorious_hunt_buff_num: i64,
    pub plan_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChargePlanToSave {
    #[serde(rename = "loop")]
    pub loop_: bool,
    pub skip_plan: bool,
    pub use_coupon: bool,
    pub restore_charge: String,
    pub plan_list: Vec<ChargePlanItemToSave>,
    pub history_list: Vec<ChargePlanItemToSave>,
}

pub fn load_charge_plan_yaml(path: &Path) -> AppResult<ChargePlanYaml> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| AppError::read_file_failed(path.display().to_string(), e))?;
    serde_yaml::from_str(&text)
        .map_err(|e| AppError::parse_yaml_failed(path.display().to_string(), e))
}

pub fn dump_charge_plan_yaml(yaml: &ChargePlanYaml) -> AppResult<String> {
    let to_save = ChargePlanToSave {
        loop_: yaml.loop_.unwrap_or(true),
        skip_plan: yaml.skip_plan.unwrap_or(false),
        use_coupon: yaml.use_coupon.unwrap_or(false),
        restore_charge: yaml
            .restore_charge
            .clone()
            .unwrap_or_else(|| "不使用".into()),
        plan_list: yaml.plan_list.iter().map(item_to_save).collect(),
        history_list: yaml.history_list.iter().map(item_to_save).collect(),
    };

    serde_yaml::to_string(&to_save).map_err(|e| AppError::write_file_failed("<memory>", e))
}

fn item_to_save(item: &ChargePlanItemYaml) -> ChargePlanItemToSave {
    ChargePlanItemToSave {
        tab_name: item.tab_name.clone().unwrap_or_else(|| "训练".into()),
        category_name: item.category_name.clone().unwrap_or_default(),
        mission_type_name: item.mission_type_name.clone().unwrap_or_default(),
        mission_name: item.mission_name.clone(),
        level: item.level.clone().unwrap_or_else(|| "默认等级".into()),
        auto_battle_config: item
            .auto_battle_config
            .clone()
            .unwrap_or_else(|| "全配队通用".into()),
        run_times: item.run_times.unwrap_or(0),
        plan_times: item.plan_times.unwrap_or(1),
        card_num: item.card_num.clone().unwrap_or_else(|| "默认数量".into()),
        predefined_team_idx: item.predefined_team_idx.unwrap_or(-1),
        notorious_hunt_buff_num: item.notorious_hunt_buff_num.unwrap_or(1),
        plan_id: item
            .plan_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string()),
    }
}

pub fn to_model(mut yaml: ChargePlanYaml, warnings: &mut Vec<String>) -> ChargePlanConfigModel {
    let mut generated = 0usize;
    for item in yaml.plan_list.iter_mut().chain(yaml.history_list.iter_mut()) {
        if item.plan_id.as_deref().unwrap_or("").is_empty() {
            item.plan_id = Some(Uuid::new_v4().to_string());
            generated += 1;
        }
        if item.tab_name.as_deref().unwrap_or("").is_empty() {
            item.tab_name = Some("训练".into());
        }
    }

    if generated > 0 {
        warnings.push(format!(
            "已为 {generated} 个缺失 plan_id 的条目生成 UUID；保存时将写回文件。"
        ));
    }

    ChargePlanConfigModel {
        loop_enabled: yaml.loop_.unwrap_or(true),
        skip_plan: yaml.skip_plan.unwrap_or(false),
        use_coupon: yaml.use_coupon.unwrap_or(false),
        restore_charge: yaml.restore_charge.unwrap_or_else(|| "不使用".into()),
        plan_list: yaml.plan_list.into_iter().map(item_yaml_to_model).collect(),
        history_list: yaml
            .history_list
            .into_iter()
            .map(item_yaml_to_model)
            .collect(),
    }
}

fn item_yaml_to_model(item: ChargePlanItemYaml) -> ChargePlanItemModel {
    ChargePlanItemModel {
        tab_name: item.tab_name.unwrap_or_else(|| "训练".into()),
        category_name: item.category_name.unwrap_or_default(),
        mission_type_name: item.mission_type_name.unwrap_or_default(),
        mission_name: item.mission_name,
        level: item.level,
        auto_battle_config: item
            .auto_battle_config
            .unwrap_or_else(|| "全配队通用".into()),
        run_times: item.run_times.unwrap_or(0),
        plan_times: item.plan_times.unwrap_or(1),
        card_num: item.card_num.unwrap_or_else(|| "默认数量".into()),
        predefined_team_idx: item.predefined_team_idx.unwrap_or(-1),
        notorious_hunt_buff_num: item.notorious_hunt_buff_num.unwrap_or(1),
        plan_id: item.plan_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
    }
}

pub fn from_model(model: ChargePlanConfigModel) -> AppResult<ChargePlanYaml> {
    Ok(ChargePlanYaml {
        loop_: Some(model.loop_enabled),
        skip_plan: Some(model.skip_plan),
        use_coupon: Some(model.use_coupon),
        restore_charge: Some(model.restore_charge),
        plan_list: model.plan_list.into_iter().map(item_model_to_yaml).collect(),
        history_list: model
            .history_list
            .into_iter()
            .map(item_model_to_yaml)
            .collect(),
    })
}

fn item_model_to_yaml(item: ChargePlanItemModel) -> ChargePlanItemYaml {
    ChargePlanItemYaml {
        tab_name: Some(item.tab_name),
        category_name: Some(item.category_name),
        mission_type_name: Some(item.mission_type_name),
        mission_name: item.mission_name,
        level: item.level,
        auto_battle_config: Some(item.auto_battle_config),
        run_times: Some(item.run_times),
        plan_times: Some(item.plan_times),
        card_num: Some(item.card_num),
        predefined_team_idx: Some(item.predefined_team_idx),
        notorious_hunt_buff_num: Some(item.notorious_hunt_buff_num),
        plan_id: Some(item.plan_id),
        extra: BTreeMap::new(),
    }
}

pub fn build_new_history_list(
    plan_list: &[ChargePlanItemYaml],
    old_history_list: &[ChargePlanItemYaml],
) -> Vec<ChargePlanItemYaml> {
    let mut new_history_list = Vec::new();
    for plan in plan_list {
        new_history_list.push(strip_extra(plan));
    }

    for old in old_history_list {
        let found = plan_list.iter().any(|p| is_same_plan(p, old));
        if !found {
            new_history_list.push(strip_extra(old));
        }
    }

    new_history_list
}

fn strip_extra(item: &ChargePlanItemYaml) -> ChargePlanItemYaml {
    let mut copied = item.clone();
    copied.extra = BTreeMap::new();
    copied
}

fn is_same_plan(x: &ChargePlanItemYaml, y: &ChargePlanItemYaml) -> bool {
    let xid = x.plan_id.as_deref().unwrap_or("");
    let yid = y.plan_id.as_deref().unwrap_or("");
    if !xid.is_empty() && !yid.is_empty() {
        return xid == yid;
    }

    x.tab_name.as_deref().unwrap_or("") == y.tab_name.as_deref().unwrap_or("")
        && x.category_name.as_deref().unwrap_or("") == y.category_name.as_deref().unwrap_or("")
        && x.mission_type_name.as_deref().unwrap_or("")
            == y.mission_type_name.as_deref().unwrap_or("")
        && x.mission_name.as_deref().unwrap_or("") == y.mission_name.as_deref().unwrap_or("")
}

pub fn validate_config(comp: &CompendiumData, yaml: &ChargePlanYaml) -> AppResult<ValidationResult> {
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    let restore = yaml.restore_charge.as_deref().unwrap_or("不使用");
    if !RESTORE_CHARGE_ALLOWED.contains(&restore) {
        errors.push(format!(
            "restore_charge 非法：{restore}，允许值：{}",
            RESTORE_CHARGE_ALLOWED.join(" / ")
        ));
    }

    // 说明：主程序不会对 history_list 做 compendium 级别校验（只要字段能反序列化即可）。
    // 为了与 GUI 保存保持一致，本工具只对 plan_list 做 compendium 严格校验。
    validate_items(
        comp,
        "plan_list",
        &yaml.plan_list,
        true,
        &mut errors,
        &mut warnings,
    );
    validate_items(
        comp,
        "history_list",
        &yaml.history_list,
        false,
        &mut errors,
        &mut warnings,
    );

    Ok(ValidationResult { errors, warnings })
}

fn validate_items(
    comp: &CompendiumData,
    list_name: &str,
    items: &[ChargePlanItemYaml],
    strict_compendium: bool,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    let training = comp.find_tab("训练");
    if strict_compendium && training.is_none() {
        errors.push("compendium_data.yml 中缺少 tab_name=训练".into());
        return;
    }

    for (idx, item) in items.iter().enumerate() {
        if !item.extra.is_empty() {
            let keys = item
                .extra
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            errors.push(format!(
                "{list_name}[{idx}] 存在未知键：{keys}。主程序会因未知键报错，请先清理。"
            ));
        }

        let plan_id = item.plan_id.as_deref().unwrap_or("").trim();
        if plan_id.is_empty() {
            // 兼容旧配置：主程序会在运行时生成 UUID。本工具保存时也会自动补全。
            warnings.push(format!(
                "{list_name}[{idx}] plan_id 缺失，将在保存时自动生成 UUID"
            ));
        } else if Uuid::parse_str(plan_id).is_err() {
            errors.push(format!("{list_name}[{idx}] plan_id 非法 UUID：{plan_id}"));
        }

        let tab_name = item.tab_name.as_deref().unwrap_or("训练");
        if tab_name != "训练" {
            // GUI 实际固定为“训练”，这里保持一致：plan_list 作为错误；history_list 作为告警。
            if strict_compendium {
                errors.push(format!(
                    "{list_name}[{idx}] tab_name 必须为 训练（当前：{tab_name}）"
                ));
            } else {
                warnings.push(format!(
                    "{list_name}[{idx}] tab_name 建议为 训练（当前：{tab_name}）"
                ));
            }
        }

        let run_times = item.run_times.unwrap_or(0);
        let plan_times = item.plan_times.unwrap_or(0);
        if run_times < 0 || plan_times < 0 {
            errors.push(format!("{list_name}[{idx}] run_times/plan_times 必须 >= 0"));
        }

        let predefined_team_idx = item.predefined_team_idx.unwrap_or(-1);
        if predefined_team_idx != -1 && !(0..=9).contains(&predefined_team_idx) {
            errors.push(format!(
                "{list_name}[{idx}] predefined_team_idx 非法：{predefined_team_idx}，允许 -1 或 0..9"
            ));
        }

        let card_num = item.card_num.as_deref().unwrap_or("默认数量");
        if !CARD_NUM_ALLOWED.contains(&card_num) {
            errors.push(format!(
                "{list_name}[{idx}] card_num 非法：{card_num}，允许值：{}",
                CARD_NUM_ALLOWED.join(" / ")
            ));
        }

        if !strict_compendium {
            // history_list：不做 compendium 严格校验，仅保留结构/枚举/范围校验。
            continue;
        }

        let Some(training) = training else { continue };

        let category_name = item.category_name.as_deref().unwrap_or("");
        let Some(category) = training
            .category_list
            .iter()
            .find(|c| c.category_name == category_name)
        else {
            errors.push(format!("{list_name}[{idx}] category_name 非法：{category_name}"));
            continue;
        };

        let mission_type_name = item.mission_type_name.as_deref().unwrap_or("");
        let Some(mission_type) = category
            .mission_type_list
            .iter()
            .find(|t| t.mission_type_name == mission_type_name)
        else {
            errors.push(format!(
                "{list_name}[{idx}] mission_type_name 非法：{mission_type_name}（category={category_name}）"
            ));
            continue;
        };

        let missions = &mission_type.mission_list;
        let mission_name = item.mission_name.as_deref();
        if missions.is_empty() {
            if mission_name.is_some() && mission_name != Some("") {
                warnings.push(format!(
                    "{list_name}[{idx}] mission_list 为空，建议 mission_name 保存为 null（当前：{}）",
                    mission_name.unwrap_or("")
                ));
            }
        } else {
            let Some(mission_name) = mission_name else {
                errors.push(format!(
                    "{list_name}[{idx}] mission_name 不能为空（category={category_name}, mission_type={mission_type_name}）"
                ));
                continue;
            };

            let ok = missions.iter().any(|m| m.mission_name == mission_name);
            if !ok {
                errors.push(format!(
                    "{list_name}[{idx}] mission_name 非法：{mission_name}（category={category_name}, mission_type={mission_type_name}）"
                ));
            }
        }
    }
}
