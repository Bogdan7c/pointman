//! Locate a retail F.E.A.R. Ultimate Shooter Edition install (Steam app 21090).

use std::path::{Path, PathBuf};

pub const STEAM_APP_ID: u32 = 21090;
pub const STEAM_FOLDER: &str = "FEAR Ultimate Shooter Edition";
pub const DEFAULT_ARCHCFG: &str = "Default.archcfg";

pub fn steam_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(root) = std::env::var_os("POINTMAN_GAME_ROOT") {
        out.push(PathBuf::from(root));
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        out.push(
            home.join(".local/share/Steam/steamapps/common")
                .join(STEAM_FOLDER),
        );
        out.push(home.join(".steam/steam/steamapps/common").join(STEAM_FOLDER));
        out.push(home.join(".steam/root/steamapps/common").join(STEAM_FOLDER));
        out.push(home.join(".steam/debian-installation/steamapps/common").join(STEAM_FOLDER));
    }
    if let Ok(cwd) = std::env::current_dir() {
        out.push(cwd.join("game-data"));
    }
    out
}

pub fn looks_like_fear_root(path: &Path) -> bool {
    path.join("FEAR.exe").is_file() && path.join(DEFAULT_ARCHCFG).is_file()
}

pub fn detect_game_root() -> Option<PathBuf> {
    steam_candidates().into_iter().find(|p| looks_like_fear_root(p))
}
