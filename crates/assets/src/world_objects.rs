//! Секция объектов World00p: спавн, лампы и расстановка WorldModel. Не полный object runtime.

use crate::world00p::WorldHeader;
use crate::AssetError;
use glam::{Quat, Vec3};
use std::io::{Cursor, Read, Seek, SeekFrom};

const PROP_STRING: u32 = 0;
const PROP_VECTOR: u32 = 1;
const PROP_COLOUR: u32 = 2;
const PROP_FLOAT: u32 = 3;
const PROP_INT: u32 = 4;
const PROP_FLAGS: u32 = 5;
const PROP_QUAT: u32 = 6;
const PROP_COMMAND: u32 = 7;
const PROP_TEXT: u32 = 8;

/// Типы, у которых геометрия — именованный BSP, а не Model00p.
const WORLD_MODEL_TYPES: &[&str] = &[
    "WorldModel",
    "RotatingDoor",
    "SlidingDoor",
    "RotatingWorldModel",
    "SlidingWorldModel",
    "SlidingSwitch",
    "RotatingSwitch",
    "SpinningWorldModel",
];

#[derive(Debug, Clone)]
pub struct GameStart {
    pub name: String,
    pub pos: Vec3,
    pub yaw: f32,
}

#[derive(Debug, Clone)]
pub struct WorldLight {
    pub name: String,
    pub position: Vec3,
    pub radius: f32,
    pub color: Vec3,
}

/// Экземпляр BSP в мире: машины, двери, граффити, небо.
#[derive(Debug, Clone)]
pub struct WorldModelPlacement {
    pub name: String,
    pub pos: Vec3,
    pub rotation: Quat,
    pub hidden: bool,
}

#[derive(Debug, Clone)]
pub struct WorldObjects {
    pub starts: Vec<GameStart>,
    pub lights: Vec<WorldLight>,
    pub models: Vec<WorldModelPlacement>,
    pub ambient: Vec3,
}

impl Default for WorldObjects {
    fn default() -> Self {
        Self {
            starts: Vec::new(),
            lights: Vec::new(),
            models: Vec::new(),
            ambient: Vec3::splat(0.1),
        }
    }
}

impl WorldObjects {
    pub fn parse(bytes: &[u8]) -> Result<Self, AssetError> {
        let header = WorldHeader::parse(bytes)?;
        parse_at(bytes, header.object_section_offset as u64)
    }

    pub fn spawn(&self) -> Option<&GameStart> {
        self.starts
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case("GameStartPoint00"))
            .or_else(|| self.starts.first())
    }
}

fn parse_at(bytes: &[u8], offset: u64) -> Result<WorldObjects, AssetError> {
    if offset as usize >= bytes.len() {
        return Err(AssetError::Truncated("object section"));
    }
    let mut c = Cursor::new(bytes);
    c.seek(SeekFrom::Start(offset))?;
    let count = crate::read_u32(&mut c)? as usize;
    let mut starts = Vec::new();
    let mut lights = Vec::new();
    let mut models = Vec::new();
    let mut ambient = Vec3::splat(25.0 / 255.0);
    for _ in 0..count {
        let type_name = read_lt_string(&mut c)?;
        let bag = read_property_bag(&mut c)?;
        if type_name == "GameStartPoint" {
            if let Some(pos) = bag.vector("Pos") {
                starts.push(GameStart {
                    name: bag.string("Name").unwrap_or("GameStartPoint").to_string(),
                    pos,
                    yaw: yaw_from_xyzw(bag.quat("Rotation").unwrap_or([0.0, 0.0, 0.0, 1.0])),
                });
            }
        } else if type_name == "LightPoint" || type_name == "LightPointFill" {
            if let Some(light) = world_light(&bag) {
                lights.push(light);
            }
        } else if type_name == "WorldProperties" {
            if let Some(color) = bag.colour("AmbientLight") {
                ambient = color / 255.0;
            }
        } else if WORLD_MODEL_TYPES.iter().any(|t| *t == type_name) {
            if let Some(place) = world_model_placement(&bag) {
                models.push(place);
            }
        }
    }
    Ok(WorldObjects {
        starts,
        lights,
        models,
        ambient,
    })
}

