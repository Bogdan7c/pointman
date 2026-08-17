use anyhow::{bail, Context};
use clap::{Parser, Subcommand};
use pointman_assets::{
    kind_from_path, ArchHeader, GameArchive, RawWorldObjects, WorldHeader, WorldModels,
    WorldObjects, WorldRender,
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "reztool", about = "List and extract F.E.A.R. Arch00 / REZ archives")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Probe { archive: PathBuf },
    Catalog { root: PathBuf },
    List { archive: PathBuf },
    /// Parse a packed World00p (file on disk, or game root + --name).
    World {
        path: PathBuf,
        #[arg(long, default_value = "Worlds/Release/Intro.World00p")]
        name: String,
    },
    Extract {
        archive: PathBuf,
        out: PathBuf,
        #[arg(long)]
        filter: Option<String>,
    },
    /// Сырой дамп объектов World00p: типы, лампы, WorldProperties. Не рисует кадр.
    DumpDraw {
        path: PathBuf,
        #[arg(long, default_value = "Worlds/Release/Intro.World00p")]
        name: String,
    },
    /// Fog-параметры WorldProperties всех миров (FogEnable/Color/NearZ/FarZ, SkyFog).
    Fog {
        path: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Catalog { root } => {
            let mount = pointman_game::GameMount::discover(&root);
            let cat = mount.catalog();
            println!(
                "worlds {}  models {}  dds {}  materials {}  other {}",
                cat.worlds.len(),
                cat.models.len(),
                cat.textures.len(),
                cat.materials.len(),
                cat.other
            );
            for (name, src) in &cat.worlds {
                println!(
                    "W  {name}  ({})",
                    src.file_name().unwrap_or_default().to_string_lossy()
                );
            }
        }
        Cmd::Probe { archive } => {
            let meta = std::fs::metadata(&archive)?;
            let h = ArchHeader::probe(&archive)?;
            println!(
                "{}  version {}  files {}  folders {}  names {} B  size {:.1} MiB",
                archive.display(),
                h.version,
                h.file_count,
                h.folder_count,
                h.name_table_size,
                meta.len() as f64 / (1024.0 * 1024.0)
            );
        }
        Cmd::List { archive } => {
            let pack = GameArchive::open(&archive)?;
            for name in pack.list() {
                println!("{name}");
            }
        }
        Cmd::World { path, name } => {
            let bytes = read_world_bytes(&path, &name)?;
            let world = WorldRender::parse(&bytes)?;
            let (verts, indices, draws) = world.flatten();
            println!(
                "v{}  surfaces {} (drawn {})  verts {}  indices {}  bounds {:?} → {:?}  offset {:?}",
                world.header.version,
                world.surfaces.len(),
                draws.len(),
                verts.len(),
                indices.len(),
                world.header.min,
                world.header.max,
                world.header.offset
            );
            for (i, surf) in world.surfaces.iter().take(16).enumerate() {
                println!(
                    "  [{i}] tris {}  {}",
                    surf.indices.len() / 3,
                    surf.material
                );
            }
            if world.surfaces.len() > 16 {
                println!("  … {} more", world.surfaces.len() - 16);
            }
            match WorldModels::parse(&bytes) {
                Ok(models) => {
                    if let Some(bsp) = models.physics() {
                        println!(
                            "PhysicsBSP {}  points {}  polys {}  clip tris {}  blockers {}  bsp meshes {}",
                            bsp.names.join(","),
                            bsp.points.len(),
                            bsp.polygons.len(),
                            models.triangles().len(),
                            models.blockers.len(),
                            models.models.len()
                        );
                    } else {
                        println!("PhysicsBSP missing");
                    }
                }
                Err(err) => println!("world models: {err}"),
            }
            match WorldObjects::parse(&bytes) {
                Ok(objects) => {
                    if let Some(start) = objects.spawn() {
                        println!(
                            "GameStartPoint {}  {:?}  yaw {:.1}°",
                            start.name,
                            start.pos,
                            start.yaw.to_degrees()
                        );
                    } else {
                        println!("GameStartPoint missing");
                    }
                    println!(
                        "point/fill lights {}  ambient {:?}  worldmodels {}",
                        objects.lights.len(),
                        objects.ambient,
                        objects.models.len()
                    );
                }
                Err(err) => println!("world objects: {err}"),
            }
        }
        Cmd::DumpDraw { path, name } => dump_draw(&path, &name)?,
        Cmd::Fog { path } => fog_scan(&path)?,
        Cmd::Extract {
            archive,
            out,
            filter,
        } => {
            let mut pack = GameArchive::open(&archive)?;
            let names = pack.list();
            std::fs::create_dir_all(&out)?;
            for name in names {
                if let Some(f) = &filter {
                    if !name.to_ascii_lowercase().contains(&f.to_ascii_lowercase()) {
                        continue;
                    }
                }
                let bytes = pack.read(&name).with_context(|| name.clone())?;
                if kind_from_path(&name) == pointman_assets::ResourceKind::WorldPacked {
                    match WorldHeader::parse(&bytes) {
                        Ok(h) => println!(
                            "{name}: World00p v{} bounds {:?} -> {:?}",
                            h.version, h.min, h.max
                        ),
                        Err(err) => println!("{name}: world header {err}"),
                    }
                }
                let dest = out.join(name.replace('\\', "/"));
                if dest.components().any(|c| c.as_os_str() == "..") {
                    bail!("refusing path {name}");
                }
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&dest, bytes)?;
            }
        }
    }
    Ok(())
}

