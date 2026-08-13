mod backend;
mod blinn;
mod camera;
mod cubemap;
mod error;
mod material;
mod mesh;
mod texture;

pub use backend::Renderer;
pub use blinn::{bgra_to_tangent, blinn_specular, tangent_to_world};
pub use camera::Camera;
pub use cubemap::{CubemapId, CubemapUpload};
pub use error::RenderError;
pub use mesh::{corridor_boxes, cube, tbn_from_normal, Vertex};
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
    pub albedo: TextureId,
    pub normal: TextureId,
    pub spec: TextureId,
    pub spec_power: f32,
}

impl MeshInstance {
    pub fn new(mesh: MeshId, transform: Mat4, color: Vec4) -> Self {
        Self {
            mesh,
            first_index: 0,
            index_count: 0,
            transform,
            color,
            albedo: TextureId::WHITE,
            normal: TextureId::FLAT_NORMAL,
            spec: TextureId::BLACK_SPEC,
            spec_power: 64.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PointLight {
    pub position: Vec3,
    pub radius: f32,
    pub color: Vec3,
    pub intensity: f32,
    /// Куда светит спот. `Vec3::ZERO` + `outer_cos == 0` = обычная точка (лампы мира).
    pub direction: Vec3,
    /// cos половины внешнего угла конуса. 0 = omni.
    pub outer_cos: f32,
}

impl PointLight {
    pub fn omni(position: Vec3, radius: f32, color: Vec3, intensity: f32) -> Self {
        Self {
            position,
            radius,
            color,
            intensity,
            direction: Vec3::ZERO,
            outer_cos: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DrawList {
    pub camera: Camera,
    pub instances: Vec<MeshInstance>,
    pub lights: Vec<PointLight>,
    pub ambient: Vec3,
    /// Cubemap неба. `None` — пустые пиксели остаются dim ambient, как раньше.
    pub sky: Option<CubemapId>,
}
