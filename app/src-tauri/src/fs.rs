use std::path::PathBuf;

// Used by M5 (history_parser, skills_scanner) and beyond.
#[allow(dead_code)]
pub fn claude_home() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude"))
}

pub fn app_data_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|c| c.join("ClaudeTool"))
}
