//! F.E.A.R. game module: retail Steam mount, Arch00 index, future object classes.

mod config;
mod paths;

pub use config::Config;
pub use paths::{detect_game_root, STEAM_APP_ID, STEAM_FOLDER};

use pointman_assets::{kind_from_path, ArchHeader, ResourceKind};
use std::fs;
use std::path::{Path, PathBuf};

pub struct GameMount {
    pub root: PathBuf,
    pub archcfg: PathBuf,
    pub archives: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ArchiveProbe {
    pub path: PathBuf,
    pub bytes: u64,
    pub files: u32,
    pub folders: u32,
}

impl GameMount {
    pub fn discover(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        let archcfg = root.join(paths::DEFAULT_ARCHCFG);
        let archives = if archcfg.is_file() {
            read_archcfg(&root, &archcfg)
        } else {
            scan_archives(&root)
        };
        Self {
            root,
            archcfg,
            archives,
        }
    }

    pub fn from_config(cfg: &Config) -> Option<Self> {
        cfg.game_root().map(Self::discover)
    }

    pub fn probe(&self) -> Vec<Result<ArchiveProbe, String>> {
        self.archives
            .iter()
            .map(|path| {
                let bytes = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                match ArchHeader::probe(path) {
                    Ok(h) => Ok(ArchiveProbe {
                        path: path.clone(),
                        bytes,
                        files: h.file_count,
                        folders: h.folder_count,
                    }),
                    Err(err) => Err(format!("{}: {err}", path.display())),
                }
            })
            .collect()
    }

    pub fn log_inventory(&self) {
        log::info!("game root: {}", self.root.display());
        log::info!("archcfg: {}", self.archcfg.display());
        if self.archives.is_empty() {
            log::warn!("no archives listed");
            return;
        }
        let mut files = 0u64;
        let mut bytes = 0u64;
        for result in self.probe() {
            match result {
                Ok(p) => {
                    files += u64::from(p.files);
                    bytes += p.bytes;
                    log::info!(
                        "  {}  {:>8} files  {:>8.1} MiB",
                        p.path.file_name().unwrap_or_default().to_string_lossy(),
                        p.files,
                        p.bytes as f64 / (1024.0 * 1024.0)
                    );
                }
                Err(err) => log::error!("{err}"),
            }
        }
        log::info!(
            "mounted {} archives, {files} files, {:.1} GiB",
            self.archives.len(),
            bytes as f64 / (1024.0 * 1024.0 * 1024.0)
        );
    }
}

fn read_archcfg(root: &Path, archcfg: &Path) -> Vec<PathBuf> {
    let Ok(text) = fs::read_to_string(archcfg) else {
        return scan_archives(root);
    };
    let mut archives = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        let path = root.join(line);
        if path.is_file() {
            archives.push(path);
        } else {
            log::warn!("archcfg missing {}", path.display());
        }
    }
    archives
}

fn scan_archives(root: &Path) -> Vec<PathBuf> {
    let mut archives = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            let kind = kind_from_path(&path.to_string_lossy());
            if matches!(kind, ResourceKind::ArchiveArch00 | ResourceKind::ArchiveRez) {
                archives.push(path);
            }
        }
    }
    archives.sort();
    archives
}

#[cfg(test)]
mod tests {
    use super::read_archcfg;
    use std::fs;

    #[test]
    fn archcfg_order() {
        let dir = tempfile();
        fs::write(dir.join("Default.archcfg"), "b.Arch00\na.Arch00\n").unwrap();
        fs::write(dir.join("a.Arch00"), b"x").unwrap();
        fs::write(dir.join("b.Arch00"), b"x").unwrap();
        let list = read_archcfg(&dir, &dir.join("Default.archcfg"));
        let names: Vec<_> = list
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["b.Arch00", "a.Arch00"]);
        let _ = fs::remove_dir_all(dir);
    }

    fn tempfile() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("pointman-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
