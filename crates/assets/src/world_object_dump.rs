//! Сырой дамп секции объектов World00p: все типы и поля, без фильтра рендера.
//!
//! `WorldObjects::parse` оставляет только спавн / point-fill / WorldModel.
//! Этот модуль нужен, чтобы видеть оригинал (LightSpot, FarZ, прочие объекты).

use crate::world00p::WorldHeader;
use crate::world_objects::{read_lt_string, read_property_bag, PropValue};
use crate::AssetError;
use std::collections::BTreeMap;
use std::fmt;
use std::io::{Cursor, Seek, SeekFrom};

/// Одно свойство property bag, как в файле.
#[derive(Debug, Clone, PartialEq)]
pub struct RawProp {
    pub name: String,
    pub value: String,
}

/// Объект World00p без отбора по типу.
#[derive(Debug, Clone, PartialEq)]
pub struct RawWorldObject {
    pub type_name: String,
    pub properties: Vec<RawProp>,
}

impl RawWorldObject {
    pub fn prop(&self, name: &str) -> Option<&str> {
        self.properties.iter().find_map(|prop| {
            if prop.name.eq_ignore_ascii_case(name) {
                Some(prop.value.as_str())
            } else {
                None
            }
        })
    }
}

/// Полный снимок секции объектов.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RawWorldObjects {
    pub objects: Vec<RawWorldObject>,
}

impl RawWorldObjects {
    pub fn parse(bytes: &[u8]) -> Result<Self, AssetError> {
        let header = WorldHeader::parse(bytes)?;
        parse_raw_at(bytes, header.object_section_offset as u64)
    }

    /// Сколько объектов каждого WorldEdit-типа.
    pub fn type_histogram(&self) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for object in &self.objects {
            *counts.entry(object.type_name.clone()).or_insert(0) += 1;
        }
        counts
    }

    pub fn of_type<'a>(&'a self, type_name: &'a str) -> impl Iterator<Item = &'a RawWorldObject> {
        self.objects
            .iter()
            .filter(move |object| object.type_name.eq_ignore_ascii_case(type_name))
    }
}

impl fmt::Display for PropValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PropValue::String(s) => write!(f, "{s}"),
            PropValue::Vector(v) => write!(f, "{},{},{}", v.x, v.y, v.z),
            PropValue::Colour(v) => write!(f, "{},{},{}", v.x, v.y, v.z),
            PropValue::Float(v) => write!(f, "{v}"),
            PropValue::Int(v) => write!(f, "{v}"),
            PropValue::Quat(q) => write!(f, "{},{},{},{}", q[0], q[1], q[2], q[3]),
        }
    }
}

