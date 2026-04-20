// Module organization: single .rs files for M3 simplicity.
// Will convert to folder + mod.rs pattern per ARCHITECTURE.md once
// individual modules grow beyond a single file's worth of code.
mod commands;
mod config;
mod env_checker;
mod error;
mod fs;
mod history_parser;
mod installer;
mod net;
mod skills_scanner;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_pty::init())
        .invoke_handler(tauri::generate_handler![
            commands::check_environment,
            commands::get_config,
            commands::set_suppress_login_prompt,
            commands::set_last_seen_version,
            commands::set_debug_flag,
            commands::list_sessions,
            commands::list_skills,
            commands::read_skill_md,
            commands::install_git,
            commands::install_claude_code,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
