use anyhow::{bail, Context};
use clap::{Parser, Subcommand};
use pointman_assets::{kind_from_path, ArchHeader, GameArchive, WorldHeader, WorldModels, WorldRender};
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
            let bytes = if path.is_dir() {
                let mount = pointman_game::GameMount::discover(&path);
                mount.read_file(&name).with_context(|| name.clone())?
            } else {
                std::fs::read(&path).with_context(|| path.display().to_string())?
            };
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
                            "PhysicsBSP {}  points {}  polys {}  tris {}",
                            bsp.names.join(","),
                            bsp.points.len(),
                            bsp.polygons.len(),
                            models.triangles().len()
                        );
                    } else {
                        println!("PhysicsBSP missing");
                    }
                }
                Err(err) => println!("world models: {err}"),
            }
        }
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
