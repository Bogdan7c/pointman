//! Jupiter EX Arch00 (`LTAR`) used by retail F.E.A.R.
//!
//! Layout (little-endian), documented by community extractors:
//! header 48 bytes → name table → file entries (32 B) → folder entries (16 B) → payloads.

use crate::{read_cstring, AssetError};
use flate2::Decompress;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::Path;

pub const LTAR_MAGIC: [u8; 4] = *b"LTAR";
pub const COMP_RAW: u32 = 0;
pub const COMP_ZLIB: u32 = 9;

#[derive(Debug, Clone, Copy)]
pub struct ArchHeader {
    pub version: u32,
    pub name_table_size: u32,
    pub folder_count: u32,
    pub file_count: u32,
}

impl ArchHeader {
    pub fn parse(bytes: &[u8]) -> Result<Self, AssetError> {
        if bytes.len() < 48 {
            return Err(AssetError::Truncated("Arch00 header"));
        }
        if bytes[0..4] != LTAR_MAGIC {
            return Err(AssetError::Invalid("LTAR magic"));
        }
        Ok(Self {
            version: u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            name_table_size: u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            folder_count: u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
            file_count: u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
        })
    }

    pub fn probe(path: impl AsRef<Path>) -> Result<Self, AssetError> {
        let mut file = File::open(path)?;
        let mut header = [0u8; 48];
        file.read_exact(&mut header)?;
        Self::parse(&header)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    Raw,
    Zlib,
}

#[derive(Debug, Clone)]
pub struct ArchFile {
    pub path: String,
    pub offset: u64,
    pub compressed_size: u32,
    pub raw_size: u32,
    pub compression: Compression,
}

#[derive(Debug)]
pub struct Arch00<R> {
    reader: R,
    files: Vec<ArchFile>,
}

impl Arch00<File> {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AssetError> {
        Self::from_reader(File::open(path)?)
    }
}

impl<R: Read + Seek> Arch00<R> {
    pub fn from_reader(mut reader: R) -> Result<Self, AssetError> {
        let mut header = [0u8; 48];
        reader.read_exact(&mut header)?;
        if header[0..4] != LTAR_MAGIC {
            return Err(AssetError::Invalid("LTAR magic"));
        }
        let name_table_size = u32::from_le_bytes(header[8..12].try_into().unwrap()) as usize;
        let folder_count = u32::from_le_bytes(header[12..16].try_into().unwrap()) as usize;
        let file_count = u32::from_le_bytes(header[16..20].try_into().unwrap()) as usize;

        let mut name_table = vec![0u8; name_table_size];
        reader.read_exact(&mut name_table)?;

        let mut file_bytes = vec![0u8; file_count * 32];
        reader.read_exact(&mut file_bytes)?;
        let mut folder_bytes = vec![0u8; folder_count * 16];
        reader.read_exact(&mut folder_bytes)?;

        let mut raw_files = Vec::with_capacity(file_count);
        for i in 0..file_count {
            let e = &file_bytes[i * 32..(i + 1) * 32];
            let name_off = u32::from_le_bytes(e[0..4].try_into().unwrap()) as usize;
            let offset = u32::from_le_bytes(e[4..8].try_into().unwrap()) as u64;
            let comp_size = u32::from_le_bytes(e[12..16].try_into().unwrap());
            let raw_size = u32::from_le_bytes(e[20..24].try_into().unwrap());
            let method = u32::from_le_bytes(e[28..32].try_into().unwrap());
            let name = read_cstring(&name_table, name_off)?.replace('\\', "/");
            let compression = match method {
                COMP_RAW => Compression::Raw,
                COMP_ZLIB => Compression::Zlib,
                other => return Err(AssetError::Compression(other)),
            };
            raw_files.push(ArchFile {
                path: name,
                offset,
                compressed_size: comp_size,
                raw_size,
                compression,
            });
        }

        let mut files = Vec::new();
        let mut file_index = 0usize;
        for i in 0..folder_count {
            let e = &folder_bytes[i * 16..(i + 1) * 16];
            let name_off = u32::from_le_bytes(e[0..4].try_into().unwrap()) as usize;
            let count = u32::from_le_bytes(e[12..16].try_into().unwrap()) as usize;
            let folder = read_cstring(&name_table, name_off)?.replace('\\', "/");
            for _ in 0..count {
                if file_index >= raw_files.len() {
                    return Err(AssetError::Truncated("folder file list"));
                }
                let mut f = raw_files[file_index].clone();
                file_index += 1;
                f.path = if folder.is_empty() {
                    f.path
                } else {
                    format!("{folder}/{}", f.path)
                };
                files.push(f);
            }
        }
        // Files not claimed by a folder still exist (some archives do this).
        while file_index < raw_files.len() {
            files.push(raw_files[file_index].clone());
            file_index += 1;
        }

        Ok(Self { reader, files })
    }

    pub fn files(&self) -> impl Iterator<Item = &ArchFile> {
        self.files.iter()
    }

