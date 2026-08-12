//! Capsule vs PhysicsBSP triangles. LithTech centimetres, Y-up.

use glam::Vec3;
use std::collections::{HashMap, HashSet};

const CELL: f32 = 256.0;
const ITERATIONS: u32 = 4;

#[derive(Clone, Debug)]
pub struct ClipMesh {
    triangles: Vec<[Vec3; 3]>,
    cells: HashMap<(i32, i32, i32), Vec<u32>>,
}

impl ClipMesh {
    pub fn from_triangles(triangles: Vec<[Vec3; 3]>) -> Self {
        let mut cells: HashMap<(i32, i32, i32), Vec<u32>> = HashMap::new();
        for (i, tri) in triangles.iter().enumerate() {
            let min = tri[0].min(tri[1]).min(tri[2]);
            let max = tri[0].max(tri[1]).max(tri[2]);
            for key in cell_range(min, max) {
                cells.entry(key).or_default().push(i as u32);
            }
        }
        Self { triangles, cells }
    }

    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    pub fn move_eye(
        &self,
        mut eye: Vec3,
        wish: Vec3,
        radius: f32,
        eye_height: f32,
        dt: f32,
        gravity: f32,
        vertical_speed: &mut f32,
    ) -> Vec3 {
        *vertical_speed -= gravity * dt;
        let mut vel = wish;
        vel.y += *vertical_speed;
        let step_dt = dt / ITERATIONS as f32;
        for _ in 0..ITERATIONS {
            let stepped = eye + vel * step_dt;
            eye = stepped;
            self.resolve(&mut eye, radius, eye_height);
            if vel.y < 0.0 && eye.y > stepped.y + 0.05 {
                *vertical_speed = 0.0;
                vel.y = 0.0;
            }
        }
        eye
    }

    fn resolve(&self, eye: &mut Vec3, radius: f32, eye_height: f32) {
        let bottom = Vec3::new(eye.x, eye.y - eye_height + radius, eye.z);
        let top = Vec3::new(eye.x, eye.y - radius * 0.25, eye.z);
        let mid = (bottom + top) * 0.5;
        let min = bottom.min(top) - Vec3::splat(radius);
        let max = bottom.max(top) + Vec3::splat(radius);
        let mut seen = HashSet::new();
        let mut push = Vec3::ZERO;
        for key in cell_range(min, max) {
            let Some(list) = self.cells.get(&key) else {
                continue;
            };
            for &idx in list {
                if !seen.insert(idx) {
                    continue;
                }
                let tri = self.triangles[idx as usize];
                for center in [bottom, mid, top] {
                    if let Some(delta) = sphere_push(center, radius, tri) {
                        push += delta;
                    }
                }
            }
        }
        if push.length_squared() > 0.0 {
            *eye += push;
        }
    }
}

fn cell_range(min: Vec3, max: Vec3) -> impl Iterator<Item = (i32, i32, i32)> {
    let x0 = (min.x / CELL).floor() as i32;
    let y0 = (min.y / CELL).floor() as i32;
    let z0 = (min.z / CELL).floor() as i32;
    let x1 = (max.x / CELL).floor() as i32;
    let y1 = (max.y / CELL).floor() as i32;
    let z1 = (max.z / CELL).floor() as i32;
    (x0..=x1).flat_map(move |x| {
        (y0..=y1).flat_map(move |y| (z0..=z1).map(move |z| (x, y, z)))
    })
}

fn sphere_push(center: Vec3, radius: f32, tri: [Vec3; 3]) -> Option<Vec3> {
    let p = closest_on_triangle(center, tri);
    let delta = center - p;
    let dist = delta.length();
    if dist >= radius {
        return None;
    }
    if dist < 1e-4 {
        let n = (tri[1] - tri[0]).cross(tri[2] - tri[0]);
        if n.length_squared() < 1e-8 {
            return None;
        }
        return Some(n.normalize() * (radius - dist).max(0.01));
    }
    Some(delta / dist * (radius - dist))
}

fn closest_on_triangle(p: Vec3, tri: [Vec3; 3]) -> Vec3 {
    let [a, b, c] = tri;
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return a + ab * v;
    }
    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return a + ac * w;
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return b + (c - b) * w;
    }
    let denom = 1.0 / (va + vb + vc);
    a + ab * (vb * denom) + ac * (vc * denom)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wall_and_floor() -> ClipMesh {
        let floor = [
            Vec3::new(-200.0, 0.0, -200.0),
            Vec3::new(200.0, 0.0, -200.0),
            Vec3::new(200.0, 0.0, 200.0),
        ];
        let floor2 = [
            Vec3::new(-200.0, 0.0, -200.0),
            Vec3::new(200.0, 0.0, 200.0),
            Vec3::new(-200.0, 0.0, 200.0),
        ];
        let wall = [
            Vec3::new(80.0, 0.0, -80.0),
            Vec3::new(80.0, 200.0, -80.0),
            Vec3::new(80.0, 200.0, 80.0),
        ];
        let wall2 = [
            Vec3::new(80.0, 0.0, -80.0),
            Vec3::new(80.0, 200.0, 80.0),
            Vec3::new(80.0, 0.0, 80.0),
        ];
        ClipMesh::from_triangles(vec![floor, floor2, wall, wall2])
    }

    #[test]
    fn does_not_walk_through_wall() {
        let mesh = wall_and_floor();
        let radius = 40.0;
        let eye_h = 160.0;
        let mut eye = Vec3::new(0.0, 160.0, 0.0);
        let mut vy = 0.0;
        for _ in 0..40 {
            eye = mesh.move_eye(
                eye,
                Vec3::new(400.0, 0.0, 0.0),
                radius,
                eye_h,
                0.05,
                0.0,
                &mut vy,
            );
        }
        assert!(
            eye.x < 80.0 - radius + 4.0,
            "walked through wall to x={}",
            eye.x
        );
        assert!(eye.x > 0.0, "did not move at all");
    }

    #[test]
    fn lands_on_floor() {
        let mesh = wall_and_floor();
        let mut eye = Vec3::new(0.0, 400.0, 0.0);
        let mut vy = 0.0;
        for _ in 0..45 {
            eye = mesh.move_eye(eye, Vec3::ZERO, 40.0, 160.0, 1.0 / 30.0, 980.0, &mut vy);
        }
        assert!(
            (eye.y - 160.0).abs() < 24.0,
            "expected eye near 160 after landing, got {}",
            eye.y
        );
    }
}
