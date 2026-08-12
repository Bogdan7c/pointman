mod backend;
mod camera;
mod error;
mod mesh;
mod texture;

pub use backend::Renderer;
pub use camera::Camera;
pub use error::RenderError;
pub use mesh::{corridor_boxes, cube, Vertex};
pub use texture::{TextureFormat, TextureId, TextureUpload};

use glam::{Mat4, Vec3, Vec4};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct MeshId(pub u32);

impl MeshId {
    pub const CUBE: Self = Self(0);
}

#[derive(Clone, Debug)]
pub struct MeshInstance {
    pub mesh: MeshId,
    pub first_index: u32,
    pub index_count: u32,
    pub transform: Mat4,
    pub color: Vec4,
    pub texture: TextureId,
}

impl MeshInstance {
    pub fn new(mesh: MeshId, transform: Mat4, color: Vec4) -> Self {
        Self {
            mesh,
            first_index: 0,
            index_count: 0,
            transform,
            color,
            texture: TextureId::WHITE,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PointLight {
    pub position: Vec3,
    pub radius: f32,
    pub color: Vec3,
    pub intensity: f32,
}

#[derive(Clone, Debug)]
pub struct DrawList {
    pub camera: Camera,
    pub instances: Vec<MeshInstance>,
    pub lights: Vec<PointLight>,
}