fn world_light(bag: &PropertyBag) -> Option<WorldLight> {
    let position = bag.vector("Pos")?;
    let radius = bag.float("LightRadius").unwrap_or(0.0);
    let scale = bag.float("IntensityScale").unwrap_or(1.0);
    if radius <= 1.0 || scale <= 0.0 {
        return None;
    }
    let rgb = bag.colour("LightColor").unwrap_or(Vec3::splat(255.0)) / 255.0;
    Some(WorldLight {
        name: bag.string("Name").unwrap_or("light").to_string(),
        position,
        radius,
        color: rgb * scale,
    })
}

fn world_model_placement(bag: &PropertyBag) -> Option<WorldModelPlacement> {
    let pos = bag.vector("Pos")?;
    let q = bag.quat("Rotation").unwrap_or([0.0, 0.0, 0.0, 1.0]);
    Some(WorldModelPlacement {
        name: bag.string("Name").unwrap_or("WorldModel").to_string(),
        pos,
        rotation: Quat::from_xyzw(q[0], q[1], q[2], q[3]),
        hidden: bag.int("StartHidden").unwrap_or(0) != 0,
    })
}

fn yaw_from_xyzw(q: [f32; 4]) -> f32 {
    let rot = Quat::from_xyzw(q[0], q[1], q[2], q[3]);
    let fwd = rot * Vec3::Z;
    fwd.x.atan2(fwd.z)
}

struct PropertyBag {
    values: Vec<(String, PropValue)>,
}

enum PropValue {
    String(String),
    Vector(Vec3),
    Colour(Vec3),
    Float(f32),
    Int(i32),
    Quat([f32; 4]),
}

impl PropertyBag {
    fn string(&self, name: &str) -> Option<&str> {
        self.values.iter().find_map(|(n, v)| match v {
            PropValue::String(s) if n.eq_ignore_ascii_case(name) => Some(s.as_str()),
            _ => None,
        })
    }

    fn vector(&self, name: &str) -> Option<Vec3> {
        self.values.iter().find_map(|(n, v)| match v {
            PropValue::Vector(p) if n.eq_ignore_ascii_case(name) => Some(*p),
            _ => None,
        })
    }

    fn colour(&self, name: &str) -> Option<Vec3> {
        self.values.iter().find_map(|(n, v)| match v {
            PropValue::Colour(p) if n.eq_ignore_ascii_case(name) => Some(*p),
            _ => None,
        })
    }

    fn float(&self, name: &str) -> Option<f32> {
        self.values.iter().find_map(|(n, v)| match v {
            PropValue::Float(p) if n.eq_ignore_ascii_case(name) => Some(*p),
            _ => None,
        })
    }

    fn int(&self, name: &str) -> Option<i32> {
        self.values.iter().find_map(|(n, v)| match v {
            PropValue::Int(p) if n.eq_ignore_ascii_case(name) => Some(*p),
            _ => None,
        })
    }

    fn quat(&self, name: &str) -> Option<[f32; 4]> {
        self.values.iter().find_map(|(n, v)| match v {
            PropValue::Quat(p) if n.eq_ignore_ascii_case(name) => Some(*p),
            _ => None,
        })
    }
}

fn read_property_bag(c: &mut Cursor<&[u8]>) -> Result<PropertyBag, AssetError> {
    let prop_count = crate::read_u32(c)? as usize;
    let props_size = crate::read_u32(c)? as usize;
    let mut heap = vec![0u8; props_size];
    c.read_exact(&mut heap)?;
    let mut values = Vec::with_capacity(prop_count);
    for _ in 0..prop_count {
        let name_off = crate::read_u32(c)? as usize;
        let kind = crate::read_u32(c)?;
        let mut data = [0u8; 4];
        c.read_exact(&mut data)?;
        let name = cstring(&heap, name_off).unwrap_or_default();
        let value = match kind {
            PROP_STRING | PROP_COMMAND | PROP_TEXT => {
                let off = u32::from_le_bytes(data) as usize;
                PropValue::String(cstring(&heap, off).unwrap_or_default())
            }
            PROP_VECTOR => {
                let off = u32::from_le_bytes(data) as usize;
                PropValue::Vector(vec3_at(&heap, off).unwrap_or(Vec3::ZERO))
            }
            PROP_COLOUR => {
                let off = u32::from_le_bytes(data) as usize;
                PropValue::Colour(vec3_at(&heap, off).unwrap_or(Vec3::ZERO))
            }
            PROP_FLOAT => PropValue::Float(f32::from_le_bytes(data)),
            PROP_INT | PROP_FLAGS => PropValue::Int(i32::from_le_bytes(data)),
            PROP_QUAT => {
                let off = u32::from_le_bytes(data) as usize;
                PropValue::Quat(quat_at(&heap, off).unwrap_or([0.0, 0.0, 0.0, 1.0]))
            }
            _ => continue,
        };
        values.push((name, value));
    }
    Ok(PropertyBag { values })
}

