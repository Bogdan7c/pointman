mod gamepad;

use anyhow::Context;
use gamepad::Devices;
use glam::{Vec2, Vec3};
use pointman_assets::{archive_key, DdsFormat, DdsImage, Material, WorldModels, WorldRender};
use pointman_engine::{LevelDraw, Simulation};
use pointman_game::{AssetIndex, Config, GameMount, INTRO_WORLD};
use pointman_render::{Renderer, TextureFormat, TextureId, TextureUpload, Vertex};
use std::collections::HashMap;
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
    mount: Option<GameMount>,
}

impl App {
    fn new(mount: Option<GameMount>) -> Self {
        Self {
            window: None,
            renderer: None,
            sim: Simulation::new(),
            devices: Devices::new(),
            last: Instant::now(),
            mouse_captured: false,
            mount,
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
            Ok(mut renderer) => {
                log::info!("Vulkan deferred renderer ready");
                if let Some(mount) = self.mount.as_ref() {
                    load_intro(&mut renderer, &mut self.sim, mount);
                }
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
    let mount = GameMount::from_config(&cfg);
    if let Some(mount) = mount.as_ref() {
        mount.log_inventory();
        mount.catalog().log_summary();
    } else {
        log::warn!(
            "F.E.A.R. install not found. Set POINTMAN_GAME_ROOT or copy pointman.toml.example → pointman.toml"
        );
    }

    let event_loop = EventLoop::new().context("event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new(mount);
    event_loop.run_app(&mut app).context("run")?;
    Ok(())
}

fn load_intro(renderer: &mut Renderer, sim: &mut Simulation, mount: &GameMount) {
    match mount.read_file(INTRO_WORLD) {
        Ok(bytes) => match WorldRender::parse(&bytes) {
            Ok(world) => {
            let (verts, indices, draws) = world.flatten();
            log::info!(
                "{INTRO_WORLD}: {} surfaces, {} verts, {} indices, bounds {:?} → {:?}",
                world.surfaces.len(),
                verts.len(),
                indices.len(),
                world.header.min,
                world.header.max
            );
            let gpu: Vec<Vertex> = verts
                .iter()
                .map(|v| Vertex {
                    pos: v.position,
                    normal: v.normal,
                    uv: v.uv,
                })
                .collect();
            match renderer.upload_mesh(&gpu, &indices) {
                Ok(mesh) => {
                    let mut index = mount.index();
                    let mut dds_cache = HashMap::new();
                    let mut mat_cache = HashMap::new();
                    let mut textured = 0u32;
                    let mut fallback = 0u32;
                    let mut level_draws = Vec::with_capacity(draws.len());
                    for draw in draws {
                        let tex = texture_for(
                            &mut index,
                            renderer,
                            &mut mat_cache,
                            &mut dds_cache,
                            &draw.material,
                        );
                        if tex == TextureId::WHITE {
                            fallback += 1;
                        } else {
                            textured += 1;
                        }
                        level_draws.push(LevelDraw {
                            first_index: draw.first_index,
                            index_count: draw.index_count,
                            color: if tex == TextureId::WHITE {
                                draw.color
                            } else {
                                [1.0, 1.0, 1.0, 1.0]
                            },
                            texture: tex,
                        });
                    }
                    log::info!(
                        "intro textures: {} unique dds, {} surfaces textured, {} fallback",
                        dds_cache.len(),
                        textured,
                        fallback
                    );
                    let spawn = verts.first().map(|v| Vec3::from_array(v.position));
                    let triangles = WorldModels::parse(&bytes)
                        .map(|m| {
                            if let Some(bsp) = m.physics() {
                                log::info!(
                                    "PhysicsBSP {}  points {}  polys {}",
                                    bsp.names.join(","),
                                    bsp.points.len(),
                                    bsp.polygons.len()
                                );
                            }
                            m.triangles()
                        })
                        .unwrap_or_else(|err| {
                            log::error!("PhysicsBSP: {err}");
                            Vec::new()
                        });
                    sim.set_level(
                        mesh,
                        level_draws,
                        world.header.min,
                        world.header.max,
                        spawn,
                        triangles,
                    );
                }
                Err(err) => log::error!("upload {INTRO_WORLD}: {err}"),
            }
            }
            Err(err) => log::error!("parse {INTRO_WORLD}: {err}"),
        },
        Err(err) => log::error!("load {INTRO_WORLD}: {err}"),
    }
}

fn texture_for(
    index: &mut AssetIndex,
    renderer: &mut Renderer,
    mat_cache: &mut HashMap<String, TextureId>,
    dds_cache: &mut HashMap<String, TextureId>,
    material: &str,
) -> TextureId {
    if let Some(id) = mat_cache.get(material) {
        return *id;
    }
    let id = load_diffuse(index, renderer, dds_cache, material).unwrap_or(TextureId::WHITE);
    mat_cache.insert(material.to_string(), id);
    id
}

fn load_diffuse(
    index: &mut AssetIndex,
    renderer: &mut Renderer,
    dds_cache: &mut HashMap<String, TextureId>,
    material: &str,
) -> anyhow::Result<TextureId> {
    let mat = Material::parse(&index.read(&archive_key(material))?)?;
    let diffuse = mat
        .diffuse_map()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("no tDiffuseMap in {material}"))?;
    let mut key = archive_key(diffuse);
    if !key.to_ascii_lowercase().ends_with(".dds") {
        key.push_str(".dds");
    }
    if let Some(id) = dds_cache.get(&key) {
        return Ok(*id);
    }
    let dds = DdsImage::parse(&index.read(&key)?)?;
    let format = match dds.format {
        DdsFormat::Bc1 => TextureFormat::Bc1,
        DdsFormat::Bc2 => TextureFormat::Bc2,
        DdsFormat::Bc3 => TextureFormat::Bc3,
    };
    let id = renderer.upload_texture(TextureUpload {
        width: dds.width,
        height: dds.height,
        mip_count: dds.mip_count,
        format,
        bytes: &dds.bytes,
    })?;
    dds_cache.insert(key, id);
    Ok(id)
}
