mod clip;
mod input;
mod player;

pub use clip::ClipMesh;
pub use input::{xbox360, Input};
pub use player::{CROUCH_SPEED_CM, JUMP_VEL_CM, RUN_SPEED_CM};

use glam::{Mat4, Vec3, Vec4};
use pointman_ai::replica::{self, ALERT, HAS_WEAPON, TARGET_VISIBLE, WEAPON_LOADED};
use pointman_ai::{Goal, Plan, Planner, WorldState};
use pointman_render::{
    corridor_boxes, Camera, CubemapId, DrawList, MeshId, MeshInstance, PointLight, TextureId,
};

pub struct LevelDraw {
    pub first_index: u32,
    pub index_count: u32,
    pub color: [f32; 4],
    pub albedo: TextureId,
    pub normal: TextureId,
    pub spec: TextureId,
    pub spec_power: f32,
}

pub struct LevelLight {
    pub position: Vec3,
    pub radius: f32,
    pub color: Vec3,
}

/// BSP WorldModel, поставленный в мир. Пока без своих Mat00 — цвет-хеш, чтобы дыры были видны.
pub struct LevelProp {
    pub mesh: MeshId,
    pub transform: Mat4,
    pub color: [f32; 4],
}

struct LoadedLevel {
    mesh: MeshId,
    draws: Vec<LevelDraw>,
    props: Vec<LevelProp>,
    clip: Option<ClipMesh>,
    lights: Vec<LevelLight>,
    ambient: Vec3,
    sky: Option<CubemapId>,
}

pub struct Replica {
    pub position: Vec3,
    pub world: WorldState,
    pub plan: Option<Plan>,
    pub timer: f32,
}

pub struct Simulation {
    pub camera: Camera,
    pub replica: Replica,
    planner: Planner,
    goals: Vec<Goal>,
    time: f32,
    last_plan_log: String,
    crouch: bool,
    flashlight: bool,
    slowmo: bool,
    /// 1.0 = demo corridor (metres). 100.0 ≈ LithTech centimetres.
    unit: f32,
    /// World-space floor used for eye height (LithTech Y-up).
    floor_y: f32,
    vertical_speed: f32,
    /// Стоим на полу (прошлый кадр). Нужен, чтобы прыжок не требовал угадывать Y.
    grounded: bool,
    /// Lean −1..+1. Капсула не едет — смещение только в `draw_list`.
    lean: f32,
    level: Option<LoadedLevel>,
}

impl Simulation {
    pub fn new() -> Self {
        let world = WorldState::from_pairs(&[(ALERT, 0), (HAS_WEAPON, 1), (WEAPON_LOADED, 1)]);
        let planner = replica::planner();
        let goals = vec![
            replica::kill_enemy(),
            replica::investigate(),
            replica::patrol(),
        ];
        let plan = planner.best_goal(&world, &goals).map(|(_, p)| p);
        Self {
            camera: Camera::default(),
            replica: Replica {
                position: Vec3::new(0.0, 1.0, -16.0),
                world,
                plan,
                timer: 0.0,
            },
            planner,
            goals,
            time: 0.0,
            last_plan_log: String::new(),
            crouch: false,
            flashlight: true,
            slowmo: false,
            unit: 1.0,
            floor_y: 0.0,
            vertical_speed: 0.0,
            grounded: true,
            lean: 0.0,
            level: None,
        }
    }

