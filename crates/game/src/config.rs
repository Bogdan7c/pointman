use crate::paths::{detect_game_root, DEFAULT_ARCHCFG};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub game: GameConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct GameConfig {
    pub root: Option<PathBuf>,
    #[serde(default = "default_archcfg")]
    pub archcfg: String,
}

fn default_archcfg() -> String {
    DEFAULT_ARCHCFG.to_string()
}

impl Config {
    pub fn load() -> Self {
        let path = Path::new("pointman.toml");
        if path.is_file() {
            match fs::read_to_string(path) {
                Ok(text) => match toml::from_str(&text) {
                    Ok(cfg) => return cfg,
                    Err(err) => log::error!("pointman.toml: {err}"),
                },
                Err(err) => log::error!("pointman.toml: {err}"),
            }
        }
        Self::default()
    }

    pub fn game_root(&self) -> Option<PathBuf> {
        self.game
            .root
            .clone()
            .filter(|p| p.is_dir())
            .or_else(detect_game_root)
    }
}
