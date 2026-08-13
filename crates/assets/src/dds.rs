//! Retail DDS (D3D9). Upload BC1/BC2/BC3 and BGRA8 as-is; do not decompress on the CPU.

use crate::AssetError;

const DDS_MAGIC: u32 = 0x2053_4444; // 'DDS '
const DXT1: u32 = 0x3154_5844;
const DXT3: u32 = 0x3354_5844;
const DXT5: u32 = 0x3554_5844;
const DDPF_ALPHAPIXELS: u32 = 0x1;
const DDPF_RGB: u32 = 0x40;
const MASK_R: u32 = 0x00FF_0000;
const MASK_G: u32 = 0x0000_FF00;
const MASK_B: u32 = 0x0000_00FF;
const MASK_A: u32 = 0xFF00_0000;
const DDSCAPS2_CUBEMAP: u32 = 0x200;
/// Все шесть граней: +X −X +Y −Y +Z −Z. Без полного набора небо нельзя семплить.
const DDSCAPS2_CUBEMAP_ALL_FACES: u32 = 0xFC00;
const HEADER_SIZE: usize = 128; // magic + 124-byte DDS_HEADER
/// Порядок граней D3D9 / Vulkan cube: +X, −X, +Y, −Y, +Z, −Z.
pub const CUBEMAP_FACES: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DdsFormat {
    Bc1,
    Bc2,
    Bc3,
    Bgra8,
}

#[derive(Debug, Clone)]
pub struct DdsImage {
    pub width: u32,
    pub height: u32,
    pub mip_count: u32,
    pub format: DdsFormat,
    pub bytes: Vec<u8>,
}

struct DdsHeader {
    width: u32,
    height: u32,
    mip_count: u32,
    format: DdsFormat,
    is_cubemap: bool,
    all_faces: bool,
}

fn parse_header(bytes: &[u8]) -> Result<DdsHeader, AssetError> {
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
    let pf_flags = u32::from_le_bytes(bytes[80..84].try_into().unwrap());
    let four_cc = u32::from_le_bytes(bytes[84..88].try_into().unwrap());
    let rgb_bits = u32::from_le_bytes(bytes[88..92].try_into().unwrap());
    let mask_r = u32::from_le_bytes(bytes[92..96].try_into().unwrap());
    let mask_g = u32::from_le_bytes(bytes[96..100].try_into().unwrap());
    let mask_b = u32::from_le_bytes(bytes[100..104].try_into().unwrap());
    let mask_a = u32::from_le_bytes(bytes[104..108].try_into().unwrap());
    let caps2 = u32::from_le_bytes(bytes[112..116].try_into().unwrap());
    let format = match four_cc {
        DXT1 => DdsFormat::Bc1,
        DXT3 => DdsFormat::Bc2,
        DXT5 => DdsFormat::Bc3,
        _ if pf_flags & (DDPF_RGB | DDPF_ALPHAPIXELS) == (DDPF_RGB | DDPF_ALPHAPIXELS)
            && rgb_bits == 32
            && mask_r == MASK_R
            && mask_g == MASK_G
            && mask_b == MASK_B
            && mask_a == MASK_A =>
        {
            DdsFormat::Bgra8
        }
        other => return Err(AssetError::UnsupportedDds(other)),
    };
    if width == 0 || height == 0 {
        return Err(AssetError::Invalid("DDS dimensions"));
    }
    Ok(DdsHeader {
        width,
        height,
        mip_count: mip_raw.max(1),
        format,
        is_cubemap: caps2 & DDSCAPS2_CUBEMAP != 0,
        all_faces: caps2 & DDSCAPS2_CUBEMAP_ALL_FACES == DDSCAPS2_CUBEMAP_ALL_FACES,
    })
}

impl DdsImage {
    pub fn parse(bytes: &[u8]) -> Result<Self, AssetError> {
        let header = parse_header(bytes)?;
        if header.is_cubemap {
            return Err(AssetError::Invalid("DDS cubemap"));
        }
        let expected = mip_chain_bytes(header.width, header.height, header.mip_count, header.format);
        let payload = bytes.get(HEADER_SIZE..).ok_or(AssetError::Truncated("DDS payload"))?;
        if payload.len() < expected {
            return Err(AssetError::Truncated("DDS payload"));
        }
        Ok(Self {
            width: header.width,
            height: header.height,
            mip_count: header.mip_count,
            format: header.format,
            bytes: payload[..expected].to_vec(),
        })
    }

    pub fn block_bytes(self: &Self) -> u32 {
        match self.format {
            DdsFormat::Bc1 => 8,
            DdsFormat::Bc2 | DdsFormat::Bc3 => 16,
            DdsFormat::Bgra8 => 4,
        }
    }
}

/// Cubemap DDS (небо). Грани идут подряд: каждая — полная цепочка mip, как в D3D9.
#[derive(Debug, Clone)]
pub struct DdsCubemap {
    pub width: u32,
    pub height: u32,
    pub mip_count: u32,
    pub format: DdsFormat,
    pub bytes: Vec<u8>,
}

impl DdsCubemap {
    pub fn parse(bytes: &[u8]) -> Result<Self, AssetError> {
        let header = parse_header(bytes)?;
        if !header.is_cubemap {
            return Err(AssetError::Invalid("DDS not cubemap"));
        }
        if !header.all_faces {
            return Err(AssetError::Invalid("DDS cubemap faces"));
        }
        let face = mip_chain_bytes(header.width, header.height, header.mip_count, header.format);
        let expected = face * CUBEMAP_FACES;
        let payload = bytes.get(HEADER_SIZE..).ok_or(AssetError::Truncated("DDS cubemap"))?;
        if payload.len() < expected {
            return Err(AssetError::Truncated("DDS cubemap"));
        }
        Ok(Self {
            width: header.width,
            height: header.height,
            mip_count: header.mip_count,
            format: header.format,
            bytes: payload[..expected].to_vec(),
        })
    }