    pub fn set_level(
        &mut self,
        mesh: MeshId,
        draws: Vec<LevelDraw>,
        min: Vec3,
        max: Vec3,
        spawn: Option<Vec3>,
        yaw: Option<f32>,
        triangles: Vec<[Vec3; 3]>,
        lights: Vec<LevelLight>,
        ambient: Vec3,
        props: Vec<LevelProp>,
    ) {
        self.vertical_speed = 0.0;
        self.grounded = true;
        self.lean = 0.0;
        self.crouch = false;
        self.unit = 100.0;
        let clip = if triangles.is_empty() {
            None
        } else {
            log::info!("PhysicsBSP triangles {}", triangles.len());
            Some(ClipMesh::from_triangles(triangles))
        };
        self.level = Some(LoadedLevel {
            mesh,
            draws,
            props,
            clip,
            lights,
            ambient,
            sky: None,
        });
        let center = (min + max) * 0.5;
        let extent = max - min;
        let spawn = spawn.unwrap_or(Vec3::new(center.x, min.y, center.z));
        // GameStartPoint Pos — центр капсулы (~90 см над полом), глаза выше на 70 см.
        self.floor_y = spawn.y - 0.9 * self.unit;
        self.camera.position = Vec3::new(spawn.x, spawn.y + 0.7 * self.unit, spawn.z);
        if let Some(yaw) = yaw {
            self.camera.yaw = yaw;
            self.camera.pitch = 0.0;
        }
        self.camera.z_near = 4.0;
        self.camera.z_far = 12000.0;
        self.replica.position = self.camera.position + Vec3::new(2.0, 0.0, 4.0) * self.unit;
        log::info!(
            "level camera {:?} yaw {:.1}°  lights {}  props {}  ambient {:?}  extent {:?}",
            self.camera.position,
            self.camera.yaw.to_degrees(),
            self.level.as_ref().map(|l| l.lights.len()).unwrap_or(0),
            self.level.as_ref().map(|l| l.props.len()).unwrap_or(0),
            ambient,
            extent
        );
    }

    /// Небо кадра: cubemap из SkyPointer/SkyCube. Вызывать после `set_level`.
    pub fn set_sky(&mut self, sky: Option<CubemapId>) {
        if let Some(level) = self.level.as_mut() {
            level.sky = sky;
        }
    }

    pub fn time_scale(&self) -> f32 {
        if self.slowmo {
            0.35
        } else {
            1.0
        }
    }

    pub fn tick(&mut self, real_dt: f32, input: &mut Input) {
        if input.flashlight {
            self.flashlight = !self.flashlight;
        }
        if input.slowmo {
            self.slowmo = !self.slowmo;
            log::info!("slow-mo {}", if self.slowmo { "on" } else { "off" });
        }
        if input.fire {
            log::debug!("fire");
        }
        if input.grenade {
            log::info!("grenade");
        }

        let dt = real_dt * self.time_scale();
        self.time += dt;
        self.camera.add_look(input.look.x, input.look.y);

        let radius = player::scale_cm(self.unit, player::CAPSULE_RADIUS_CM);
        if input.crouch {
            if self.crouch {
                // Встать, если потолок не зажимает стоячую капсулу.
                let raised = self.camera.position + Vec3::Y * player::stand_raise(self.unit);
                let stand_h = player::eye_height(false, self.unit);
                let fits = self
                    .level
                    .as_ref()
                    .and_then(|level| level.clip.as_ref())
                    .map(|clip| clip.eye_fits(raised, radius, stand_h))
                    .unwrap_or(true);
                if fits {
                    self.crouch = false;
                    self.camera.position = raised;
                }
            } else {
                self.crouch = true;
                self.camera.position.y -= player::stand_raise(self.unit);
            }
        }

        let wish = player::wish_velocity(
            self.camera.forward(),
            self.camera.right(),
            input.move_axis,
            self.crouch,
            self.unit,
        );
        if let Some(impulse) = player::jump_impulse(self.grounded, self.crouch, input.jump, self.unit)
        {
            self.vertical_speed = impulse;
            self.grounded = false;
        }

        let eye_h = player::eye_height(self.crouch, self.unit);
        let gravity = player::scale_cm(self.unit, player::PLAYER_GRAVITY_CM);
        if let Some(clip) = self.level.as_ref().and_then(|l| l.clip.as_ref()) {
            let step = clip.move_eye(
                self.camera.position,
                wish,
                radius,
                eye_h,
                dt,
                gravity,
                &mut self.vertical_speed,
            );
            self.camera.position = step.eye;
            self.grounded = step.grounded;
        } else {
            self.vertical_speed -= gravity * dt;
            self.camera.position += wish * dt;
            self.camera.position.y += self.vertical_speed * dt;
            let min_eye = self.floor_y + eye_h;
            if self.camera.position.y <= min_eye {
                self.camera.position.y = min_eye;
                self.vertical_speed = 0.0;
                self.grounded = true;
            } else {
                self.grounded = false;
            }
        }

        let moving = input.move_axis.length() > 0.05;
        self.lean = player::step_lean(self.lean, input.lean, moving, self.grounded, dt);

        let dist = self.camera.position.distance(self.replica.position);
        let alert = if dist < 6.0 * self.unit {
            2
        } else if dist < 14.0 * self.unit {
            1
        } else {
            0
        };
        self.replica.world.set(ALERT, alert);
        self.replica
            .world
            .set(TARGET_VISIBLE, i32::from(dist < 10.0 * self.unit));
        if alert == 2 && self.time as i32 % 7 == 0 {
            self.replica.world.set(WEAPON_LOADED, 0);
        }

        if let Some((_, plan)) = self.planner.best_goal(&self.replica.world, &self.goals) {
            let key = format!("{}:{}", plan.goal, plan.steps.join(" > "));
            if key != self.last_plan_log {
                log::info!("GOAP [{}] {}", plan.goal, plan.steps.join(" → "));
                self.last_plan_log = key;
            }
            self.replica.plan = Some(plan);
        }

        self.replica.timer += dt;
        if let Some(plan) = &self.replica.plan {
            if plan.steps.iter().any(|s| *s == "Advance" || *s == "Attack") {
                let dir = (self.camera.position - self.replica.position) * Vec3::new(1.0, 0.0, 1.0);
                if dir.length_squared() > 1.0 * self.unit * self.unit {
                    self.replica.position += dir.normalize() * 1.6 * self.unit * dt;
                }
            }
        }

        input.clear_edges();
    }

