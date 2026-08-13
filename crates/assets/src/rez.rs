//! Classic LithTech REZ (RezMgr Version 1). Used by older titles and some F.E.A.R. packs.
//! Spec: Xentax / Reverse Engineering Wiki, little-endian.

use crate::AssetError;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const TEXT_HEADER_LEN: usize = 127;
const SIG: &[u8] = b"RezMgr Version 1 Copyright (C) 1995 MONOLITH INC.";

#[derive(Debug, Clone)]
pub struct RezEntry {
    pub path: String,
    pub offset: u64,
    pub size: u32,
    pub id: u32,
    pub kind: String,
}

#[derive(Debug)]
pub struct RezArchive<R> {
    reader: R,
    entries: Vec<RezEntry>,
}

impl RezArchive<File> {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AssetError> {
        Self::from_reader(File::open(path)?)
    }
}

impl<R: Read + Seek> RezArchive<R> {
    pub fn from_reader(mut reader: R) -> Result<Self, AssetError> {
        let mut text = [0u8; TEXT_HEADER_LEN];
        reader.read_exact(&mut text)?;
        if text[0] != 13 || text[1] != 10 || !text[2..].starts_with(SIG) {
            return Err(AssetError::Invalid("REZ signature"));
        }
        let version = crate::read_u32(&mut reader)?;
        if version != 1 {
            return Err(AssetError::Invalid("REZ version"));
        }
        let dir_offset = crate::read_u32(&mut reader)? as u64;
        let dir_size = crate::read_u32(&mut reader)? as usize;
        // skip remaining header fields (times, largest-name sizes, sorted flag)
        reader.seek(SeekFrom::Start(dir_offset))?;
        let dir = crate::read_exact_vec(&mut reader, dir_size)?;
        let mut entries = Vec::new();
        parse_dir(&dir, "", &mut entries)?;
        Ok(Self { reader, entries })
    }

    pub fn entries(&self) -> impl Iterator<Item = &RezEntry> {
        self.entries.iter()
    }

    pub fn read(&mut self, path: &str) -> Result<Vec<u8>, AssetError> {
        let want = path.replace('\\', "/");
        let entry = self
            .entries
            .iter()
            .find(|e| e.path.eq_ignore_ascii_case(&want))
            .cloned()
            .ok_or_else(|| AssetError::NotFound(path.to_string()))?;
        self.reader.seek(SeekFrom::Start(entry.offset))?;
        crate::read_exact_vec(&mut self.reader, entry.size as usize)
    }
}

fn parse_dir(dir: &[u8], prefix: &str, out: &mut Vec<RezEntry>) -> Result<(), AssetError> {
    let mut i = 0usize;
    while i + 4 <= dir.len() {
        let kind = u32::from_le_bytes(dir[i..i + 4].try_into().unwrap());
        i += 4;
        match kind {
            1 => {
                if i + 12 > dir.len() {
                    return Err(AssetError::Truncated("REZ dir"));
                }
                let _off = u32::from_le_bytes(dir[i..i + 4].try_into().unwrap());
                i += 4;
                let _len = u32::from_le_bytes(dir[i..i + 4].try_into().unwrap());
                i += 4;
                i += 4; // time
                let name = read_z(&dir, &mut i, 1)?;
                // Nested directories are stored as separate dir blobs at `_off`.
                // We only index files from the blobs we are given; callers who
                // need recursion pass each blob. Root listing walks files here.
                let _ = (prefix, name);
            }
            0 => {
                if i + 16 > dir.len() {
                    return Err(AssetError::Truncated("REZ file"));
                }
                let offset = u32::from_le_bytes(dir[i..i + 4].try_into().unwrap()) as u64;
                i += 4;
                let size = u32::from_le_bytes(dir[i..i + 4].try_into().unwrap());
                i += 4;
                i += 4; // time
                let id = u32::from_le_bytes(dir[i..i + 4].try_into().unwrap());
                i += 4;
                let ext = read_fixed_ext(&dir, &mut i)?;
                i += 4; // null
                let name = read_z(&dir, &mut i, 2)?;
                let path = if prefix.is_empty() {
                    if ext.is_empty() {
                        name
                    } else {
                        format!("{name}.{ext}")
                    }
                } else if ext.is_empty() {
                    format!("{prefix}/{name}")
                } else {
                    format!("{prefix}/{name}.{ext}")
                };
                out.push(RezEntry {
                    path,
                    offset,
                    size,
                    id,
                    kind: ext,
                });
            }
            _ => return Err(AssetError::Invalid("REZ entry type")),
        }
    }
    Ok(())
}

