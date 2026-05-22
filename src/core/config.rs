use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum AppTheme {
    Light,
    #[default]
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub to_sort_dir: Option<PathBuf>,
    pub library_dir: Option<PathBuf>,
    #[serde(default)]
    pub theme: AppTheme,
}

impl Default for Config {
    fn default() -> Self {
        let dirs = directories::UserDirs::new();
        let pictures = dirs
            .as_ref()
            .and_then(|d| d.picture_dir().map(|p| p.to_path_buf()));

        Self {
            to_sort_dir: pictures.as_ref().map(|p| p.join("ToSort")),
            library_dir: pictures.as_ref().map(|p| p.join("Library")),
            theme: AppTheme::default(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let proj_dirs = directories::ProjectDirs::from("com", "mlambir", "photosort")
            .ok_or("Could not find project directories")?;
        let config_path = proj_dirs.config_dir().join("config.json");

        if config_path.exists() {
            let data = std::fs::read_to_string(config_path)?;
            let config = serde_json::from_str(&data)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let proj_dirs = directories::ProjectDirs::from("com", "mlambir", "photosort")
            .ok_or("Could not find project directories")?;
        let config_dir = proj_dirs.config_dir();
        std::fs::create_dir_all(config_dir)?;
        let config_path = config_dir.join("config.json");

        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(config_path, data)?;
        Ok(())
    }
}