    pub fn draw_list(&self) -> DrawList {
        let mut instances = Vec::new();
        if let Some(level) = &self.level {
            for draw in &level.draws {
                instances.push(MeshInstance {
                    mesh: level.mesh,
                    first_index: draw.first_index,
                    index_count: draw.index_count,
                    transform: Mat4::IDENTITY,
                    color: Vec4::from_array(draw.color),
                    albedo: draw.albedo,
                    normal: draw.normal,
                    spec: draw.spec,
                    spec_power: draw.spec_power,
                });
            }
            for prop in &level.props {
                instances.push(MeshInstance::new(
                    prop.mesh,
                    prop.transform,
                    Vec4::from_array(prop.color),
                ));
            }
        } else {
            instances.extend(corridor_boxes().into_iter().map(|(center, scale, color)| {
                MeshInstance::new(
                    MeshId::CUBE,
                    Mat4::from_translation(center) * Mat4::from_scale(scale),
                    Vec4::from_array(color),
                )
            }));
            let color = match self.replica.plan.as_ref().map(|p| p.goal) {
                Some("KillEnemy") => Vec4::new(0.85, 0.12, 0.10, 1.0),
                Some("InvestigateDisturbance") => Vec4::new(0.85, 0.7, 0.15, 1.0),
                _ => Vec4::new(0.25, 0.4, 0.75, 1.0),
            };
            instances.push(MeshInstance::new(
                MeshId::CUBE,
                Mat4::from_translation(self.replica.position)
                    * Mat4::from_scale(Vec3::new(0.6, 1.8, 0.6) * self.unit),
                color,
            ));
        }

        let camera = player::apply_lean(&self.camera, self.lean, self.unit);
        let flashlight = camera.position + camera.forward() * 0.4 * self.unit;
        let mut lights = Vec::new();
        if self.flashlight {
            lights.push(PointLight {
                position: flashlight,
                radius: player::scale_cm(self.unit, player::FLASH_RADIUS_CM),
                color: Vec3::new(1.0, 0.95, 0.85),
                intensity: player::FLASH_INTENSITY,
                direction: camera.forward(),
                outer_cos: player::FLASH_OUTER_COS,
            });
        }
        let ambient;
        if let Some(level) = &self.level {
            ambient = level.ambient.max(Vec3::splat(0.08));
            let slots = 8usize.saturating_sub(lights.len());
            lights.extend(nearest_lights(&level.lights, self.camera.position, slots));
        } else {
            ambient = Vec3::splat(0.12);
            lights.push(PointLight::omni(
                self.camera.position + Vec3::Y * 0.8 * self.unit,
                8.0 * self.unit,
                Vec3::new(1.0, 0.55, 0.25),
                4.0 + (self.time * 6.0).sin().abs() * 2.0,
            ));
            lights.push(PointLight::omni(
                self.replica.position + Vec3::Y * 0.6 * self.unit,
                5.0 * self.unit,
                Vec3::new(0.4, 0.7, 1.0),
                2.5,
            ));
        }
        DrawList {
            camera,
            instances,
            lights,
            ambient,
            sky: self.level.as_ref().and_then(|level| level.sky),
        }
    }
}