fn read_z(buf: &[u8], i: &mut usize, nulls: usize) -> Result<String, AssetError> {
    let start = *i;
    while *i < buf.len() && buf[*i] != 0 {
        *i += 1;
    }
    let s = std::str::from_utf8(&buf[start..*i]).map_err(|_| AssetError::Utf8)?;
    *i += nulls;
    if *i > buf.len() {
        return Err(AssetError::Truncated("REZ name"));
    }
    Ok(s.to_string())
}

fn read_fixed_ext(buf: &[u8], i: &mut usize) -> Result<String, AssetError> {
    if *i + 4 > buf.len() {
        return Err(AssetError::Truncated("REZ ext"));
    }
    let raw = &buf[*i..*i + 4];
    *i += 4;
    // Stored reversed (e.g. "DTX\0" as "XTD\0") in classic RezMgr.
    let mut chars: Vec<u8> = raw.iter().copied().filter(|&b| b != 0).collect();
    chars.reverse();
    Ok(String::from_utf8_lossy(&chars).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn pad60(s: &str) -> [u8; 60] {
        let mut b = [0u8; 60];
        b[..s.len()].copy_from_slice(s.as_bytes());
        b
    }

    #[test]
    fn roundtrip_file() {
        let payload = b"hello-rez";
        let mut dir = Vec::new();
        dir.extend_from_slice(&0u32.to_le_bytes()); // file
        dir.extend_from_slice(&171u32.to_le_bytes()); // offset: after 127+44 header
        dir.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        dir.extend_from_slice(&0u32.to_le_bytes()); // time
        dir.extend_from_slice(&1u32.to_le_bytes()); // id
        dir.extend_from_slice(b"txtd"); // reversed "dtx" plus leftover — actually 4 bytes "txt\0" reversed is "txt\0"?
                                        // "txt" reversed into 4 bytes: store as b"txt\0" then we reverse non-null → "txt". Wait our reader reverses.
                                        // Store reversed: "txt" → write 't','x','t',0 then reverse filter → t,x,t reversed = t,x,t. Hmm.
                                        // Reader: take 4 bytes, filter zeros, reverse. So to get "dtx" we store "xtd\0".
        dir.truncate(dir.len() - 4);
        dir.extend_from_slice(b"xtd\0");
        dir.extend_from_slice(&0u32.to_le_bytes());
        dir.extend_from_slice(b"sample\0\0");

        let header_len = 127 + 11 * 4;
        let payload_off = header_len as u32;
        let dir_off = payload_off + payload.len() as u32;
        dir[4..8].copy_from_slice(&payload_off.to_le_bytes());

        let mut file = Vec::new();
        file.push(13);
        file.push(10);
        file.extend_from_slice(&pad60("RezMgr Version 1 Copyright (C) 1995 MONOLITH INC."));
        file.push(13);
        file.push(10);
        file.extend_from_slice(&pad60("LithTech Resource File"));
        file.push(13);
        file.push(10);
        file.push(26);
        file.extend_from_slice(&1u32.to_le_bytes());
        file.extend_from_slice(&dir_off.to_le_bytes());
        file.extend_from_slice(&(dir.len() as u32).to_le_bytes());
        file.extend_from_slice(&[0u8; 32]); // remaining 8 u32s
        assert_eq!(file.len(), header_len);
        file.extend_from_slice(payload);
        file.extend_from_slice(&dir);

        let mut rez = RezArchive::from_reader(Cursor::new(file)).unwrap();
        let names: Vec<_> = rez.entries().map(|e| e.path.clone()).collect();
        assert_eq!(names, ["sample.dtx"]);
        assert_eq!(rez.read("sample.dtx").unwrap(), b"hello-rez");
    }
}
