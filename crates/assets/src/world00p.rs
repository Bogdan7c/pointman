//! Packed Jupiter EX world (`World00p`). F.E.A.R. version 113, XOR magic 399.

use crate::AssetError;
use glam::Vec3;
use std::io::{Cursor, Read, Seek, SeekFrom};

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

#[derive(Debug, Clone)]
pub struct WorldVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub tangent: [f32; 3],
    pub binormal: [f32; 3],
}

#[derive(Debug, Clone)]
pub struct WorldSurface {
    pub vertices: Vec<WorldVertex>,
    pub indices: Vec<u32>,
    pub material: String,
}

#[derive(Debug, Clone)]
pub struct WorldRender {
    pub header: WorldHeader,
    pub surfaces: Vec<WorldSurface>,
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

    pub fn extent(&self) -> Vec3 {
        self.max - self.min
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }
}

impl WorldRender {
    pub fn parse(bytes: &[u8]) -> Result<Self, AssetError> {
        let header = WorldHeader::parse(bytes)?;
        let mut c = Cursor::new(bytes);
        c.seek(SeekFrom::Start(u64::from(header.render_section_offset)))?;

        let mut section = [0u32; 10];
        for slot in &mut section {
            *slot = crate::read_u32(&mut c)?;
        }
        let _branch_count = section[0];
        let render_mesh_count = section[3];
        if render_mesh_count != 1 {
            return Err(AssetError::Invalid("render_mesh_count"));
        }

        let unknown_id = crate::read_u32(&mut c)?;
        if unknown_id != 0 {
            return Err(AssetError::Invalid("render mesh id"));
        }
        let surface_count = crate::read_u32(&mut c)?;
        let material_count = crate::read_u32(&mut c)?;
        let vertex_block_size = crate::read_u32(&mut c)? as usize;
        let triangles_block_size = crate::read_u32(&mut c)? as usize;

        let vertex_blob = read_bytes(&mut c, vertex_block_size)?;
        let index_blob = read_bytes(&mut c, triangles_block_size)?;

        let def_count = crate::read_u32(&mut c)? as usize;
        let mut defs = Vec::with_capacity(def_count);
        for _ in 0..def_count {
            defs.push(read_vertex_def(&mut c)?);
        }

        let listed_surfaces = crate::read_u32(&mut c)? as usize;
        if listed_surfaces != surface_count as usize {
            log::warn!(
                "World00p surface count mismatch header {surface_count} vs list {listed_surfaces}"
            );
        }
        let mut raw_surfaces = Vec::with_capacity(listed_surfaces);
        for _ in 0..listed_surfaces {
            raw_surfaces.push(read_surface_header(&mut c)?);
        }

        let mut materials = Vec::with_capacity(material_count as usize);
        for _ in 0..material_count {
            materials.push(read_lt_string(&mut c).unwrap_or_default());
        }

        let mut surfaces = Vec::new();
        for raw in raw_surfaces {
            let name = materials
                .get(raw.material_id as usize)
                .cloned()
                .unwrap_or_default();
            if name.to_ascii_lowercase().contains("shadowvolume") {
                continue;
            }
            let def = defs
                .get(raw.pack_type_id as usize)
                .ok_or(AssetError::Invalid("pack_type_id"))?;
            let verts = match decode_surface_vertices(&vertex_blob, &raw, def) {
                Ok(v) => v,
                Err(err) => {
                    log::warn!("skip surface (verts): {err}");
                    continue;
                }
            };
            let indices = match decode_surface_indices(&index_blob, &raw, verts.len()) {
                Ok(i) => i,
                Err(err) => {
                    log::warn!("skip surface (indices): {err}");
                    continue;
                }
            };
            if verts.is_empty() || indices.is_empty() {
                continue;
            }
            surfaces.push(WorldSurface {
                vertices: verts,
                indices,
                material: name,
            });
        }

        Ok(Self { header, surfaces })
    }

    pub fn flatten(&self) -> (Vec<WorldVertex>, Vec<u32>, Vec<SurfaceDraw>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut draws = Vec::new();
        for surface in &self.surfaces {
            let base = vertices.len() as u32;
            let first_index = indices.len() as u32;
            vertices.extend_from_slice(&surface.vertices);
            indices.extend(surface.indices.iter().map(|i| i + base));
            draws.push(SurfaceDraw {
                first_index,
                index_count: surface.indices.len() as u32,
                color: material_color(&surface.material),
                material: surface.material.clone(),
            });
        }
        (vertices, indices, draws)
    }
}

#[derive(Debug, Clone)]
pub struct SurfaceDraw {
    pub first_index: u32,
    pub index_count: u32,
    pub color: [f32; 4],
    pub material: String,
}

