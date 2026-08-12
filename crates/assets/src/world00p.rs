//! Packed Jupiter EX world (`World00p`). F.E.A.R. version 113, XOR magic 399.

use crate::AssetError;
use glam::Vec3;
use std::io::Cursor;
use std::io::Read;

pub const FEAR_WORLD_VERSION: u32 = 113;
pub const FEAR_WORLD_MAGIC: u32 = 399; // 113 + b'F'+b'E'+b'A'+b'R'

#[derive(Debug, Clone)]
pub struct WorldHeader {
    pub version: u32,
    pub render_section_offset: u32,
    pub sector_section_offset: u32,
    pub object_section_offset: u32,
    pub blinddata_section_offset: u32,
    pub min: Vec3,
    pub max: Vec3,
    pub offset: Vec3,
}

impl WorldHeader {
    pub fn parse(bytes: &[u8]) -> Result<Self, AssetError> {
        if bytes.len() < 56 {
            return Err(AssetError::Truncated("World00p header"));
        }
        let mut c = Cursor::new(bytes);
        let version = crate::read_u32(&mut c)?;
        if version != FEAR_WORLD_VERSION {
            return Err(AssetError::WorldVersion(version));
        }
        Ok(Self {
            version,
            render_section_offset: crate::read_u32(&mut c)?,
            sector_section_offset: crate::read_u32(&mut c)?,
            object_section_offset: crate::read_u32(&mut c)?,
            blinddata_section_offset: crate::read_u32(&mut c)?,
            min: read_vec3(&mut c)?,
            max: read_vec3(&mut c)?,
            offset: read_vec3(&mut c)?,
        })
    }

    pub fn decode_count(raw: u32) -> u32 {
        raw ^ FEAR_WORLD_MAGIC
    }
}

fn read_vec3(c: &mut Cursor<&[u8]>) -> Result<Vec3, AssetError> {
    let mut buf = [0u8; 12];
    c.read_exact(&mut buf)?;
    Ok(Vec3::new(
        f32::from_le_bytes(buf[0..4].try_into().unwrap()),
        f32::from_le_bytes(buf[4..8].try_into().unwrap()),
        f32::from_le_bytes(buf[8..12].try_into().unwrap()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_header() {
        let mut bytes = vec![0u8; 56];
        bytes[0..4].copy_from_slice(&113u32.to_le_bytes());
        bytes[4..8].copy_from_slice(&100u32.to_le_bytes());
        let h = WorldHeader::parse(&bytes).unwrap();
        assert_eq!(h.render_section_offset, 100);
        assert_eq!(WorldHeader::decode_count(399 ^ 12), 12);
    }
}
