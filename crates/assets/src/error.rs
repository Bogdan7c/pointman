use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AssetError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("truncated {0}")]
    Truncated(&'static str),
    #[error("invalid {0}")]
    Invalid(&'static str),
    #[error("unknown archive {path} magic {magic:?}")]
    UnknownArchive { path: String, magic: [u8; 4] },
    #[error("entry not found: {0}")]
    NotFound(String),
    #[error("unsupported compression method {0}")]
    Compression(u32),
    #[error("zlib: {0}")]
    Zlib(#[from] flate2::DecompressError),
    #[error("utf-8 in name table")]
    Utf8,
    #[error("world version {0} is not Jupiter EX F.E.A.R. (113)")]
    WorldVersion(u32),
    #[error("unsupported DDS fourcc {0:#x}")]
    UnsupportedDds(u32),
}
