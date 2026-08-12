//! Retail F.E.A.R. asset I/O.
//!
//! Formats are documented by the official mod SDK and public reverse-engineering
//! notes. This crate never ships game data.

mod arch00;
mod error;
mod extensions;
mod rez;
mod world00p;

pub use arch00::{Arch00, ArchFile, ArchHeader, Compression};
pub use error::AssetError;
pub use extensions::{kind_from_path, ResourceKind};
pub use rez::{RezArchive, RezEntry};
pub use world00p::{WorldHeader, FEAR_WORLD_MAGIC, FEAR_WORLD_VERSION};

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