fn fog_scan(path: &std::path::Path) -> anyhow::Result<()> {
    let mount = pointman_game::GameMount::discover(path);
    let cat = mount.catalog();
    let mut worlds: Vec<&str> = cat
        .worlds
        .iter()
        .filter(|(n, _)| n.starts_with("Worlds/Release/") && n.ends_with(".World00p"))
        .map(|(s, _)| s.as_str())
        .collect();
    worlds.sort();
    for name in worlds {
        let Ok(bytes) = mount.read_file(name) else {
            continue;
        };
        let Ok(raw) = RawWorldObjects::parse(&bytes) else {
            println!("{name}: parse error");
            continue;
        };
        let Some(wp) = raw.of_type("WorldProperties").next() else {
            println!("{name}: no WorldProperties");
            continue;
        };
        let f = |k: &str| wp.prop(k).unwrap_or("-").to_string();
        println!(
            "{name}: FogEnable={} FogColor={} FogNearZ={} FogFarZ={} SkyFogEnable={} SkyFogNearZ={} SkyFogFarZ={}",
            f("FogEnable"),
            f("FogColor"),
            f("FogNearZ"),
            f("FogFarZ"),
            f("SkyFogEnable"),
            f("SkyFogNearZ"),
            f("SkyFogFarZ")
        );
    }
    Ok(())
}

fn read_world_bytes(path: &std::path::Path, name: &str) -> anyhow::Result<Vec<u8>> {
    if path.is_dir() {
        let mount = pointman_game::GameMount::discover(path);
        mount.read_file(name).with_context(|| name.to_string())
    } else {
        std::fs::read(path).with_context(|| path.display().to_string())
    }
}

fn dump_draw(path: &std::path::Path, name: &str) -> anyhow::Result<()> {
    let bytes = read_world_bytes(path, name)?;
    let raw = RawWorldObjects::parse(&bytes)?;
    println!("objects {}  file {name}", raw.objects.len());
    println!("types:");
    for (ty, count) in raw.type_histogram() {
        println!("  {count:>5}  {ty}");
    }
    println!("WorldProperties:");
    for object in raw.of_type("WorldProperties") {
        for prop in &object.properties {
            println!("  {} = {}", prop.name, prop.value);
        }
    }
    println!("lights:");
    for object in &raw.objects {
        if !object.type_name.starts_with("Light") {
            continue;
        }
        print!("  [{}]", object.type_name);
        for prop in &object.properties {
            print!("  {}={}", prop.name, prop.value);
        }
        println!();
    }
    println!("RenderTargetGroup:");
    for object in raw.of_type("RenderTargetGroup") {
        print!("  [{}]", object.type_name);
        for prop in &object.properties {
            print!("  {}={}", prop.name, prop.value);
        }
        println!();
    }
    println!("RenderTarget:");
    for object in raw.of_type("RenderTarget") {
        print!("  [{}]", object.type_name);
        for prop in &object.properties {
            print!("  {}={}", prop.name, prop.value);
        }
        println!();
    }
    Ok(())
}
