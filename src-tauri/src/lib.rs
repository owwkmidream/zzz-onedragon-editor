#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(crate::state::AppState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            crate::commands::project::detect_project_root,
            crate::commands::project::set_project_root,
            crate::commands::project::list_instances,
            crate::commands::options::load_compendium_for_charge_plan,
            crate::commands::options::load_team_list,
            crate::commands::options::list_auto_battle_templates,
            crate::commands::charge_plan::get_charge_plan_paths,
            crate::commands::charge_plan::read_charge_plan,
            crate::commands::charge_plan::validate_charge_plan,
            crate::commands::charge_plan::save_charge_plan,
            crate::commands::charge_plan::migrate_legacy_to_main,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

mod commands;
mod domain;
mod error;
mod infra;
mod state;
