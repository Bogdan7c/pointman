//! Загрузка неба Intro: cubemap из skybox-материала, без плоской DDS на стене.

use anyhow::Context;
use pointman_assets::{archive_key, material_key, DdsCubemap, DdsFormat, Material};
use pointman_game::AssetIndex;
use pointman_render::{CubemapId, CubemapUpload, Renderer, TextureFormat};

pub fn is_skybox_material(index: &mut AssetIndex, material: &str) -> bool {
    let Ok(bytes) = index.read(&material_key(material)) else {
        return looks_like_sky_surface(material);
    };
    Material::parse(&bytes)
        .ok()
        .is_some_and(|mat| mat.is_skybox())
        || looks_like_sky_surface(material)
}

/// Короткое имя вроде `Sky_Day.Mat00` без папки — всё равно небо, не стена.
pub fn looks_like_sky_surface(material: &str) -> bool {
    let name = material.to_ascii_lowercase();
    name.contains("sky_day")
        || name.contains("sky_night")
        || name.contains("skybox")
        || name.contains("cloudplane")
}

pub const INTRO_SKY_MAT: &str = r"Prefabs\Systemic\Sky\Sky_Day.Mat00";

pub fn load_sky_cubemap(
    index: &mut AssetIndex,
    renderer: &mut Renderer,
    material: &str,
) -> anyhow::Result<CubemapId> {
    let mat = Material::parse(&index.read(&material_key(material))?)?;
    let path = mat
        .diffuse_map()
        .filter(|p| !p.is_empty())
        .context("skybox without tDiffuseMap")?;
    let cube = DdsCubemap::parse(&index.read(&archive_key(path))?)
        .with_context(|| format!("cubemap {path}"))?;
    let format = match cube.format {
        DdsFormat::Bc1 => TextureFormat::Bc1,
        DdsFormat::Bc2 => TextureFormat::Bc2,
        DdsFormat::Bc3 => TextureFormat::Bc3,
        DdsFormat::Bgra8 => TextureFormat::Bgra8,
    };
    let id = renderer
        .upload_cubemap(CubemapUpload {
            width: cube.width,
            height: cube.height,
            mip_count: cube.mip_count,
            format,
            bytes: &cube.bytes,
        })
        .with_context(|| format!("upload cubemap {path}"))?;
    log::info!(
        "sky cubemap {path}  {}x{}  mips {}  faces 6",
        cube.width,
        cube.height,
        cube.mip_count
    );
    Ok(id)
}
