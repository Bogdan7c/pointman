use bytemuck::{Pod, Zeroable};
use glam::Vec3;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

pub fn cube() -> (Vec<Vertex>, Vec<u16>) {
    let faces = [
        ([0.0, 0.0, 1.0], [[-1, -1, 1], [1, -1, 1], [1, 1, 1], [-1, 1, 1]]),
        ([0.0, 0.0, -1.0], [[1, -1, -1], [-1, -1, -1], [-1, 1, -1], [1, 1, -1]]),
        ([1.0, 0.0, 0.0], [[1, -1, 1], [1, -1, -1], [1, 1, -1], [1, 1, 1]]),
        ([-1.0, 0.0, 0.0], [[-1, -1, -1], [-1, -1, 1], [-1, 1, 1], [-1, 1, -1]]),
        ([0.0, 1.0, 0.0], [[-1, 1, 1], [1, 1, 1], [1, 1, -1], [-1, 1, -1]]),
        ([0.0, -1.0, 0.0], [[-1, -1, -1], [1, -1, -1], [1, -1, 1], [-1, -1, 1]]),
    ];
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for (n, corners) in faces {
        let base = vertices.len() as u16;
        for c in corners {
            vertices.push(Vertex {
                pos: [c[0] as f32 * 0.5, c[1] as f32 * 0.5, c[2] as f32 * 0.5],
                normal: n,
                uv: [0.0, 0.0],
            });
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (vertices, indices)
}

pub fn corridor_boxes() -> Vec<(Vec3, Vec3, [f32; 4])> {
    // (center, scale, rgba) — a dark office hallway until World00p lands.
    let wall = [0.22, 0.20, 0.18, 1.0];
    let floor = [0.12, 0.11, 0.10, 1.0];
    let ceil = [0.16, 0.16, 0.17, 1.0];
    let crate_c = [0.35, 0.22, 0.12, 1.0];
    vec![
        (Vec3::new(0.0, 0.0, 0.0), Vec3::new(6.0, 0.2, 40.0), floor),
        (Vec3::new(0.0, 3.2, 0.0), Vec3::new(6.0, 0.2, 40.0), ceil),
        (Vec3::new(-3.0, 1.6, 0.0), Vec3::new(0.2, 3.2, 40.0), wall),
        (Vec3::new(3.0, 1.6, 0.0), Vec3::new(0.2, 3.2, 40.0), wall),
        (Vec3::new(0.0, 1.6, -20.0), Vec3::new(6.0, 3.2, 0.2), wall),
        (Vec3::new(-1.6, 0.5, -4.0), Vec3::new(1.2, 1.0, 1.2), crate_c),
        (Vec3::new(1.8, 0.4, -9.0), Vec3::new(0.8, 0.8, 1.6), crate_c),
        (Vec3::new(-1.2, 0.7, -14.0), Vec3::new(1.4, 1.4, 1.0), crate_c),
    ]
}
