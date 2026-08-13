//! World-model / PhysicsBSP section of a packed World00p (starts at byte 56).

use crate::world00p::FEAR_WORLD_MAGIC;
use crate::AssetError;
use glam::Vec3;
use std::io::{Cursor, Read, Seek, SeekFrom};

#[derive(Debug, Clone)]
pub struct BspPoly {
    pub flags: u16,
    pub plane_id: u32,
    pub verts: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct WorldBsp {
    pub names: Vec<String>,
    pub center: Vec3,
    pub half_extents: Vec3,
    pub points: Vec<Vec3>,
    pub polygons: Vec<BspPoly>,
}

#[derive(Debug, Clone)]
pub struct WorldModels {
    pub min: Vec3,
    pub max: Vec3,
    pub planes: Vec<Vec3>,
    pub models: Vec<WorldBsp>,
    pub blockers: Vec<[Vec3; 3]>,
}

impl WorldModels {
    pub fn parse(bytes: &[u8]) -> Result<Self, AssetError> {
        if bytes.len() < 56 {
            return Err(AssetError::Truncated("World00p header"));
        }
        let mut c = Cursor::new(bytes);
        c.seek(SeekFrom::Start(56))?;
        let min = read_vec3(&mut c)?;
        let max = read_vec3(&mut c)?;
        let count = crate::read_u32(&mut c)? as usize;
        let _zero = crate::read_u32(&mut c)?;
        let flag_bytes = count.div_ceil(8);
        let mut flags = vec![0u8; flag_bytes];
        c.read_exact(&mut flags)?;

        let mut raw = [0u32; 8];
        for slot in &mut raw {
            *slot = crate::read_u32(&mut c)? ^ FEAR_WORLD_MAGIC;
        }
        let name_count = raw[0] as usize;
        let names_len = raw[1] as usize;
        let plane_count = raw[2] as usize;
        let bsp_count = raw[3] as usize;

        let name_blob = read_bytes(&mut c, names_len)?;
        let mut name_index = Vec::with_capacity(name_count);
        for _ in 0..name_count {
            let off = crate::read_u32(&mut c)? as usize;
            let bsp_i = crate::read_u32(&mut c)? as usize;
            name_index.push((off, bsp_i));
        }
        let mut names = vec![Vec::new(); bsp_count];
        for (off, bsp_i) in name_index {
            if bsp_i >= bsp_count {
                continue;
            }
            names[bsp_i].push(cstring(&name_blob, off)?);
        }

        let mut planes = Vec::with_capacity(plane_count);
        for _ in 0..plane_count {
            planes.push(read_vec3(&mut c)?);
        }

        let mut models = Vec::new();
        for i in 0..bsp_count {
            let mut bsp = read_bsp(&mut c)?;
            bsp.names = std::mem::take(&mut names[i]);
            models.push(bsp);
        }
        let blockers = read_blockers(&mut c).unwrap_or_default();
        Ok(Self {
            min,
            max,
            planes,
            models,
            blockers,
        })
    }

    pub fn physics(&self) -> Option<&WorldBsp> {
        self.models.iter().find(|bsp| bsp.is_physics())
    }

    /// Клип: только PhysicsBSP + world-space blockers. Двери пока не входят.
    pub fn triangles(&self) -> Vec<[Vec3; 3]> {
        let mut out = Vec::new();
        if let Some(bsp) = self.physics() {
            out.extend(bsp.triangles());
        }
        out.extend_from_slice(&self.blockers);
        out
    }

    pub fn mesh_named(&self, name: &str) -> Option<&WorldBsp> {
        self.models
            .iter()
            .find(|bsp| bsp.names.iter().any(|n| n.eq_ignore_ascii_case(name)))
    }
}

impl WorldBsp {
    pub fn is_physics(&self) -> bool {
        self.names
            .iter()
            .any(|n| n.eq_ignore_ascii_case("PhysicsBSP"))
    }

