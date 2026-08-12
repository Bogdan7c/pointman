use glam::Vec2;

/// Device-agnostic player intent. Keyboard and Xbox 360 pad both write here.
#[derive(Clone, Debug, Default)]
pub struct Input {
    /// X = strafe right, Y = forward. Magnitude 1 is a full stick/WASD.
    pub move_axis: Vec2,
    /// Look delta this frame, already scaled (mouse pixels or stick*dt*sens).
    pub look: Vec2,
    pub jump: bool,
    pub crouch: bool,
    pub fire: bool,
    pub grenade: bool,
    pub slowmo: bool,
    pub reload: bool,
    pub use_action: bool,
    pub melee: bool,
    pub holster: bool,
    pub medkit: bool,
    pub next_weapon: bool,
    pub next_grenade: bool,
    pub flashlight: bool,
    /// -1 lean left, +1 lean right
    pub lean: f32,
    pub ads: bool,
    pub pause: bool,
    pub objectives: bool,
}

impl Input {
    pub fn clear_edges(&mut self) {
        self.look = Vec2::ZERO;
        self.jump = false;
        self.reload = false;
        self.use_action = false;
        self.melee = false;
        self.holster = false;
        self.medkit = false;
        self.next_weapon = false;
        self.next_grenade = false;
        self.flashlight = false;
        self.pause = false;
        self.objectives = false;
        self.grenade = false;
        self.slowmo = false;
    }
}

/// Official Xbox 360 F.E.A.R. layout (console SKU).
///
/// | Control | Action |
/// | --- | --- |
/// | LS | move |
/// | RS | look |
/// | RT | fire |
/// | LT | grenade |
/// | RB | next weapon |
/// | LB | slow-mo |
/// | A | jump |
/// | B | melee (hold: holster) |
/// | X | reload (hold: use) |
/// | Y | medkit |
/// | D-pad U/D/L/R | next grenade / flashlight / lean L / lean R |
/// | LS click | crouch |
/// | RS click | ADS |
/// | Start | pause |
/// | Back | objectives |
pub mod xbox360 {
    pub const LOOK_SENS: f32 = 2.4;
    pub const DEADZONE: f32 = 0.22;
    pub const TRIGGER: f32 = 0.35;
}
