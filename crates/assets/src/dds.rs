//! Retail DDS (D3D9). Upload BC1/BC2/BC3 as-is; do not decompress on the CPU.

use crate::AssetError;

const DDS_MAGIC: u32 = 0x2053_4444; // 'DDS '
const DXT1: u32 = 0x3154_5844;
const DXT3: u32 = 0x3354_5844;
const DXT5: u32 = 0x3554_5844;
const HEADER_SIZE: usize = 128; // magic + 124-byte DDS_HEADER

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DdsFormat {
    Bc1,
    Bc2,
    Bc3,
}

#[derive(Debug, Clone)]
pub struct DdsImage {
    pub width: u32,
    pub height: u32,
    pub mip_count: u32,
    pub format: DdsFormat,
    pub bytes: Vec<u8>,
}

impl DdsImage {
    pub fn parse(bytes: &[u8]) -> Result<Self, AssetError> {
        if bytes.len() < HEADER_SIZE {
            return Err(AssetError::Truncated("DDS header"));
        }
        let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        if magic != DDS_MAGIC {
            return Err(AssetError::Invalid("DDS magic"));
        }
        let header_size = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        if header_size != 124 {
            return Err(AssetError::Invalid("DDS header size"));
        }
        let height = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
        let width = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
        let mip_raw = u32::from_le_bytes(bytes[28..32].try_into().unwrap());
        let four_cc = u32::from_le_bytes(bytes[84..88].try_into().unwrap());
        let format = match four_cc {
            DXT1 => DdsFormat::Bc1,
            DXT3 => DdsFormat::Bc2,
            DXT5 => DdsFormat::Bc3,
            _ => return Err(AssetError::UnsupportedDds(four_cc)),
        };
        if width == 0 || height == 0 {
            return Err(AssetError::Invalid("DDS dimensions"));
        }
        let mip_count = mip_raw.max(1);
        let payload = bytes[HEADER_SIZE..].to_vec();
        let expected = mip_chain_bytes(width, height, mip_count, format);
        if payload.len() < expected {
            return Err(AssetError::Truncated("DDS payload"));
        }
        Ok(Self {
            width,
            height,
            mip_count,
            format,
            bytes: payload[..expected].to_vec(),
        })
    }

    pub fn block_bytes(self: &Self) -> u32 {
        match self.format {
            DdsFormat::Bc1 => 8,
            DdsFormat::Bc2 | DdsFormat::Bc3 => 16,
        }
    }
}

pub fn mip_chain_bytes(mut width: u32, mut height: u32, mips: u32, format: DdsFormat) -> usize {
    let mut total = 0usize;
    for _ in 0..mips {
        total += mip_bytes(width, height, format);
        width = (width / 2).max(1);
        height = (height / 2).max(1);
    }
    total
}

pub fn mip_bytes(width: u32, height: u32, format: DdsFormat) -> usize {
    let blocks_x = (width.max(1) + 3) / 4;
    let blocks_y = (height.max(1) + 3) / 4;
    let block = match format {
        DdsFormat::Bc1 => 8usize,
        DdsFormat::Bc2 | DdsFormat::Bc3 => 16,
    };
    blocks_x as usize * blocks_y as usize * block
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_dxt1_4x4() {
        let mut bytes = vec![0u8; HEADER_SIZE + 8];
        bytes[0..4].copy_from_slice(&DDS_MAGIC.to_le_bytes());
        bytes[4..8].copy_from_slice(&124u32.to_le_bytes());
        bytes[12..16].copy_from_slice(&4u32.to_le_bytes());
        bytes[16..20].copy_from_slice(&4u32.to_le_bytes());
        bytes[28..32].copy_from_slice(&1u32.to_le_bytes());
        bytes[84..88].copy_from_slice(&DXT1.to_le_bytes());
        let img = DdsImage::parse(&bytes).unwrap();
        assert_eq!(img.width, 4);
        assert_eq!(img.height, 4);
        assert_eq!(img.format, DdsFormat::Bc1);
        assert_eq!(img.bytes.len(), 8);
    }
}
