mod backend;
mod camera;
mod error;
mod mesh;

pub use backend::Renderer;
pub use camera::Camera;
pub use error::RenderError;
pub use mesh::{corridor_boxes, cube, Vertex};

use glam::{Mat4, Vec3, Vec4};

#[derive(Clone, Debug)]
pub struct MeshInstance {
    pub transform: Mat4,
    pub color: Vec4,
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
