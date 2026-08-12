use gilrs::{Axis, Button, Event, EventType, Gilrs};
use glam::Vec2;
use pointman_engine::xbox360::{DEADZONE, LOOK_SENS, TRIGGER};
use pointman_engine::Input;
use std::collections::HashSet;
use winit::keyboard::KeyCode;

#[derive(Default)]
struct Edges {
    jump: bool,
    crouch: bool,
    flashlight: bool,
    slowmo: bool,
    reload: bool,
    melee: bool,
    medkit: bool,
    grenade: bool,
    next_weapon: bool,
}

pub struct Devices {
    gilrs: Option<Gilrs>,
    pub keys: HashSet<KeyCode>,
    pub mouse_look: Vec2,
    pub mouse_fire: bool,
    edges: Edges,
    logged_pad: bool,
}

impl Devices {
    pub fn new() -> Self {
        let gilrs = match Gilrs::new() {
            Ok(g) => Some(g),
            Err(err) => {
                log::warn!("gamepad: {err}");
                None
            }
        };
        Self {
            gilrs,
            keys: HashSet::new(),
            mouse_look: Vec2::ZERO,
            mouse_fire: false,
            edges: Edges::default(),
            logged_pad: false,
        }
    }

    pub fn key_edge(&mut self, code: KeyCode) {
        match code {
            KeyCode::Space => self.edges.jump = true,
            KeyCode::ControlLeft => self.edges.crouch = true,
            KeyCode::KeyF => self.edges.flashlight = true,
            KeyCode::KeyQ => self.edges.slowmo = true,
            KeyCode::KeyR => self.edges.reload = true,
            KeyCode::KeyV => self.edges.melee = true,
            KeyCode::KeyH => self.edges.medkit = true,
            KeyCode::KeyG => self.edges.grenade = true,
            KeyCode::KeyC => self.edges.next_weapon = true,
            _ => {}
        }
    }

    pub fn collect(&mut self, dt: f32, mouse_captured: bool) -> Input {
        let mut input = Input::default();

        if self.keys.contains(&KeyCode::KeyW) {
            input.move_axis.y += 1.0;
        }
        if self.keys.contains(&KeyCode::KeyS) {
            input.move_axis.y -= 1.0;
        }
        if self.keys.contains(&KeyCode::KeyD) {
            input.move_axis.x += 1.0;
        }
        if self.keys.contains(&KeyCode::KeyA) {
            input.move_axis.x -= 1.0;
        }
        if self.keys.contains(&KeyCode::ArrowLeft) {
            input.lean = -1.0;
        } else if self.keys.contains(&KeyCode::ArrowRight) {
            input.lean = 1.0;
        }
        if input.move_axis.length_squared() > 1.0 {
            input.move_axis = input.move_axis.normalize();
        }
        if mouse_captured {
            input.look += self.mouse_look * 0.0025;
            input.fire |= self.mouse_fire;
        }
        self.mouse_look = Vec2::ZERO;

        input.jump |= self.edges.jump;
        input.crouch |= self.edges.crouch;
        input.flashlight |= self.edges.flashlight;
        input.slowmo |= self.edges.slowmo;
        input.reload |= self.edges.reload;
        input.melee |= self.edges.melee;
        input.medkit |= self.edges.medkit;
        input.grenade |= self.edges.grenade;
        input.next_weapon |= self.edges.next_weapon;
        self.edges = Edges::default();

        if let Some(gilrs) = self.gilrs.as_mut() {
            while let Some(Event { id, event, .. }) = gilrs.next_event() {
                if !self.logged_pad {
                    if let Some(pad) = gilrs.connected_gamepad(id) {
                        log::info!("gamepad: {} (Xbox 360 scheme)", pad.name());
                        self.logged_pad = true;
                    }
                }
                match event {
                    EventType::ButtonPressed(Button::South, _) => input.jump = true,
                    EventType::ButtonPressed(Button::East, _) => input.melee = true,
                    EventType::ButtonPressed(Button::West, _) => input.reload = true,
                    EventType::ButtonPressed(Button::North, _) => input.medkit = true,
                    EventType::ButtonPressed(Button::LeftTrigger, _) => input.slowmo = true,
                    EventType::ButtonPressed(Button::RightTrigger, _) => input.next_weapon = true,
                    EventType::ButtonPressed(Button::DPadDown, _) => input.flashlight = true,
                    EventType::ButtonPressed(Button::DPadUp, _) => input.next_grenade = true,
                    EventType::ButtonPressed(Button::LeftThumb, _) => input.crouch = true,
                    EventType::ButtonPressed(Button::RightThumb, _) => input.ads = true,
                    EventType::ButtonPressed(Button::Start, _) => input.pause = true,
                    EventType::ButtonPressed(Button::Select, _) => input.objectives = true,
                    _ => {}
                }
            }

            if let Some((_id, pad)) = gilrs.gamepads().next() {
                let lx = dead(pad.value(Axis::LeftStickX));
                let ly = dead(pad.value(Axis::LeftStickY));
                let rx = dead(pad.value(Axis::RightStickX));
                let ry = dead(pad.value(Axis::RightStickY));
                input.move_axis.x += lx;
                input.move_axis.y += ly;
                if input.move_axis.length_squared() > 1.0 {
                    input.move_axis = input.move_axis.normalize();
                }
                input.look.x += rx * LOOK_SENS * dt;
                input.look.y += -ry * LOOK_SENS * dt;

                if trigger_value(pad, Axis::LeftZ, Button::LeftTrigger2) > TRIGGER {
                    input.grenade = true;
                }
                if trigger_value(pad, Axis::RightZ, Button::RightTrigger2) > TRIGGER {
                    input.fire = true;
                }
                if pad.is_pressed(Button::DPadLeft) {
                    input.lean = -1.0;
                } else if pad.is_pressed(Button::DPadRight) {
                    input.lean = 1.0;
                }
            }
        }

        input
    }
}

fn dead(v: f32) -> f32 {
    if v.abs() < DEADZONE {
        0.0
    } else {
        v
    }
}

fn trigger_value(pad: gilrs::Gamepad<'_>, axis: Axis, button: Button) -> f32 {
    if pad.is_pressed(button) {
        return 1.0;
    }
    let v = pad.value(axis);
    if v > 0.0 {
        v
    } else {
        0.0
    }
}
