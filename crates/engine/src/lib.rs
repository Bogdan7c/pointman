mod input;

pub use input::{Input, xbox360};

use glam::{Mat4, Vec3, Vec4};
use pointman_ai::replica::{self, ALERT, HAS_WEAPON, TARGET_VISIBLE, WEAPON_LOADED};
use pointman_ai::{Goal, Plan, Planner, WorldState};
use pointman_render::{corridor_boxes, Camera, DrawList, MeshInstance, PointLight};

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
}

impl Simulation {
    pub fn new() -> Self {
        let world = WorldState::from_pairs(&[
            (ALERT, 0),
            (HAS_WEAPON, 1),
            (WEAPON_LOADED, 1),
        ]);
        let planner = replica::planner();
        let goals = vec![replica::kill_enemy(), replica::investigate(), replica::patrol()];
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
        if wish.length_squared() > 0.0 {
            let speed = if self.crouch {
                2.2
            } else if input.move_axis.length() > 0.9 {
                4.2
            } else {
                2.4
            };
            self.camera.position += wish.normalize() * speed * dt * input.move_axis.length().min(1.0);
        }
        self.camera.position.y = if self.crouch { 1.05 } else { 1.6 };

        let dist = self.camera.position.distance(self.replica.position);
        let alert = if dist < 6.0 {
            2
        } else if dist < 14.0 {
            1
        } else {
            0
        };
        self.replica.world.set(ALERT, alert);
        self.replica.world.set(TARGET_VISIBLE, i32::from(dist < 10.0));
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
                if dir.length_squared() > 1.0 {
                    self.replica.position += dir.normalize() * 1.6 * dt;
                }
            }
        }

        input.clear_edges();
    }

    pub fn draw_list(&self) -> DrawList {
        let mut instances: Vec<MeshInstance> = corridor_boxes()
            .into_iter()
            .map(|(center, scale, color)| MeshInstance {
                transform: Mat4::from_translation(center) * Mat4::from_scale(scale),
                color: Vec4::from_array(color),
            })
            .collect();

        let color = match self.replica.plan.as_ref().map(|p| p.goal) {
            Some("KillEnemy") => Vec4::new(0.85, 0.12, 0.10, 1.0),
            Some("InvestigateDisturbance") => Vec4::new(0.85, 0.7, 0.15, 1.0),
            _ => Vec4::new(0.25, 0.4, 0.75, 1.0),
        };
        instances.push(MeshInstance {
            transform: Mat4::from_translation(self.replica.position)
                * Mat4::from_scale(Vec3::new(0.6, 1.8, 0.6)),
            color,
        });

        let flashlight = self.camera.position + self.camera.forward() * 0.4;
        let flash_i = if self.flashlight { 18.0 } else { 0.0 };
        DrawList {
            camera: self.camera.clone(),
            instances,
            lights: vec![
                PointLight {
                    position: flashlight,
                    radius: 14.0,
                    color: Vec3::new(1.0, 0.95, 0.85),
                    intensity: flash_i,
                },
                PointLight {
                    position: Vec3::new(0.0, 2.4, -6.0),
                    radius: 8.0,
                    color: Vec3::new(1.0, 0.55, 0.25),
                    intensity: 4.0 + (self.time * 6.0).sin().abs() * 2.0,
                },
                PointLight {
                    position: self.replica.position + Vec3::Y * 0.6,
                    radius: 5.0,
                    color: Vec3::new(0.4, 0.7, 1.0),
                    intensity: 2.5,
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