    pub fn face_bytes(&self) -> usize {
        mip_chain_bytes(self.width, self.height, self.mip_count, self.format)
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
    match format {
        DdsFormat::Bgra8 => width.max(1) as usize * height.max(1) as usize * 4,
        DdsFormat::Bc1 | DdsFormat::Bc2 | DdsFormat::Bc3 => {
            let blocks_x = (width.max(1) + 3) / 4;
            let blocks_y = (height.max(1) + 3) / 4;
            let block = match format {
                DdsFormat::Bc1 => 8usize,
                DdsFormat::Bc2 | DdsFormat::Bc3 => 16,
                DdsFormat::Bgra8 => unreachable!(),
            };
            blocks_x as usize * blocks_y as usize * block
        }
    }
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

    #[test]
    fn parse_bgra8_1x1_normal() {
        let mut bytes = vec![0u8; HEADER_SIZE + 4];
        bytes[0..4].copy_from_slice(&DDS_MAGIC.to_le_bytes());
        bytes[4..8].copy_from_slice(&124u32.to_le_bytes());
        bytes[12..16].copy_from_slice(&1u32.to_le_bytes());
        bytes[16..20].copy_from_slice(&1u32.to_le_bytes());
        bytes[28..32].copy_from_slice(&0u32.to_le_bytes());
        bytes[80..84].copy_from_slice(&(DDPF_RGB | DDPF_ALPHAPIXELS).to_le_bytes());
        bytes[88..92].copy_from_slice(&32u32.to_le_bytes());
        bytes[92..96].copy_from_slice(&MASK_R.to_le_bytes());
        bytes[96..100].copy_from_slice(&MASK_G.to_le_bytes());
        bytes[100..104].copy_from_slice(&MASK_B.to_le_bytes());
        bytes[104..108].copy_from_slice(&MASK_A.to_le_bytes());
        bytes[128..132].copy_from_slice(&[254, 127, 127, 30]);
        let img = DdsImage::parse(&bytes).unwrap();
        assert_eq!(img.format, DdsFormat::Bgra8);
        assert_eq!(img.mip_count, 1);
        assert_eq!(img.bytes, [254, 127, 127, 30]);
    }

    #[test]
    fn rejects_cubemap_as_2d() {
        let mut bytes = vec![0u8; HEADER_SIZE + 8];
        bytes[0..4].copy_from_slice(&DDS_MAGIC.to_le_bytes());
        bytes[4..8].copy_from_slice(&124u32.to_le_bytes());
        bytes[12..16].copy_from_slice(&4u32.to_le_bytes());
        bytes[16..20].copy_from_slice(&4u32.to_le_bytes());
        bytes[28..32].copy_from_slice(&1u32.to_le_bytes());
        bytes[84..88].copy_from_slice(&DXT1.to_le_bytes());
        bytes[112..116].copy_from_slice(&DDSCAPS2_CUBEMAP.to_le_bytes());
        let err = DdsImage::parse(&bytes).unwrap_err();
        assert!(
            matches!(err, AssetError::Invalid("DDS cubemap")),
            "got {err}"
        );
    }

    #[test]
    fn parse_cubemap_six_dxt1_faces() {
        let face = mip_chain_bytes(4, 4, 1, DdsFormat::Bc1);
        let mut bytes = vec![0u8; HEADER_SIZE + face * CUBEMAP_FACES];
        bytes[0..4].copy_from_slice(&DDS_MAGIC.to_le_bytes());
        bytes[4..8].copy_from_slice(&124u32.to_le_bytes());
        bytes[12..16].copy_from_slice(&4u32.to_le_bytes());
        bytes[16..20].copy_from_slice(&4u32.to_le_bytes());
        bytes[28..32].copy_from_slice(&1u32.to_le_bytes());
        bytes[84..88].copy_from_slice(&DXT1.to_le_bytes());
        let caps2 = DDSCAPS2_CUBEMAP | DDSCAPS2_CUBEMAP_ALL_FACES;
        bytes[112..116].copy_from_slice(&caps2.to_le_bytes());
        // Маркер последней грани, чтобы проверить, что читаем все шесть, а не одну.
        bytes[HEADER_SIZE + face * 5] = 0xAB;
        let cube = DdsCubemap::parse(&bytes).unwrap();
        assert_eq!(cube.width, 4);
        assert_eq!(cube.height, 4);
        assert_eq!(cube.format, DdsFormat::Bc1);
        assert_eq!(cube.bytes.len(), face * CUBEMAP_FACES);
        assert_eq!(cube.bytes[face * 5], 0xAB);
        assert!(DdsImage::parse(&bytes).is_err());
    }

    #[test]
    fn cubemap_without_all_faces_is_rejected() {
        let mut bytes = vec![0u8; HEADER_SIZE + 8];
        bytes[0..4].copy_from_slice(&DDS_MAGIC.to_le_bytes());
        bytes[4..8].copy_from_slice(&124u32.to_le_bytes());
        bytes[12..16].copy_from_slice(&4u32.to_le_bytes());
        bytes[16..20].copy_from_slice(&4u32.to_le_bytes());
        bytes[28..32].copy_from_slice(&1u32.to_le_bytes());
        bytes[84..88].copy_from_slice(&DXT1.to_le_bytes());
        bytes[112..116].copy_from_slice(&DDSCAPS2_CUBEMAP.to_le_bytes());
        let err = DdsCubemap::parse(&bytes).unwrap_err();
        assert!(
            matches!(err, AssetError::Invalid("DDS cubemap faces")),
            "got {err}"
        );
    }
}
