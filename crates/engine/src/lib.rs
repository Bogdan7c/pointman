mod clip;
mod input;

pub use clip::ClipMesh;
pub use input::{xbox360, Input};

use glam::{Mat4, Vec3, Vec4};
use pointman_ai::replica::{self, ALERT, HAS_WEAPON, TARGET_VISIBLE, WEAPON_LOADED};
use pointman_ai::{Goal, Plan, Planner, WorldState};
use pointman_render::{
    corridor_boxes, Camera, DrawList, MeshId, MeshInstance, PointLight, TextureId,
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

struct LoadedLevel {
    mesh: MeshId,
    draws: Vec<LevelDraw>,
    clip: Option<ClipMesh>,
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
        triangles: Vec<[Vec3; 3]>,
    ) {
        self.vertical_speed = 0.0;
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
            clip,
        });
        let center = (min + max) * 0.5;
        let extent = max - min;
        let spawn = spawn.unwrap_or(Vec3::new(center.x, min.y, center.z));
        self.floor_y = spawn.y;
        self.camera.position = Vec3::new(spawn.x, spawn.y + 1.6 * self.unit, spawn.z);
        self.camera.z_near = 4.0;
        self.camera.z_far = 12000.0;
        self.replica.position = self.camera.position + Vec3::new(2.0, 0.0, 4.0) * self.unit;
        log::info!(
            "level camera {:?}  extent {:?}  z_far {:.0}  surfaces {}",
            self.camera.position,
            extent,
            self.camera.z_far,
            self.level.as_ref().map(|l| l.draws.len()).unwrap_or(0)
        );
    }

    pub fn time_scale(&self) -> f32 {
        if self.slowmo {
            0.35
        } else {
            1.0
        }
    }

    pub fn tick(&mut self, real_dt: f32, input: &mut Input) {
        if input.crouch {
            self.crouch = !self.crouch;
        }
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
        if input.jump {
            log::debug!("jump");
        }

        let dt = real_dt * self.time_scale();
        self.time += dt;
        self.camera.add_look(input.look.x, input.look.y);

        let mut wish = self.camera.forward() * input.move_axis.y + self.camera.right() * input.move_axis.x;
        wish.y = 0.0;
        let speed = if self.crouch {
            2.2
        } else if input.move_axis.length() > 0.9 {
            4.2
        } else {
            2.4
        } * self.unit;
        if wish.length_squared() > 0.0 {
            wish = wish.normalize() * speed * input.move_axis.length().min(1.0);
        }
        let eye_h = if self.crouch {
            1.05 * self.unit
        } else {
            1.6 * self.unit
        };
        if let Some(clip) = self.level.as_ref().and_then(|l| l.clip.as_ref()) {
            self.camera.position = clip.move_eye(
                self.camera.position,
                wish,
                0.40 * self.unit,
                eye_h,
                dt,
                9.8 * self.unit,
                &mut self.vertical_speed,
            );
        } else {
            self.camera.position += wish * dt;
            self.camera.position.y = self.floor_y + eye_h;
        }

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
        } else {
            instances.extend(corridor_boxes().into_iter().map(|(center, scale, color)| {
                MeshInstance::new(
                    MeshId::CUBE,
                    Mat4::from_translation(center) * Mat4::from_scale(scale),
                    Vec4::from_array(color),
                )
            }));
        }

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

        let flashlight = self.camera.position + self.camera.forward() * 0.4 * self.unit;
        let flash_i = if self.flashlight { 18.0 } else { 0.0 };
        DrawList {
            camera: self.camera.clone(),
            instances,
            lights: vec![
                PointLight {
                    position: flashlight,
                    radius: 14.0 * self.unit,
                    color: Vec3::new(1.0, 0.95, 0.85),
                    intensity: flash_i,
                },
                PointLight {
                    position: self.camera.position + Vec3::Y * 0.8 * self.unit,
                    radius: 8.0 * self.unit,
                    color: Vec3::new(1.0, 0.55, 0.25),
                    intensity: 4.0 + (self.time * 6.0).sin().abs() * 2.0,
                },
                PointLight {
                    position: self.replica.position + Vec3::Y * 0.6 * self.unit,
                    radius: 5.0 * self.unit,
                    intensity: 2.5,
                    color: Vec3::new(0.4, 0.7, 1.0),
                },
            ],
        }
    }
}

impl Default for Simulation {
    fn default() -> Self {
        Self::new()
    }
}
