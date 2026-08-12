//! Resource extensions from the public F.E.A.R. SDK 1.08 (`resourceextensions.h`).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    WorldPacked,
    WorldCompressed,
    WorldAscii,
    MeshPacked,
    ObjectsPacked,
    ModelPacked,
    ModelCompressed,
    ModelAscii,
    EffectPacked,
    TextureDds,
    TexturePacked,
    MaterialPacked,
    Material,
    GameDatabasePacked,
    StringDatabasePacked,
    AnimTreePacked,
    SoundWav,
    ArchiveArch00,
    ArchiveRez,
    Other,
}

pub fn kind_from_path(path: &str) -> ResourceKind {
    let ext = path
        .rsplit(['.', '/', '\\'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "world00p" => ResourceKind::WorldPacked,
        "world00c" => ResourceKind::WorldCompressed,
        "world00a" => ResourceKind::WorldAscii,
        "mesh00p" => ResourceKind::MeshPacked,
        "objects00p" => ResourceKind::ObjectsPacked,
        "model00p" => ResourceKind::ModelPacked,
        "model00c" => ResourceKind::ModelCompressed,
        "model00a" => ResourceKind::ModelAscii,
        "fx00p" => ResourceKind::EffectPacked,
        "dds" => ResourceKind::TextureDds,
        "texture00p" => ResourceKind::TexturePacked,
        "mat00p" => ResourceKind::MaterialPacked,
        "mat00" => ResourceKind::Material,
        "gamdb00p" => ResourceKind::GameDatabasePacked,
        "strdb00p" => ResourceKind::StringDatabasePacked,
        "anmtree00p" => ResourceKind::AnimTreePacked,
        "wav" => ResourceKind::SoundWav,
        "arch00" | "arch01" => ResourceKind::ArchiveArch00,
        "rez" => ResourceKind::ArchiveRez,
        _ => ResourceKind::Other,
    }
}