    pub fn read(&mut self, path: &str) -> Result<Vec<u8>, AssetError> {
        let want = path.replace('\\', "/");
        let file = self
            .files
            .iter()
            .find(|f| f.path.eq_ignore_ascii_case(&want))
            .cloned()
            .ok_or_else(|| AssetError::NotFound(path.to_string()))?;
        self.reader.seek(SeekFrom::Start(file.offset))?;
        match file.compression {
            Compression::Raw => {
                let mut buf = vec![0u8; file.raw_size as usize];
                self.reader.read_exact(&mut buf)?;
                Ok(buf)
            }
            Compression::Zlib => {
                let mut compressed = vec![0u8; file.compressed_size as usize];
                self.reader.read_exact(&mut compressed)?;
                decompress_payload(&compressed, file.raw_size)
            }
        }
    }
}

fn decompress_payload(compressed: &[u8], raw_size: u32) -> Result<Vec<u8>, AssetError> {
    if let Ok(out) = try_zlib(compressed, raw_size) {
        return Ok(out);
    }
    decompress_fear_blocks(compressed, raw_size)
}

fn try_zlib(compressed: &[u8], raw_size: u32) -> Result<Vec<u8>, AssetError> {
    let mut dec = Decompress::new(true);
    let mut out = vec![0u8; raw_size as usize];
    let status = dec.decompress(compressed, &mut out, flate2::FlushDecompress::Finish)?;
    if matches!(
        status,
        flate2::Status::Ok | flate2::Status::StreamEnd | flate2::Status::BufError
    ) && dec.total_out() as u32 == raw_size
    {
        Ok(out)
    } else {
        Err(AssetError::Invalid("zlib size"))
    }
}

/// F.E.A.R. 2-style block zlib: repeating `{comp_size, raw_size, payload, pad4}`.
fn decompress_fear_blocks(compressed: &[u8], raw_size: u32) -> Result<Vec<u8>, AssetError> {
    let mut cur = Cursor::new(compressed);
    let mut out = Vec::with_capacity(raw_size as usize);
    while (cur.position() as usize) + 8 <= compressed.len() {
        let mut hdr = [0u8; 8];
        cur.read_exact(&mut hdr)?;
        let block_comp = u32::from_le_bytes(hdr[0..4].try_into().unwrap()) as usize;
        let block_raw = u32::from_le_bytes(hdr[4..8].try_into().unwrap()) as usize;
        if (cur.position() as usize) + block_comp > compressed.len() {
            return Err(AssetError::Truncated("zlib block"));
        }
        let mut payload = vec![0u8; block_comp];
        cur.read_exact(&mut payload)?;
        let pad = (4 - (block_comp % 4)) % 4;
        cur.seek(SeekFrom::Current(pad as i64))?;
        if block_comp == block_raw {
            out.extend_from_slice(&payload);
        } else {
            let mut dec = Decompress::new(false);
            let mut chunk = vec![0u8; block_raw];
            dec.decompress(&payload, &mut chunk, flate2::FlushDecompress::Finish)?;
            out.extend_from_slice(&chunk[..dec.total_out() as usize]);
        }
    }
    if raw_size != 0 && out.len() != raw_size as usize {
        return Err(AssetError::Invalid("block zlib size"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn build_raw_archive(folder: &str, name: &str, payload: &[u8]) -> Vec<u8> {
        let mut names = Vec::new();
        let folder_off = 0u32;
        names.extend_from_slice(folder.as_bytes());
        names.push(0);
        let file_off = names.len() as u32;
        names.extend_from_slice(name.as_bytes());
        names.push(0);

        let mut header = [0u8; 48];
        header[0..4].copy_from_slice(&LTAR_MAGIC);
        header[4..8].copy_from_slice(&1u32.to_le_bytes());
        header[8..12].copy_from_slice(&(names.len() as u32).to_le_bytes());
        header[12..16].copy_from_slice(&1u32.to_le_bytes());
        header[16..20].copy_from_slice(&1u32.to_le_bytes());

        let data_offset = 48 + names.len() + 32 + 16;
        let mut file_entry = [0u8; 32];
        file_entry[0..4].copy_from_slice(&file_off.to_le_bytes());
        file_entry[4..8].copy_from_slice(&(data_offset as u32).to_le_bytes());
        file_entry[12..16].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        file_entry[20..24].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        file_entry[28..32].copy_from_slice(&COMP_RAW.to_le_bytes());

        let mut folder_entry = [0u8; 16];
        folder_entry[0..4].copy_from_slice(&folder_off.to_le_bytes());
        folder_entry[12..16].copy_from_slice(&1u32.to_le_bytes());

        let mut out = Vec::new();
        out.extend_from_slice(&header);
        out.extend_from_slice(&names);
        out.extend_from_slice(&file_entry);
        out.extend_from_slice(&folder_entry);
        out.extend_from_slice(payload);
        out
    }

    #[test]
    fn roundtrip_raw() {
        let bytes = build_raw_archive("Worlds", "Intro.World00p", b"world-bytes");
        let mut arch = Arch00::from_reader(Cursor::new(bytes)).unwrap();
        let files: Vec<_> = arch.files().map(|f| f.path.clone()).collect();
        assert_eq!(files, ["Worlds/Intro.World00p"]);
        assert_eq!(arch.read("Worlds/Intro.World00p").unwrap(), b"world-bytes");
    }
}
