use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub last_repo_path: Option<String>,
}

fn config_path() -> Option<PathBuf> {
    let appdata = std::env::var("APPDATA").ok()?;
    let dir = PathBuf::from(appdata).join("gitwit");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("config.toml"))
}

pub fn load_config() -> AppConfig {
    let Some(path) = config_path() else {
        return AppConfig::default();
    };
    let Ok(content) = std::fs::read_to_string(&path) else {
        return AppConfig::default();
    };
    toml::from_str(&content).unwrap_or_default()
}

pub fn save_config(config: &AppConfig) {
    let Some(path) = config_path() else {
        return;
    };
    match toml::to_string(config) {
        Ok(content) => {
            if let Err(e) = std::fs::write(&path, content) {
                eprintln!("設定の保存に失敗しました: {} ({})", path.display(), e);
            }
        }
        Err(e) => eprintln!("設定のシリアライズに失敗しました: {}", e),
    }
}
