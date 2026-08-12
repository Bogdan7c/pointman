mod gamepad;

use anyhow::Context;
use gamepad::Devices;
use glam::{Mat4, Vec2};
use pointman_assets::{
    archive_key, material_key, DdsFormat, DdsImage, Material, WorldBsp, WorldModels, WorldObjects,
    WorldRender,
};
use pointman_engine::{LevelDraw, LevelLight, LevelProp, Simulation};
use pointman_game::{AssetIndex, Config, GameMount, INTRO_WORLD};
use pointman_render::{tbn_from_normal, Renderer, TextureFormat, TextureId, TextureUpload, Vertex};
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
                    tangent: v.tangent,
                    binormal: v.binormal,
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
                        let maps = maps_for(
                            &mut index,
                            renderer,
                            &mut mat_cache,
                            &mut dds_cache,
                            &draw.material,
                        );
                        if maps.albedo == TextureId::WHITE {
                            fallback += 1;
                        } else {
                            textured += 1;
                        }
                        level_draws.push(LevelDraw {
                            first_index: draw.first_index,
                            index_count: draw.index_count,
                            color: if maps.albedo == TextureId::WHITE {
                                draw.color
                            } else {
                                [1.0, 1.0, 1.0, 1.0]
                            },
                            albedo: maps.albedo,
                            normal: maps.normal,
                            spec: maps.spec,
                            spec_power: maps.spec_power,
                        });
                    }
                    log::info!(
                        "intro textures: {} unique dds, {} surfaces textured, {} fallback",
                        dds_cache.len(),
                        textured,
                        fallback
                    );
                    let objects = WorldObjects::parse(&bytes).unwrap_or_else(|err| {
                        log::error!("world objects: {err}");
                        WorldObjects::default()
                    });
                    if let Some(start) = objects.spawn() {
                        log::info!(
                            "GameStartPoint {}  {:?}  yaw {:.1}°  lights {}  ambient {:?}",
                            start.name,
                            start.pos,
                            start.yaw.to_degrees(),
                            objects.lights.len(),
                            objects.ambient
                        );
                    }
                    let triangles = WorldModels::parse(&bytes)
                        .map(|models| {
                            if let Some(bsp) = models.physics() {
                                log::info!(
                                    "PhysicsBSP {}  points {}  polys {}",
                                    bsp.names.join(","),
                                    bsp.points.len(),
                                    bsp.polygons.len()
                                );
                            }
                            let props = upload_world_models(renderer, &models, &objects);
                            log::info!(
                                "world models: {} bsp, {} instances",
                                models.models.len(),
                                props.len()
                            );
                            (models.triangles(), props)
                        })
                        .unwrap_or_else(|err| {
                            log::error!("PhysicsBSP: {err}");
                            (Vec::new(), Vec::new())
                        });
                    let (triangles, props) = triangles;
                    sim.set_level(
                        mesh,
                        level_draws,
                        world.header.min,
                        world.header.max,
                        objects.spawn().map(|s| s.pos),
                        objects.spawn().map(|s| s.yaw),
                        triangles,
                        objects
                            .lights
                            .iter()
                            .map(|light| LevelLight {
                                position: light.position,
                                radius: light.radius,
                                color: light.color,
                            })
                            .collect(),
                        objects.ambient,
                        props,
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

fn maps_for(
    index: &mut AssetIndex,
    renderer: &mut Renderer,
    mat_cache: &mut HashMap<String, MaterialMaps>,
    dds_cache: &mut HashMap<String, TextureId>,
    material: &str,
) -> MaterialMaps {
    if let Some(maps) = mat_cache.get(material) {
        return *maps;
    }
    let maps = load_maps(index, renderer, dds_cache, &material_key(material)).unwrap_or_else(|err| {
        log::warn!("material {material}: {err}");
        MaterialMaps::fallback()
    });
    mat_cache.insert(material.to_string(), maps);
    maps
}

#[derive(Clone, Copy)]
struct MaterialMaps {
    albedo: TextureId,
    normal: TextureId,
    spec: TextureId,
    spec_power: f32,
}

impl MaterialMaps {
    fn fallback() -> Self {
        Self {
            albedo: TextureId::WHITE,
            normal: TextureId::FLAT_NORMAL,
            spec: TextureId::BLACK_SPEC,
            spec_power: 64.0,
        }
    }
}

fn load_maps(
    index: &mut AssetIndex,
    renderer: &mut Renderer,
    dds_cache: &mut HashMap<String, TextureId>,
    material: &str,
) -> anyhow::Result<MaterialMaps> {
    let mat = Material::parse(&index.read(&archive_key(material))?)?;
    let albedo = load_slot(
        index,
        renderer,
        dds_cache,
        mat.diffuse_map(),
        TextureId::WHITE,
    );
    let normal = load_slot(
        index,
        renderer,
        dds_cache,
        mat.normal_map(),
        TextureId::FLAT_NORMAL,
    );
    let spec = load_slot(
        index,
        renderer,
        dds_cache,
        mat.specular_map(),
        TextureId::BLACK_SPEC,
    );
    Ok(MaterialMaps {
        albedo,
        normal,
        spec,
        spec_power: mat.max_specular_power(),
    })
}

fn load_slot(
    index: &mut AssetIndex,
    renderer: &mut Renderer,
    dds_cache: &mut HashMap<String, TextureId>,
    slot: Option<&str>,
    fallback: TextureId,
) -> TextureId {
    let Some(path) = slot.filter(|s| !s.is_empty()) else {
        return fallback;
    };
    upload_dds(index, renderer, dds_cache, path).unwrap_or_else(|err| {
        log::warn!("dds {path}: {err}");
        fallback
    })
}

fn upload_world_models(
    renderer: &mut Renderer,
    models: &WorldModels,
    objects: &WorldObjects,
) -> Vec<LevelProp> {
    let mut gpu = HashMap::new();
    for bsp in &models.models {
        if bsp.is_physics() {
            continue;
        }
        let Some((verts, indices)) = bsp_mesh(bsp) else {
            continue;
        };
        match renderer.upload_mesh(&verts, &indices) {
            Ok(mesh) => {
                for name in &bsp.names {
                    gpu.insert(name.to_ascii_lowercase(), mesh);
                }
            }
            Err(err) => log::warn!("worldmodel mesh {}: {err}", bsp.names.join(",")),
        }
    }
    let mut props = Vec::new();
    for place in &objects.models {
        if place.hidden {
            continue;
        }
        let Some(&mesh) = gpu.get(&place.name.to_ascii_lowercase()) else {
            continue;
        };
        props.push(LevelProp {
            mesh,
            transform: Mat4::from_rotation_translation(place.rotation, place.pos),
            color: name_color(&place.name),
        });
    }
    props
}

fn bsp_mesh(bsp: &WorldBsp) -> Option<(Vec<Vertex>, Vec<u32>)> {
    let tris = bsp.triangles();
    if tris.is_empty() {
        return None;
    }
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for [a, b, c] in tris {
        let normal = (b - a).cross(c - a).normalize_or_zero();
        let (tangent, binormal) = tbn_from_normal(normal.to_array());
        let base = vertices.len() as u32;
        for pos in [a, b, c] {
            vertices.push(Vertex {
                pos: pos.to_array(),
                normal: normal.to_array(),
                uv: [0.0, 0.0],
                tangent,
                binormal,
            });
        }
        indices.extend_from_slice(&[base, base + 1, base + 2]);
    }
    Some((vertices, indices))
}

fn name_color(name: &str) -> [f32; 4] {
    let h = name
        .bytes()
        .fold(2166136261u32, |a, b| a.wrapping_mul(16777619) ^ u32::from(b));
    [
        0.18 + ((h & 0xFF) as f32 / 255.0) * 0.45,
        0.16 + (((h >> 8) & 0xFF) as f32 / 255.0) * 0.40,
        0.14 + (((h >> 16) & 0xFF) as f32 / 255.0) * 0.38,
        1.0,
    ]
}

fn upload_dds(
    index: &mut AssetIndex,
    renderer: &mut Renderer,
    dds_cache: &mut HashMap<String, TextureId>,
    path: &str,
) -> anyhow::Result<TextureId> {
    let mut key = archive_key(path);
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
        DdsFormat::Bgra8 => TextureFormat::Bgra8,
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