fn read_lt_string(c: &mut Cursor<&[u8]>) -> Result<String, AssetError> {
    let mut len_buf = [0u8; 2];
    c.read_exact(&mut len_buf)?;
    let len = u16::from_le_bytes(len_buf) as usize;
    let mut bytes = vec![0u8; len];
    c.read_exact(&mut bytes)?;
    String::from_utf8(bytes).map_err(|_| AssetError::Utf8)
}

fn cstring(heap: &[u8], off: usize) -> Option<String> {
    if off >= heap.len() {
        return None;
    }
    let end = heap[off..]
        .iter()
        .position(|&b| b == 0)
        .map(|p| off + p)
        .unwrap_or(heap.len());
    std::str::from_utf8(&heap[off..end]).ok().map(|s| s.to_string())
}

fn vec3_at(heap: &[u8], off: usize) -> Option<Vec3> {
    let end = off.checked_add(12)?;
    let slice = heap.get(off..end)?;
    Some(Vec3::new(
        f32::from_le_bytes(slice[0..4].try_into().ok()?),
        f32::from_le_bytes(slice[4..8].try_into().ok()?),
        f32::from_le_bytes(slice[8..12].try_into().ok()?),
    ))
}

fn quat_at(heap: &[u8], off: usize) -> Option<[f32; 4]> {
    let end = off.checked_add(16)?;
    let slice = heap.get(off..end)?;
    Some([
        f32::from_le_bytes(slice[0..4].try_into().ok()?),
        f32::from_le_bytes(slice[4..8].try_into().ok()?),
        f32::from_le_bytes(slice[8..12].try_into().ok()?),
        f32::from_le_bytes(slice[12..16].try_into().ok()?),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world00p::FEAR_WORLD_VERSION;

    #[test]
    fn parse_start_and_fill_light() {
        let mut world = vec![0u8; 56];
        world[0..4].copy_from_slice(&FEAR_WORLD_VERSION.to_le_bytes());
        world[12..16].copy_from_slice(&56u32.to_le_bytes());

        let mut section = Vec::new();
        section.extend_from_slice(&3u32.to_le_bytes());
        write_start(&mut section);
        write_fill(&mut section);
        write_world_model(&mut section, "Crate00", false);
        world.extend_from_slice(&section);

        let objects = WorldObjects::parse(&world).unwrap();
        let spawn = objects.spawn().unwrap();
        assert_eq!(spawn.name, "GameStartPoint00");
        assert_eq!(spawn.pos, Vec3::new(-1100.0, -1436.0, -370.0));
        assert!(spawn.yaw.abs() < 0.2, "yaw {}", spawn.yaw);
        assert_eq!(objects.lights.len(), 1);
        assert!((objects.lights[0].color.x - (126.0 / 255.0) * 0.5).abs() < 0.01);
        assert_eq!(objects.lights[0].radius, 500.0);
        assert_eq!(objects.models.len(), 1);
        assert_eq!(objects.models[0].name, "Crate00");
        assert_eq!(objects.models[0].pos, Vec3::new(10.0, 20.0, 30.0));
        assert!(!objects.models[0].hidden);
    }

    #[test]
    fn hidden_worldmodel_is_marked() {
        let mut world = vec![0u8; 56];
        world[0..4].copy_from_slice(&FEAR_WORLD_VERSION.to_le_bytes());
        world[12..16].copy_from_slice(&56u32.to_le_bytes());
        let mut section = Vec::new();
        section.extend_from_slice(&1u32.to_le_bytes());
        write_world_model(&mut section, "HiddenDoor", true);
        world.extend_from_slice(&section);
        let objects = WorldObjects::parse(&world).unwrap();
        assert!(objects.models[0].hidden);
    }

    #[test]
    fn skips_disabled_light() {
        let mut world = vec![0u8; 56];
        world[0..4].copy_from_slice(&FEAR_WORLD_VERSION.to_le_bytes());
        world[12..16].copy_from_slice(&56u32.to_le_bytes());
        let mut section = Vec::new();
        section.extend_from_slice(&1u32.to_le_bytes());
        write_light(&mut section, 0.0, 500.0);
        world.extend_from_slice(&section);
        let objects = WorldObjects::parse(&world).unwrap();
        assert!(objects.lights.is_empty());
    }

    fn write_start(buf: &mut Vec<u8>) {
        write_lt(buf, "GameStartPoint");
        let mut heap = Vec::new();
        let name_key = push_cstr(&mut heap, "Name");
        let name_val = push_cstr(&mut heap, "GameStartPoint00");
        let pos_key = push_cstr(&mut heap, "Pos");
        let pos_val = heap.len();
        write_f32s(&mut heap, &[-1100.0, -1436.0, -370.0]);
        let rot_key = push_cstr(&mut heap, "Rotation");
        let rot_val = heap.len();
        write_f32s(&mut heap, &[0.0, -0.07846, 0.0, 0.99692]);
        buf.extend_from_slice(&3u32.to_le_bytes());
        buf.extend_from_slice(&(heap.len() as u32).to_le_bytes());
        buf.extend_from_slice(&heap);
        write_prop(buf, name_key, PROP_STRING, name_val as u32);
        write_prop(buf, pos_key, PROP_VECTOR, pos_val as u32);
        write_prop(buf, rot_key, PROP_QUAT, rot_val as u32);
    }

    fn write_fill(buf: &mut Vec<u8>) {
        write_light(buf, 0.5, 500.0);
    }

    fn write_light(buf: &mut Vec<u8>, scale: f32, radius: f32) {
        write_lt(buf, "LightPointFill");
        let mut heap = Vec::new();
        let name_key = push_cstr(&mut heap, "Name");
        let name_val = push_cstr(&mut heap, "fluo");
        let pos_key = push_cstr(&mut heap, "Pos");
        let pos_val = heap.len();
        write_f32s(&mut heap, &[-1567.0, -1386.0, 483.0]);
        let col_key = push_cstr(&mut heap, "LightColor");
        let col_val = heap.len();
        write_f32s(&mut heap, &[126.0, 145.0, 154.0]);
        let radius_key = push_cstr(&mut heap, "LightRadius");
        let scale_key = push_cstr(&mut heap, "IntensityScale");
        buf.extend_from_slice(&5u32.to_le_bytes());
        buf.extend_from_slice(&(heap.len() as u32).to_le_bytes());
        buf.extend_from_slice(&heap);
        write_prop(buf, name_key, PROP_STRING, name_val as u32);
        write_prop(buf, pos_key, PROP_VECTOR, pos_val as u32);
        write_prop(buf, col_key, PROP_COLOUR, col_val as u32);
        write_prop(buf, radius_key, PROP_FLOAT, radius.to_bits());
        write_prop(buf, scale_key, PROP_FLOAT, scale.to_bits());
    }

    fn write_world_model(buf: &mut Vec<u8>, name: &str, hidden: bool) {
        write_lt(buf, "WorldModel");
        let mut heap = Vec::new();
        let name_key = push_cstr(&mut heap, "Name");
        let name_val = push_cstr(&mut heap, name);
        let pos_key = push_cstr(&mut heap, "Pos");
        let pos_val = heap.len();
        write_f32s(&mut heap, &[10.0, 20.0, 30.0]);
        let rot_key = push_cstr(&mut heap, "Rotation");
        let rot_val = heap.len();
        write_f32s(&mut heap, &[0.0, 0.0, 0.0, 1.0]);
        let hide_key = push_cstr(&mut heap, "StartHidden");
        buf.extend_from_slice(&4u32.to_le_bytes());
        buf.extend_from_slice(&(heap.len() as u32).to_le_bytes());
        buf.extend_from_slice(&heap);
        write_prop(buf, name_key, PROP_STRING, name_val as u32);
        write_prop(buf, pos_key, PROP_VECTOR, pos_val as u32);
        write_prop(buf, rot_key, PROP_QUAT, rot_val as u32);
        write_prop(buf, hide_key, PROP_INT, if hidden { 1 } else { 0 });
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

    fn write_f32s(buf: &mut Vec<u8>, vals: &[f32]) {
        for v in vals {
            buf.extend_from_slice(&v.to_le_bytes());
        }
    }

    fn write_prop(buf: &mut Vec<u8>, name_off: u32, kind: u32, data: u32) {
        buf.extend_from_slice(&name_off.to_le_bytes());
        buf.extend_from_slice(&kind.to_le_bytes());
        buf.extend_from_slice(&data.to_le_bytes());
    }
}