    pub fn triangles(&self) -> Vec<[Vec3; 3]> {
        let mut out = Vec::new();
        fan_polys(&self.points, &self.polygons, &mut out);
        out
    }
}

fn read_bsp(c: &mut Cursor<&[u8]>) -> Result<WorldBsp, AssetError> {
    let _id = crate::read_u32(c)?;
    let point_count = crate::read_u32(c)? as usize;
    let polygon_count = crate::read_u32(c)? as usize;
    let _unk = crate::read_u32(c)?;
    let node_count = crate::read_u32(c)? as usize;
    let half_extents = read_vec3(c)?;
    let center = read_vec3(c)?;
    let _zero = crate::read_u32(c)?;
    let mut vert_counts = vec![0u8; polygon_count];
    c.read_exact(&mut vert_counts)?;
    let mut polygons = Vec::with_capacity(polygon_count);
    for count in vert_counts {
        let mut skip = [0u8; 2];
        c.read_exact(&mut skip)?;
        let flags = read_u16(c)?;
        let plane_id = crate::read_u32(c)?;
        let _dist = read_f32(c)?;
        let mut verts = Vec::with_capacity(count as usize);
        for _ in 0..count {
            verts.push(crate::read_u32(c)?);
        }
        polygons.push(BspPoly {
            flags,
            plane_id,
            verts,
        });
    }
    c.seek(SeekFrom::Current(node_count as i64 * 12))?;
    let mut points = Vec::with_capacity(point_count);
    for _ in 0..point_count {
        points.push(read_vec3(c)?);
    }
    Ok(WorldBsp {
        names: Vec::new(),
        center,
        half_extents,
        points,
        polygons,
    })
}

fn fan_polys(points: &[Vec3], polygons: &[BspPoly], out: &mut Vec<[Vec3; 3]>) {
    for poly in polygons {
        if poly.verts.len() < 3 {
            continue;
        }
        let Some(&i0) = poly.verts.first() else {
            continue;
        };
        let Some(&p0) = points.get(i0 as usize) else {
            continue;
        };
        for pair in poly.verts.windows(2).skip(1) {
            let Some(&a) = points.get(pair[0] as usize) else {
                continue;
            };
            let Some(&b) = points.get(pair[1] as usize) else {
                continue;
            };
            out.push([p0, a, b]);
        }
    }
}

fn read_blockers(c: &mut Cursor<&[u8]>) -> Result<Vec<[Vec3; 3]>, AssetError> {
    let poly_count = crate::read_u32(c)? as usize;
    let _unk = crate::read_u32(c)?;
    if poly_count > 10_000 {
        return Err(AssetError::Truncated("blocker count"));
    }
    let mut tris = Vec::new();
    for _ in 0..poly_count {
        let _normal = read_vec3(c)?;
        let _distance = read_f32(c)?;
        let nverts = crate::read_u32(c)? as usize;
        if nverts > 256 {
            return Err(AssetError::Truncated("blocker verts"));
        }
        let mut verts = Vec::with_capacity(nverts);
        for _ in 0..nverts {
            verts.push(read_vec3(c)?);
        }
        if verts.len() < 3 {
            continue;
        }
        let p0 = verts[0];
        for pair in verts.windows(2).skip(1) {
            tris.push([p0, pair[0], pair[1]]);
        }
    }
    Ok(tris)
}

fn read_vec3(c: &mut Cursor<&[u8]>) -> Result<Vec3, AssetError> {
    Ok(Vec3::new(read_f32(c)?, read_f32(c)?, read_f32(c)?))
}

fn read_f32(c: &mut Cursor<&[u8]>) -> Result<f32, AssetError> {
    let mut buf = [0u8; 4];
    c.read_exact(&mut buf)?;
    Ok(f32::from_le_bytes(buf))
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

fn cstring(blob: &[u8], off: usize) -> Result<String, AssetError> {
    if off >= blob.len() {
        return Err(AssetError::Truncated("bsp name"));
    }
    let end = blob[off..]
        .iter()
        .position(|&b| b == 0)
        .map(|p| off + p)
        .unwrap_or(blob.len());
    std::str::from_utf8(&blob[off..end])
        .map(|s| s.to_string())
        .map_err(|_| AssetError::Utf8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world00p::FEAR_WORLD_VERSION;

    #[test]
    fn parse_physics_floor() {
        let mut bytes = vec![0u8; 56];
        bytes[0..4].copy_from_slice(&FEAR_WORLD_VERSION.to_le_bytes());
        bytes[4..8].copy_from_slice(&2000u32.to_le_bytes());

        let mut body = Vec::new();
        body.extend_from_slice(&[0u8; 24]); // min/max
        body.extend_from_slice(&1u32.to_le_bytes());
        body.extend_from_slice(&0u32.to_le_bytes());
        body.push(0); // flags
        let counts = [1u32, 11, 0, 1, 0, 1, 3, 3];
        for c in counts {
            body.extend_from_slice(&(c ^ FEAR_WORLD_MAGIC).to_le_bytes());
        }
        body.extend_from_slice(b"PhysicsBSP\0");
        body.extend_from_slice(&0u32.to_le_bytes());
        body.extend_from_slice(&0u32.to_le_bytes());
        // bsp
        body.extend_from_slice(&1u32.to_le_bytes()); // id
        body.extend_from_slice(&3u32.to_le_bytes()); // points
        body.extend_from_slice(&1u32.to_le_bytes()); // polys
        body.extend_from_slice(&0u32.to_le_bytes());
        body.extend_from_slice(&0u32.to_le_bytes()); // nodes
        body.extend_from_slice(&[0u8; 24]); // half + center
        body.extend_from_slice(&0u32.to_le_bytes());
        body.push(3); // vert count
        body.extend_from_slice(&[0u8; 2]);
        body.extend_from_slice(&0u16.to_le_bytes());
        body.extend_from_slice(&0u32.to_le_bytes());
        body.extend_from_slice(&0f32.to_le_bytes());
        for i in 0u32..3 {
            body.extend_from_slice(&i.to_le_bytes());
        }
        write_vec3(&mut body, Vec3::new(0.0, 0.0, 0.0));
        write_vec3(&mut body, Vec3::new(10.0, 0.0, 0.0));
        write_vec3(&mut body, Vec3::new(0.0, 0.0, 10.0));
        body.extend_from_slice(&1u32.to_le_bytes());
        body.extend_from_slice(&3u32.to_le_bytes());
        write_vec3(&mut body, Vec3::Y);
        body.extend_from_slice(&0f32.to_le_bytes());
        body.extend_from_slice(&3u32.to_le_bytes());
        write_vec3(&mut body, Vec3::new(20.0, -10.0, 20.0));
        write_vec3(&mut body, Vec3::new(30.0, -10.0, 20.0));
        write_vec3(&mut body, Vec3::new(20.0, -10.0, 30.0));

        bytes.extend_from_slice(&body);
        let models = WorldModels::parse(&bytes).unwrap();
        assert_eq!(models.physics().unwrap().names, ["PhysicsBSP"]);
        assert_eq!(models.models.len(), 1);
        assert_eq!(models.blockers.len(), 1);
        assert_eq!(models.triangles().len(), 2);
    }

    #[test]
    fn keeps_named_worldmodel_bsp() {
        let mut bytes = vec![0u8; 56];
        bytes[0..4].copy_from_slice(&FEAR_WORLD_VERSION.to_le_bytes());
        bytes[4..8].copy_from_slice(&2000u32.to_le_bytes());

        let mut body = Vec::new();
        body.extend_from_slice(&[0u8; 24]);
        body.extend_from_slice(&1u32.to_le_bytes());
        body.extend_from_slice(&0u32.to_le_bytes());
        body.push(0);
        let crate_name = b"Crate00\0";
        let phys_name = b"PhysicsBSP\0";
        let names_len = (phys_name.len() + crate_name.len()) as u32;
        let counts = [2u32, names_len, 0, 2, 0, 1, 3, 3];
        for c in counts {
            body.extend_from_slice(&(c ^ FEAR_WORLD_MAGIC).to_le_bytes());
        }
        body.extend_from_slice(phys_name);
        body.extend_from_slice(crate_name);
        body.extend_from_slice(&0u32.to_le_bytes());
        body.extend_from_slice(&0u32.to_le_bytes());
        body.extend_from_slice(&(phys_name.len() as u32).to_le_bytes());
        body.extend_from_slice(&1u32.to_le_bytes());
        write_tri_bsp(&mut body);
        write_tri_bsp(&mut body);
        body.extend_from_slice(&0u32.to_le_bytes());
        body.extend_from_slice(&0u32.to_le_bytes());

        bytes.extend_from_slice(&body);
        let models = WorldModels::parse(&bytes).unwrap();
        assert_eq!(models.models.len(), 2);
        assert!(models.physics().unwrap().is_physics());
        let crate_bsp = models.mesh_named("Crate00").expect("named BSP dropped");
        assert_eq!(crate_bsp.triangles().len(), 1);
        assert_eq!(
            models.triangles().len(),
            1,
            "clip must stay PhysicsBSP-only"
        );
    }

    fn write_tri_bsp(body: &mut Vec<u8>) {
        body.extend_from_slice(&1u32.to_le_bytes());
        body.extend_from_slice(&3u32.to_le_bytes());
        body.extend_from_slice(&1u32.to_le_bytes());
        body.extend_from_slice(&0u32.to_le_bytes());
        body.extend_from_slice(&0u32.to_le_bytes());
        body.extend_from_slice(&[0u8; 24]);
        body.extend_from_slice(&0u32.to_le_bytes());
        body.push(3);
        body.extend_from_slice(&[0u8; 2]);
        body.extend_from_slice(&0u16.to_le_bytes());
        body.extend_from_slice(&0u32.to_le_bytes());
        body.extend_from_slice(&0f32.to_le_bytes());
        for i in 0u32..3 {
            body.extend_from_slice(&i.to_le_bytes());
        }
        write_vec3(body, Vec3::new(0.0, 0.0, 0.0));
        write_vec3(body, Vec3::new(10.0, 0.0, 0.0));
        write_vec3(body, Vec3::new(0.0, 0.0, 10.0));
    }

    fn write_vec3(buf: &mut Vec<u8>, v: Vec3) {
        buf.extend_from_slice(&v.x.to_le_bytes());
        buf.extend_from_slice(&v.y.to_le_bytes());
        buf.extend_from_slice(&v.z.to_le_bytes());
    }
}
