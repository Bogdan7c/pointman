mod gamepad;

use anyhow::Context;
use gamepad::Devices;
use glam::Vec2;
use pointman_engine::Simulation;
use pointman_game::{Config, GameMount};
use pointman_render::Renderer;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

struct App {
    window: Option<Window>,
    renderer: Option<Renderer>,
    sim: Simulation,
    devices: Devices,
    last: Instant,
    mouse_captured: bool,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            renderer: None,
            sim: Simulation::new(),
            devices: Devices::new(),
            last: Instant::now(),
            mouse_captured: false,
        }
    }

    fn capture_mouse(&mut self, grab: bool) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        self.mouse_captured = grab;
        let _ = window.set_cursor_grab(if grab {
            CursorGrabMode::Confined
        } else {
            CursorGrabMode::None
        });
        window.set_cursor_visible(!grab);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("POINTMAN — F.E.A.R. native")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0));
        let window = event_loop.create_window(attrs).expect("create window");
        match Renderer::new(&window) {
            Ok(renderer) => {
                log::info!("Vulkan deferred renderer ready");
                self.renderer = Some(renderer);
            }
            Err(err) => {
                log::error!("renderer init failed: {err:#}");
                event_loop.exit();
                return;
            }
        }
        self.window = Some(window);
        self.last = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    let _ = renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            self.devices.keys.insert(code);
                            self.devices.key_edge(code);
                            if code == KeyCode::Escape {
                                event_loop.exit();
                            }
                        }
                        ElementState::Released => {
                            self.devices.keys.remove(&code);
                        }
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => match (button, state) {
                (MouseButton::Left, ElementState::Pressed) => {
                    if self.mouse_captured {
                        self.devices.mouse_fire = true;
                    } else {
                        self.capture_mouse(true);
                    }
                }
                (MouseButton::Left, ElementState::Released) => self.devices.mouse_fire = false,
                _ => {}
            },
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last).as_secs_f32().min(0.05);
                self.last = now;
                let mut input = self.devices.collect(dt, self.mouse_captured);
                self.sim.tick(dt, &mut input);
                if let Some(renderer) = self.renderer.as_mut() {
                    if let Err(err) = renderer.draw(&self.sim.draw_list()) {
                        log::error!("draw: {err}");
                    }
                }
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.devices.mouse_look += Vec2::new(delta.0 as f32, delta.1 as f32);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("Pointman — native F.E.A.R. engine (Vulkan)");
    let cfg = Config::load();
    if let Some(mount) = GameMount::from_config(&cfg) {
        mount.log_inventory();
    } else {
        log::warn!(
            "F.E.A.R. install not found. Set POINTMAN_GAME_ROOT or copy pointman.toml.example → pointman.toml"
        );
    }

    let event_loop = EventLoop::new().context("event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new();
    event_loop.run_app(&mut app).context("run")?;
    Ok(())
}