fn parse_raw_at(bytes: &[u8], offset: u64) -> Result<RawWorldObjects, AssetError> {
    if offset as usize >= bytes.len() {
        return Err(AssetError::Truncated("object section"));
    }
    let mut cursor = Cursor::new(bytes);
    cursor.seek(SeekFrom::Start(offset))?;
    let count = crate::read_u32(&mut cursor)? as usize;
    let mut objects = Vec::with_capacity(count);
    for _ in 0..count {
        let type_name = read_lt_string(&mut cursor)?;
        let bag = read_property_bag(&mut cursor)?;
        let properties = bag
            .values
            .into_iter()
            .map(|(name, value)| RawProp {
                name,
                value: value.to_string(),
            })
            .collect();
        objects.push(RawWorldObject {
            type_name,
            properties,
        });
    }
    Ok(RawWorldObjects { objects })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world00p::FEAR_WORLD_VERSION;
    use crate::world_objects::WorldObjects;

    const PROP_STRING: u32 = 0;
    const PROP_FLOAT: u32 = 3;

    #[test]
    fn dump_keeps_light_spot_that_render_parse_drops() {
        let mut world = vec![0u8; 56];
        world[0..4].copy_from_slice(&FEAR_WORLD_VERSION.to_le_bytes());
        world[12..16].copy_from_slice(&56u32.to_le_bytes());
        let mut section = Vec::new();
        section.extend_from_slice(&1u32.to_le_bytes());
        write_spot(&mut section, 400.0, 45.0);
        world.extend_from_slice(&section);

        let filtered = WorldObjects::parse(&world).unwrap();
        assert!(filtered.lights.is_empty());

        let raw = RawWorldObjects::parse(&world).unwrap();
        assert_eq!(raw.of_type("LightSpot").count(), 1);
        let spot = raw.of_type("LightSpot").next().unwrap();
        assert_eq!(spot.prop("LightRadius"), Some("400"));
        assert_eq!(spot.prop("FovX"), Some("45"));
        assert_eq!(raw.type_histogram().get("LightSpot"), Some(&1));
    }

    #[test]
    fn dump_keeps_worldproperties_farz() {
        let mut world = vec![0u8; 56];
        world[0..4].copy_from_slice(&FEAR_WORLD_VERSION.to_le_bytes());
        world[12..16].copy_from_slice(&56u32.to_le_bytes());
        let mut section = Vec::new();
        section.extend_from_slice(&1u32.to_le_bytes());
        write_world_properties(&mut section, 100000.0);
        world.extend_from_slice(&section);

        let raw = RawWorldObjects::parse(&world).unwrap();
        let props = raw.of_type("WorldProperties").next().unwrap();
        assert_eq!(props.prop("FarZ"), Some("100000"));
        assert!(WorldObjects::parse(&world).unwrap().lights.is_empty());
    }

    fn write_spot(buf: &mut Vec<u8>, radius: f32, fov_x: f32) {
        write_lt(buf, "LightSpot");
        let mut heap = Vec::new();
        let name_key = push_cstr(&mut heap, "Name");
        let name_val = push_cstr(&mut heap, "spot00");
        let radius_key = push_cstr(&mut heap, "LightRadius");
        let fov_key = push_cstr(&mut heap, "FovX");
        buf.extend_from_slice(&3u32.to_le_bytes());
        buf.extend_from_slice(&(heap.len() as u32).to_le_bytes());
        buf.extend_from_slice(&heap);
        write_prop(buf, name_key, PROP_STRING, name_val as u32);
        write_prop(buf, radius_key, PROP_FLOAT, radius.to_bits());
        write_prop(buf, fov_key, PROP_FLOAT, fov_x.to_bits());
    }

    fn write_world_properties(buf: &mut Vec<u8>, far_z: f32) {
        write_lt(buf, "WorldProperties");
        let mut heap = Vec::new();
        let name_key = push_cstr(&mut heap, "Name");
        let name_val = push_cstr(&mut heap, "WorldProperties00");
        let far_key = push_cstr(&mut heap, "FarZ");
        buf.extend_from_slice(&2u32.to_le_bytes());
        buf.extend_from_slice(&(heap.len() as u32).to_le_bytes());
        buf.extend_from_slice(&heap);
        write_prop(buf, name_key, PROP_STRING, name_val as u32);
        write_prop(buf, far_key, PROP_FLOAT, far_z.to_bits());
    }

    fn write_lt(buf: &mut Vec<u8>, s: &str) {
        buf.extend_from_slice(&(s.len() as u16).to_le_bytes());
        buf.extend_from_slice(s.as_bytes());
    }

    fn push_cstr(heap: &mut Vec<u8>, s: &str) -> u32 {
        let off = heap.len() as u32;
        heap.extend_from_slice(s.as_bytes());
        heap.push(0);
        off
    }

    fn write_prop(buf: &mut Vec<u8>, name_off: u32, kind: u32, data: u32) {
        buf.extend_from_slice(&name_off.to_le_bytes());
        buf.extend_from_slice(&kind.to_le_bytes());
        buf.extend_from_slice(&data.to_le_bytes());
    }
}
