//! Packed Jupiter EX material (`Mat00`). Magic `LTMI`, first FX supplies texture slots.

use crate::AssetError;
use std::io::{Cursor, Read};

const MAGIC: &[u8; 4] = b"LTMI";

#[derive(Debug, Clone)]
pub enum MatValue {
    String(String),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Int(i32),
    Float(f32),
}

#[derive(Debug, Clone)]
pub struct MatFx {
    pub shader: String,
    pub defs: Vec<(String, MatValue)>,
}

#[derive(Debug, Clone)]
pub struct Material {
    pub fx: Vec<MatFx>,
}

impl Material {
    pub fn parse(bytes: &[u8]) -> Result<Self, AssetError> {
        if bytes.len() < 8 {
            return Err(AssetError::Truncated("Mat00 header"));
        }
        let mut c = Cursor::new(bytes);
        let mut magic = [0u8; 4];
        c.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(AssetError::Invalid("Mat00 magic"));
        }
        let count = crate::read_u32(&mut c)? as usize;
        let mut fx = Vec::with_capacity(count);
        for _ in 0..count {
            fx.push(read_fx(&mut c)?);
        }
        Ok(Self { fx })
    }

    pub fn string_def(&self, name: &str) -> Option<&str> {
        for layer in &self.fx {
            for (key, value) in &layer.defs {
                if key.eq_ignore_ascii_case(name) {
                    if let MatValue::String(s) = value {
                        return Some(s.as_str());
                    }
                }
            }
        }
        None
    }

    pub fn diffuse_map(&self) -> Option<&str> {
        self.string_def("tDiffuseMap")
    }

    pub fn normal_map(&self) -> Option<&str> {
        self.string_def("tNormalMap")
    }

    pub fn specular_map(&self) -> Option<&str> {
        self.string_def("tSpecularMap")
    }

    /// Путь шейдера первого FX. Нужен, чтобы отличить skybox от обычной стены.
    pub fn shader(&self) -> Option<&str> {
        self.fx.first().map(|layer| layer.shader.as_str())
    }

    /// `skybox.fx` семплит cubemap через нормаль куба, это не плоская стена.
    pub fn is_skybox(&self) -> bool {
        self.shader()
            .is_some_and(|shader| shader.to_ascii_lowercase().contains("skybox"))
    }

    pub fn float_def(&self, name: &str) -> Option<f32> {
        for layer in &self.fx {
            for (key, value) in &layer.defs {
                if key.eq_ignore_ascii_case(name) {
                    if let MatValue::Float(v) = value {
                        return Some(*v);
                    }
                }
            }
        }
        None
    }

    pub fn max_specular_power(&self) -> f32 {
        self.float_def("fMaxSpecularPower").unwrap_or(64.0)
    }
}

fn read_fx(c: &mut Cursor<&[u8]>) -> Result<MatFx, AssetError> {
    let shader = read_lt_string(c)?;
    let count = crate::read_u32(c)? as usize;
    let mut defs = Vec::with_capacity(count);
    for _ in 0..count {
        let kind = crate::read_u32(c)?;
        let name = read_lt_string(c)?;
        let value = match kind {
            1 => MatValue::String(read_lt_string(c)?),
            2 => MatValue::Vec3(read_f32x(c, 3)?.try_into().unwrap()),
            3 => MatValue::Vec4(read_f32x(c, 4)?.try_into().unwrap()),
            4 => MatValue::Int(read_i32(c)?),
            5 => MatValue::Float(read_f32(c)?),
            _ => return Err(AssetError::Invalid("Mat00 def type")),
        };
        defs.push((name, value));
    }
    Ok(MatFx { shader, defs })
}

fn read_lt_string(c: &mut Cursor<&[u8]>) -> Result<String, AssetError> {
    let mut len_buf = [0u8; 2];
    c.read_exact(&mut len_buf)?;
    let len = u16::from_le_bytes(len_buf) as usize;
    let mut bytes = vec![0u8; len];
    c.read_exact(&mut bytes)?;
    String::from_utf8(bytes).map_err(|_| AssetError::Utf8)
}

fn read_i32(c: &mut Cursor<&[u8]>) -> Result<i32, AssetError> {
    let mut buf = [0u8; 4];
    c.read_exact(&mut buf)?;
    Ok(i32::from_le_bytes(buf))
}

fn read_f32(c: &mut Cursor<&[u8]>) -> Result<f32, AssetError> {
    let mut buf = [0u8; 4];
    c.read_exact(&mut buf)?;
    Ok(f32::from_le_bytes(buf))
}

fn read_f32x(c: &mut Cursor<&[u8]>, n: usize) -> Result<Vec<f32>, AssetError> {
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(read_f32(c)?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_diffuse_slot() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"LTMI");
        bytes.extend_from_slice(&1u32.to_le_bytes());
        write_lt(&mut bytes, "shaders/model.fx");
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        write_lt(&mut bytes, "tDiffuseMap");
        write_lt(&mut bytes, r"Textures\Office\Floor.dds");
        let mat = Material::parse(&bytes).unwrap();
        assert_eq!(mat.diffuse_map(), Some(r"Textures\Office\Floor.dds"));
        assert_eq!(
            mat.string_def("TDIFFUSEMAP"),
            Some(r"Textures\Office\Floor.dds")
        );
    }

    #[test]
    fn parse_spec_normal_and_power() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"LTMI");
        bytes.extend_from_slice(&1u32.to_le_bytes());
        write_lt(&mut bytes, "shaders/rigid/Solid/specular.fx");
        bytes.extend_from_slice(&4u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        write_lt(&mut bytes, "tDiffuseMap");
        write_lt(&mut bytes, r"Tex\Office\Floor_D.dds");
        bytes.extend_from_slice(&1u32.to_le_bytes());
        write_lt(&mut bytes, "tNormalMap");
        write_lt(&mut bytes, r"Tex\Office\Floor_N.dds");
        bytes.extend_from_slice(&1u32.to_le_bytes());
        write_lt(&mut bytes, "tSpecularMap");
        write_lt(&mut bytes, r"Tex\Office\Floor_S.dds");
        bytes.extend_from_slice(&5u32.to_le_bytes());
        write_lt(&mut bytes, "fMaxSpecularPower");
        bytes.extend_from_slice(&64f32.to_le_bytes());
        let mat = Material::parse(&bytes).unwrap();
        assert_eq!(mat.normal_map(), Some(r"Tex\Office\Floor_N.dds"));
        assert_eq!(mat.specular_map(), Some(r"Tex\Office\Floor_S.dds"));
        assert_eq!(mat.max_specular_power(), 64.0);
        assert!(!mat.is_skybox());
    }

    #[test]
    fn skybox_shader_is_not_a_wall() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"LTMI");
        bytes.extend_from_slice(&1u32.to_le_bytes());
        write_lt(&mut bytes, r"shaders\rigid\Solid\skybox.fx");
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        write_lt(&mut bytes, "tDiffuseMap");
        write_lt(&mut bytes, r"Prefabs\Systemic\Sky\Sky_Day_C.dds");
        let mat = Material::parse(&bytes).unwrap();
        assert!(mat.is_skybox());
        assert_eq!(
            mat.diffuse_map(),
            Some(r"Prefabs\Systemic\Sky\Sky_Day_C.dds")
        );
    }

    fn write_lt(buf: &mut Vec<u8>, s: &str) {
        buf.extend_from_slice(&(s.len() as u16).to_le_bytes());
        buf.extend_from_slice(s.as_bytes());
    }
}