fn nearest_lights(lights: &[LevelLight], pos: Vec3, n: usize) -> Vec<PointLight> {
    let mut order: Vec<usize> = (0..lights.len()).collect();
    order.sort_by(|&a, &b| {
        lights[a]
            .position
            .distance_squared(pos)
            .total_cmp(&lights[b].position.distance_squared(pos))
    });
    order
        .into_iter()
        .take(n)
        .map(|i| {
            PointLight::omni(
                lights[i].position,
                lights[i].radius,
                lights[i].color,
                1.0,
            )
        })
        .collect()
}

impl Default for Simulation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    fn open_floor() -> Vec<[Vec3; 3]> {
        vec![
            [
                Vec3::new(-2000.0, 0.0, -2000.0),
                Vec3::new(2000.0, 0.0, -2000.0),
                Vec3::new(2000.0, 0.0, 2000.0),
            ],
            [
                Vec3::new(-2000.0, 0.0, -2000.0),
                Vec3::new(2000.0, 0.0, 2000.0),
                Vec3::new(-2000.0, 0.0, 2000.0),
            ],
        ]
    }

    fn spawn_on_floor(sim: &mut Simulation) {
        sim.set_level(
            MeshId::CUBE,
            vec![],
            Vec3::new(-2000.0, 0.0, -2000.0),
            Vec3::new(2000.0, 400.0, 2000.0),
            Some(Vec3::new(0.0, 90.0, 0.0)),
            Some(0.0),
            open_floor(),
            vec![],
            Vec3::splat(0.1),
            vec![],
        );
    }

    fn tick_frames(sim: &mut Simulation, input: &mut Input, frames: u32) {
        for _ in 0..frames {
            sim.tick(1.0 / 30.0, input);
        }
    }

    #[test]
    fn nearest_lights_picks_closest() {
        let lights = vec![
            LevelLight {
                position: Vec3::new(1000.0, 0.0, 0.0),
                radius: 100.0,
                color: Vec3::ONE,
            },
            LevelLight {
                position: Vec3::new(10.0, 0.0, 0.0),
                radius: 100.0,
                color: Vec3::X,
            },
            LevelLight {
                position: Vec3::new(50.0, 0.0, 0.0),
                radius: 100.0,
                color: Vec3::Y,
            },
        ];
        let picked = nearest_lights(&lights, Vec3::ZERO, 2);
        assert_eq!(picked.len(), 2);
        assert_eq!(picked[0].color, Vec3::X);
        assert_eq!(picked[1].color, Vec3::Y);
    }

    #[test]
    fn full_stick_walks_about_400_cm_per_second() {
        let mut sim = Simulation::new();
        spawn_on_floor(&mut sim);
        let start = sim.camera.position;
        let mut input = Input::default();
        input.move_axis = Vec2::new(0.0, 1.0);
        tick_frames(&mut sim, &mut input, 30);
        let dx = sim.camera.position.x - start.x;
        let dz = sim.camera.position.z - start.z;
        let dist = (dx * dx + dz * dz).sqrt();
        assert!(
            (dist - 400.0).abs() < 50.0,
            "expected ~400 cm in 1s, got {dist}"
        );
    }

    #[test]
    fn crouch_lowers_eye_to_105_cm() {
        let mut sim = Simulation::new();
        spawn_on_floor(&mut sim);
        let mut input = Input::default();
        input.crouch = true;
        sim.tick(1.0 / 30.0, &mut input);
        assert!(
            (sim.camera.position.y - 105.0).abs() < 8.0,
            "crouch eye should be ~105, got {}",
            sim.camera.position.y
        );
    }

    #[test]
    fn jump_leaves_ground_and_lands() {
        let mut sim = Simulation::new();
        spawn_on_floor(&mut sim);
        tick_frames(&mut sim, &mut Input::default(), 5);
        let start_y = sim.camera.position.y;
        let mut input = Input::default();
        input.jump = true;
        sim.tick(1.0 / 30.0, &mut input);
        assert!(
            sim.camera.position.y > start_y + 4.0,
            "jump must lift the camera, start {start_y} now {}",
            sim.camera.position.y
        );
        assert!(!sim.grounded);
        tick_frames(&mut sim, &mut Input::default(), 60);
        assert!(sim.grounded, "must land after the hop");
        assert!(
            (sim.camera.position.y - 160.0).abs() < 24.0,
            "landed eye should be ~160, got {}",
            sim.camera.position.y
        );
    }

    #[test]
    fn lean_offsets_camera_not_capsule() {
        let mut sim = Simulation::new();
        spawn_on_floor(&mut sim);
        let eye = sim.camera.position;
        let mut input = Input::default();
        input.lean = 1.0;
        for _ in 0..15 {
            sim.tick(0.02, &mut input);
        }
        let drawn = sim.draw_list();
        let offset = drawn.camera.position - sim.camera.position;
        assert!(
            offset.length() > 10.0,
            "lean must shift the view camera, offset {offset}"
        );
        assert!(
            drawn.camera.roll.abs() > 0.15,
            "lean must roll the camera, roll {}",
            drawn.camera.roll
        );
        let feet_shift = (sim.camera.position.x - eye.x).abs() + (sim.camera.position.z - eye.z).abs();
        assert!(
            feet_shift < 2.0,
            "lean must not walk the capsule, shift {feet_shift}"
        );
    }

    #[test]
    fn flashlight_is_a_camera_spot() {
        let mut sim = Simulation::new();
        spawn_on_floor(&mut sim);
        let list = sim.draw_list();
        let flash = list
            .lights
            .iter()
            .find(|light| light.outer_cos > 0.0)
            .expect("flashlight should be a spot");
        assert!(
            flash.direction.dot(list.camera.forward()) > 0.95,
            "beam must follow the camera"
        );
        assert!((flash.radius - 1400.0).abs() < 1.0);
    }

    #[test]
    fn world_model_prop_is_drawn() {
        let mut sim = Simulation::new();
        sim.set_level(
            MeshId::CUBE,
            vec![],
            Vec3::ZERO,
            Vec3::ONE,
            Some(Vec3::new(0.0, 90.0, 0.0)),
            Some(0.0),
            vec![],
            vec![],
            Vec3::splat(0.1),
            vec![LevelProp {
                mesh: MeshId::CUBE,
                transform: Mat4::from_translation(Vec3::new(50.0, 0.0, 0.0)),
                color: [1.0, 0.2, 0.1, 1.0],
            }],
        );
        let list = sim.draw_list();
        let prop = list
            .instances
            .iter()
            .find(|inst| (inst.color.x - 1.0).abs() < 0.01)
            .expect("world model instance missing from draw list");
        let translation = prop.transform.to_scale_rotation_translation().2;
        assert!((translation.x - 50.0).abs() < 0.1);
        assert!(list.sky.is_none(), "sky must stay off until set_sky");
    }

    #[test]
    fn sky_cubemap_reaches_draw_list() {
        let mut sim = Simulation::new();
        sim.set_level(
            MeshId::CUBE,
            vec![],
            Vec3::ZERO,
            Vec3::ONE,
            Some(Vec3::new(0.0, 90.0, 0.0)),
            Some(0.0),
            vec![],
            vec![],
            Vec3::splat(0.1),
            vec![],
        );
        sim.set_sky(Some(CubemapId::SKY));
        let list = sim.draw_list();
        assert_eq!(list.sky, Some(CubemapId::SKY));
        sim.tick(0.016, &mut Input::default());
        assert_eq!(sim.draw_list().sky, Some(CubemapId::SKY));
    }
}