struct RawSurface {
    vertices_start: u32,
    vertices_count: u32,
    vertex_stride: u32,
    indices_start: u32,
    indices_base: u32,
    triangle_count: u32,
    material_id: u32,
    pack_type_id: u32,
}

struct VertexProp {
    /// Byte offset inside the vertex (`b` in the Jupiter EX pack type).
    offset: u16,
    format: u8,
    location: u8,
    id: u8,
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

fn read_u16(c: &mut Cursor<&[u8]>) -> Result<u16, AssetError> {
    let mut buf = [0u8; 2];
    c.read_exact(&mut buf)?;
    Ok(u16::from_le_bytes(buf))
}

fn read_bytes(c: &mut Cursor<&[u8]>, len: usize) -> Result<Vec<u8>, AssetError> {
    let mut buf = vec![0u8; len];
    c.read_exact(&mut buf)?;
    Ok(buf)
}

fn read_lt_string(c: &mut Cursor<&[u8]>) -> Result<String, AssetError> {
    let len = read_u16(c)? as usize;
    let bytes = read_bytes(c, len)?;
    String::from_utf8(bytes).map_err(|_| AssetError::Utf8)
}

fn read_vertex_def(c: &mut Cursor<&[u8]>) -> Result<Vec<VertexProp>, AssetError> {
    let size = crate::read_u32(c)? as usize;
    let start = c.position();
    let slots = size / 8;
    let mut props = Vec::new();
    for _ in 0..slots {
        let a = read_u16(c)?;
        let offset = read_u16(c)?;
        let mut rest = [0u8; 4];
        c.read_exact(&mut rest)?;
        if a == 255 {
            break;
        }
        props.push(VertexProp {
            offset,
            format: rest[0],
            location: rest[2],
            id: rest[3],
        });
    }
    c.seek(SeekFrom::Start(start + size as u64))?;
    Ok(props)
}

fn read_surface_header(c: &mut Cursor<&[u8]>) -> Result<RawSurface, AssetError> {
    Ok(RawSurface {
        vertices_start: crate::read_u32(c)?,
        vertices_count: crate::read_u32(c)?,
        vertex_stride: crate::read_u32(c)?,
        indices_start: crate::read_u32(c)?,
        indices_base: crate::read_u32(c)?,
        triangle_count: crate::read_u32(c)?,
        material_id: crate::read_u32(c)?,
        pack_type_id: {
            let _unk = crate::read_u32(c)?;
            crate::read_u32(c)?
        },
    })
}

fn decode_surface_vertices(
    blob: &[u8],
    raw: &RawSurface,
    def: &[VertexProp],
) -> Result<Vec<WorldVertex>, AssetError> {
    let stride = raw.vertex_stride as usize;
    let start = raw.vertices_start as usize * stride;
    let mut out = Vec::with_capacity(raw.vertices_count as usize);
    for i in 0..raw.vertices_count as usize {
        let off = start + i * stride;
        let end = off.checked_add(stride).ok_or(AssetError::Truncated("vertex"))?;
        if end > blob.len() {
            return Err(AssetError::Truncated("vertex block"));
        }
        out.push(decode_vertex(&blob[off..end], def)?);
    }
    Ok(out)
}

fn decode_vertex(bytes: &[u8], def: &[VertexProp]) -> Result<WorldVertex, AssetError> {
    let mut pos = [0.0f32; 3];
    let mut normal = [0.0, 1.0, 0.0];
    let mut uv = [0.0f32; 2];
    let mut tangent = [1.0, 0.0, 0.0];
    let mut binormal = [0.0, 0.0, -1.0];
    let mut have_pos = false;
    for prop in def {
        let size = match prop.format {
            1 => 8,
            2 => 12,
            3 => 16,
            4 | 5 => 4,
            17 => continue,
            _ => return Err(AssetError::Invalid("vertex format")),
        };
        let off = usize::from(prop.offset);
        if off.checked_add(size).map(|end| end > bytes.len()).unwrap_or(true) {
            continue;
        }
        let slice = &bytes[off..off + size];
        if prop.id > 0 {
            continue;
        }
        match (prop.location, prop.format) {
            (0, 2) => {
                pos = unpack3(slice);
                have_pos = true;
            }
            (3, 2) => normal = unpack3(slice),
            (5, 1) => uv = unpack2(slice),
            (6, 2) => tangent = unpack3(slice),
            (7, 2) => binormal = unpack3(slice),
            _ => {}
        }
    }
    if !have_pos {
        return Err(AssetError::Invalid("vertex missing position"));
    }
    Ok(WorldVertex {
        position: pos,
        normal,
        uv,
        tangent,
        binormal,
    })
}

fn unpack2(slice: &[u8]) -> [f32; 2] {
    [
        f32::from_le_bytes(slice[0..4].try_into().unwrap()),
        f32::from_le_bytes(slice[4..8].try_into().unwrap()),
    ]
}

fn unpack3(slice: &[u8]) -> [f32; 3] {
    [
        f32::from_le_bytes(slice[0..4].try_into().unwrap()),
        f32::from_le_bytes(slice[4..8].try_into().unwrap()),
        f32::from_le_bytes(slice[8..12].try_into().unwrap()),
    ]
}

fn decode_surface_indices(
    blob: &[u8],
    raw: &RawSurface,
    vert_count: usize,
) -> Result<Vec<u32>, AssetError> {
    let offset = raw.vertices_start.wrapping_sub(raw.indices_base) as i32;
    let mut indices = Vec::with_capacity(raw.triangle_count as usize * 3);
    let base = raw.indices_start as usize * 2;
    for t in 0..raw.triangle_count as usize {
        let off = base + t * 6;
        if off + 6 > blob.len() {
            return Err(AssetError::Truncated("index block"));
        }
        let i0 = u16::from_le_bytes(blob[off..off + 2].try_into().unwrap()) as i32 - offset;
        let i1 = u16::from_le_bytes(blob[off + 2..off + 4].try_into().unwrap()) as i32 - offset;
        let i2 = u16::from_le_bytes(blob[off + 4..off + 6].try_into().unwrap()) as i32 - offset;
        for i in [i0, i2, i1] {
            if i < 0 || i as usize >= vert_count {
                return Err(AssetError::Invalid("index remap"));
            }
            indices.push(i as u32);
        }
    }
    Ok(indices)
}

fn material_color(name: &str) -> [f32; 4] {
    let h = name.bytes().fold(2166136261u32, |a, b| a.wrapping_mul(16777619) ^ u32::from(b));
    let r = 0.12 + ((h & 0xFF) as f32 / 255.0) * 0.28;
    let g = 0.11 + (((h >> 8) & 0xFF) as f32 / 255.0) * 0.22;
    let b = 0.10 + (((h >> 16) & 0xFF) as f32 / 255.0) * 0.20;
    [r, g, b, 1.0]
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

    #[test]
    fn decode_uv_by_offset() {
        let def = [
            VertexProp {
                offset: 0,
                format: 2,
                location: 0,
                id: 0,
            },
            VertexProp {
                offset: 12,
                format: 1,
                location: 5,
                id: 0,
            },
        ];
        let mut bytes = vec![0u8; 20];
        bytes[0..4].copy_from_slice(&1.0f32.to_le_bytes());
        bytes[4..8].copy_from_slice(&2.0f32.to_le_bytes());
        bytes[8..12].copy_from_slice(&3.0f32.to_le_bytes());
        bytes[12..16].copy_from_slice(&0.25f32.to_le_bytes());
        bytes[16..20].copy_from_slice(&0.75f32.to_le_bytes());
        let v = decode_vertex(&bytes, &def).unwrap();
        assert_eq!(v.position, [1.0, 2.0, 3.0]);
        assert_eq!(v.uv, [0.25, 0.75]);
    }

    #[test]
    fn decode_tangent_binormal_no_blender_swap() {
        let def = [
            VertexProp {
                offset: 0,
                format: 2,
                location: 0,
                id: 0,
            },
            VertexProp {
                offset: 12,
                format: 2,
                location: 3,
                id: 0,
            },
            VertexProp {
                offset: 24,
                format: 2,
                location: 6,
                id: 0,
            },
            VertexProp {
                offset: 36,
                format: 2,
                location: 7,
                id: 0,
            },
        ];
        let mut bytes = vec![0u8; 48];
        write3(&mut bytes, 0, [0.0, 0.0, 0.0]);
        write3(&mut bytes, 12, [0.0, 1.0, 0.0]);
        write3(&mut bytes, 24, [1.0, 0.0, 0.0]);
        write3(&mut bytes, 36, [0.0, 0.0, -1.0]);
        let v = decode_vertex(&bytes, &def).unwrap();
        assert_eq!(v.normal, [0.0, 1.0, 0.0]);
        assert_eq!(v.tangent, [1.0, 0.0, 0.0]);
        assert_eq!(v.binormal, [0.0, 0.0, -1.0]);
    }

    fn write3(buf: &mut [u8], off: usize, v: [f32; 3]) {
        buf[off..off + 4].copy_from_slice(&v[0].to_le_bytes());
        buf[off + 4..off + 8].copy_from_slice(&v[1].to_le_bytes());
        buf[off + 8..off + 12].copy_from_slice(&v[2].to_le_bytes());
    }
}
