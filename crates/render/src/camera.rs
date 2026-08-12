use glam::{Mat4, Quat, Vec3};

#[derive(Clone, Debug)]
pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    /// Крен для lean. 0 — горизонт ровный. Не крутит капсулу.
    pub roll: f32,
    pub fov_y: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 1.6, 8.0),
            yaw: std::f32::consts::PI,
            pitch: 0.0,
            roll: 0.0,
            fov_y: 75.0_f32.to_radians(),
            z_near: 0.05,
            z_far: 200.0,
        }
    }
}

impl Camera {
    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        )
        .normalize()
    }

    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }

    /// «Вверх» камеры после lean: крутим мировой Y вокруг взгляда.
    pub fn up(&self) -> Vec3 {
        Quat::from_axis_angle(self.forward(), self.roll) * Vec3::Y
    }

    pub fn view(&self) -> Mat4 {
        Mat4::look_to_rh(self.position, self.forward(), self.up())
    }

    pub fn projection(&self, aspect: f32) -> Mat4 {
        let mut p = Mat4::perspective_rh(self.fov_y, aspect, self.z_near, self.z_far);
        p.y_axis.y *= -1.0; // Vulkan clip space
        p
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        self.projection(aspect) * self.view()
    }

    pub fn add_look(&mut self, dx: f32, dy: f32) {
        self.yaw -= dx;
        self.pitch = (self.pitch - dy).clamp(-1.5, 1.5);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roll_tilts_view_without_moving_eye() {
        let mut camera = Camera::default();
        let before = camera.view();
        camera.roll = 20.0_f32.to_radians();
        let after = camera.view();
        assert_eq!(camera.position, Camera::default().position);
        assert!(before != after, "lean roll must change the view matrix");
    }
}
