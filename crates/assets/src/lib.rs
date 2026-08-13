//! Retail F.E.A.R. asset I/O.
//!
//! Formats are documented by the official mod SDK and public reverse-engineering
//! notes. This crate never ships game data.

mod arch00;
mod dds;
mod error;
mod extensions;
mod mat00;
mod rez;
mod world00p;
mod world_model_draw;
mod world_models;
mod world_objects;

pub use world_model_draw::{world_model_in_frame, BakedOverlapIndex};
pub use world_models::{WorldBsp, WorldModels};
pub use world_objects::{GameStart, WorldLight, WorldModelPlacement, WorldObjects, WorldSky};

pub use arch00::{Arch00, ArchFile, ArchHeader, Compression};
pub use dds::{DdsCubemap, DdsFormat, DdsImage};
pub use error::AssetError;
pub use extensions::{kind_from_path, ResourceKind};
pub use mat00::Material;
pub use rez::{RezArchive, RezEntry};
pub use world00p::{
    SurfaceDraw, WorldHeader, WorldRender, WorldSurface, WorldVertex, FEAR_WORLD_MAGIC,
    FEAR_WORLD_VERSION,
};

pub fn archive_key(logical: &str) -> String {
    logical.replace('\\', "/")
}

/// Имя материала в архиве. В Intro опечатка Tile_MIrrow02, в паке лежит Mirror.
pub fn material_key(logical: &str) -> String {
    archive_key(&logical.replace("Tile_MIrrow02", "Tile_Mirror02"))
}

use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;

/// Open a retail archive. F.E.A.R. 1 uses Arch00 (`LTAR`); older LithTech titles use REZ.
pub enum GameArchive {
    Arch00(Arch00<File>),
    Rez(RezArchive<File>),
}

impl GameArchive {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AssetError> {
        let path = path.as_ref();
        let mut probe = File::open(path)?;
        let mut magic = [0u8; 4];
        probe.read_exact(&mut magic)?;
        drop(probe);

        if magic == *b"LTAR" {
            Ok(Self::Arch00(Arch00::open(path)?))
        } else if magic[0] == 13 && magic[1] == 10 {
            Ok(Self::Rez(RezArchive::open(path)?))
        } else {
            Err(AssetError::UnknownArchive {
                path: path.display().to_string(),
                magic,
            })
        }
    }

    pub fn list(&self) -> Vec<String> {
        match self {
            Self::Arch00(a) => a.files().map(|f| f.path.clone()).collect(),
            Self::Rez(r) => r.entries().map(|e| e.path.clone()).collect(),
        }
    }

    pub fn read(&mut self, path: &str) -> Result<Vec<u8>, AssetError> {
        match self {
            Self::Arch00(a) => a.read(path),
            Self::Rez(r) => r.read(path),
        }
    }
}

pub(crate) fn read_cstring(bytes: &[u8], offset: usize) -> Result<&str, AssetError> {
    if offset >= bytes.len() {
        return Err(AssetError::Truncated("name table offset"));
    }
    let end = bytes[offset..]
        .iter()
        .position(|&b| b == 0)
        .map(|p| offset + p)
        .unwrap_or(bytes.len());
    std::str::from_utf8(&bytes[offset..end]).map_err(|_| AssetError::Utf8)
}

pub(crate) fn read_u32<R: Read>(r: &mut R) -> Result<u32, AssetError> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

pub(crate) fn read_exact_vec<R: Read + Seek>(r: &mut R, len: usize) -> Result<Vec<u8>, AssetError> {
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::material_key;

    #[test]
    fn aliases_intro_mirror_typo() {
        assert_eq!(
            material_key(r"Materials\Tech\Research\Tile_MIrrow02.Mat00"),
            "Materials/Tech/Research/Tile_Mirror02.Mat00"
        );
    }
}
