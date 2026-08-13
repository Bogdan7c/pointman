//! Что из WorldModel попадает в кадр, а что остаётся только коллизией.
//!
//! Запечённый `WorldRender` уже содержит стены. BSP тех же кистей — для ног, не вторая
//! серая стена поверх кирпича. В кадре остаются машины, двери и пропы, которых нет в меше.

use crate::world00p::WorldRender;
use crate::world_models::WorldBsp;
use crate::world_objects::{WorldModelPlacement, WorldSky};
use glam::{Mat4, Vec3};
use std::collections::HashSet;

/// Ячейка сетки в сантиметрах LithTech. 16 см ≈ шов кирпича: ловим z-fight, не соседний проп.
const BAKED_CELL_CM: i32 = 16;

/// Доля вершин BSP, попавших в запечённый меш, после которой это дубликат стены, не проп.
const BAKED_OVERLAP_MIN: f32 = 0.6;

/// Пространственный индекс вершин запечённого мира. Строим один раз на загрузке уровня.
#[derive(Debug, Clone)]
pub struct BakedOverlapIndex {
    cells: HashSet<(i32, i32, i32)>,
}

impl BakedOverlapIndex {
    /// Индекс из уже разобранного render-меша Intro (стены, пол, потолок).
    pub fn from_render(world: &WorldRender) -> Self {
        Self::from_points(
            world
                .surfaces
                .iter()
                .flat_map(|surf| surf.vertices.iter().map(|v| Vec3::from_array(v.position))),
        )
    }

    /// Тот же индекс из произвольных точек — для тестов без целого World00p.
    pub fn from_points(points: impl IntoIterator<Item = Vec3>) -> Self {
        let mut cells = HashSet::new();
        for point in points {
            cells.insert(grid_cell(point));
        }
        Self { cells }
    }

    /// BSP уже сидит в запечённом меше: рисуя его болванкой, получим рваные пятна на текстуре.
    pub fn duplicates_baked(&self, bsp: &WorldBsp, transform: Mat4) -> bool {
        overlap_ratio(&self.cells, bsp, transform) >= BAKED_OVERLAP_MIN
    }
}

/// Рисуем ли этот экземпляр в кадре. Клип (PhysicsBSP) сюда не входит — он для ног.
pub fn world_model_in_frame(
    place: &WorldModelPlacement,
    bsp: Option<&WorldBsp>,
    sky: Option<&WorldSky>,
    baked: &BakedOverlapIndex,
) -> bool {
    if place.hidden || !place.visible {
        return false;
    }
    // Стекло и декали с альфой: непрозрачная болванка = серая стена. Альфа — пункт 1.5.
    if place.translucent {
        return false;
    }
    if looks_like_shadow_catcher(&place.name) {
        return false;
    }
    if sky.is_some_and(|sky| sky.contains_model(&place.name)) {
        return false;
    }
    let Some(bsp) = bsp else {
        return false;
    };
    if bsp.is_physics() {
        return false;
    }
    let transform = Mat4::from_rotation_translation(place.rotation, place.pos);
    !baked.duplicates_baked(bsp, transform)
}

/// Плоскости теней машин (`Car_Shadows`) — не проп. В оригинале это пятно, не цветной ящик 36 м.
fn looks_like_shadow_catcher(name: &str) -> bool {
    name.to_ascii_lowercase().contains("shadow")
}

fn overlap_ratio(cells: &HashSet<(i32, i32, i32)>, bsp: &WorldBsp, transform: Mat4) -> f32 {
    if bsp.points.is_empty() {
        return 0.0;
    }
    let mut hits = 0usize;
    for &local in &bsp.points {
        let world = transform.transform_point3(local);
        if cells.contains(&grid_cell(world)) {
            hits += 1;
        }
    }
    hits as f32 / bsp.points.len() as f32
}

fn grid_cell(point: Vec3) -> (i32, i32, i32) {
    let size = BAKED_CELL_CM as f32;
    (
        (point.x / size).round() as i32,
        (point.y / size).round() as i32,
        (point.z / size).round() as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world00p::WorldRender;
    use crate::world_models::WorldModels;
    use crate::world_objects::WorldObjects;
    use glam::Quat;

    fn placement(name: &str, pos: Vec3) -> WorldModelPlacement {
        WorldModelPlacement {
            name: name.to_string(),
            pos,
            rotation: Quat::IDENTITY,
            hidden: false,
            visible: true,
            translucent: false,
        }
    }

    fn bsp_with_points(name: &str, points: Vec<Vec3>) -> WorldBsp {
        WorldBsp {
            names: vec![name.to_string()],
            center: Vec3::ZERO,
            half_extents: Vec3::ONE,
            points,
            polygons: Vec::new(),
        }
    }

    #[test]
    fn crate_away_from_wall_stays_in_frame() {
        let baked = BakedOverlapIndex::from_points([Vec3::ZERO, Vec3::X * 32.0]);
        let prop = placement("Crate00", Vec3::new(400.0, 0.0, 0.0));
        let bsp = bsp_with_points("Crate00", vec![Vec3::ZERO, Vec3::X * 10.0, Vec3::Y * 10.0]);
        assert!(world_model_in_frame(&prop, Some(&bsp), None, &baked));
    }

    #[test]
    fn wall_bsp_on_baked_vertices_stays_out_of_frame() {
        let wall_pts = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(32.0, 0.0, 0.0),
            Vec3::new(0.0, 64.0, 0.0),
            Vec3::new(32.0, 64.0, 0.0),
        ];
        let baked = BakedOverlapIndex::from_points(wall_pts);
        let wall = placement("WorldModel21", Vec3::ZERO);
        let bsp = bsp_with_points("WorldModel21", wall_pts.to_vec());
        assert!(!world_model_in_frame(&wall, Some(&bsp), None, &baked));
    }

    #[test]
    fn invisible_placement_is_not_drawn() {
        let baked = BakedOverlapIndex::from_points(Vec::<Vec3>::new());
        let mut prop = placement("Crate00", Vec3::ZERO);
        prop.visible = false;
        let bsp = bsp_with_points("Crate00", vec![Vec3::ZERO]);
        assert!(!world_model_in_frame(&prop, Some(&bsp), None, &baked));
    }

    #[test]
    fn hidden_placement_is_not_drawn() {
        let baked = BakedOverlapIndex::from_points(Vec::<Vec3>::new());
        let mut prop = placement("HiddenDoor", Vec3::ZERO);
        prop.hidden = true;
        let bsp = bsp_with_points("HiddenDoor", vec![Vec3::ZERO]);
        assert!(!world_model_in_frame(&prop, Some(&bsp), None, &baked));
    }

    #[test]
    fn translucent_stub_is_not_drawn_opaque() {
        let baked = BakedOverlapIndex::from_points(Vec::<Vec3>::new());
        let mut glass = placement("Door.glass", Vec3::ZERO);
        glass.translucent = true;
        let bsp = bsp_with_points("Door.glass", vec![Vec3::ZERO]);
        assert!(!world_model_in_frame(&glass, Some(&bsp), None, &baked));
    }

    #[test]
    fn car_shadow_catcher_is_not_a_prop() {
        let baked = BakedOverlapIndex::from_points(Vec::<Vec3>::new());
        let shadow = placement("Car_Shadows", Vec3::new(-50.0, -1500.0, -4000.0));
        let bsp = bsp_with_points("Car_Shadows", vec![Vec3::ZERO, Vec3::X * 100.0]);
        assert!(!world_model_in_frame(&shadow, Some(&bsp), None, &baked));
    }

    #[test]
    fn physics_bsp_is_never_drawn() {
        let baked = BakedOverlapIndex::from_points(Vec::<Vec3>::new());
        let prop = placement("PhysicsBSP", Vec3::ZERO);
        let bsp = bsp_with_points("PhysicsBSP", vec![Vec3::ZERO, Vec3::X, Vec3::Y]);
        assert!(bsp.is_physics());
        assert!(!world_model_in_frame(&prop, Some(&bsp), None, &baked));
    }

    #[test]
    fn sky_object_is_not_drawn_in_world_pass() {
        let baked = BakedOverlapIndex::from_points(Vec::<Vec3>::new());
        let cube = placement("Sky_Japan00.SkyCube", Vec3::ZERO);
        let bsp = bsp_with_points("Sky_Japan00.SkyCube", vec![Vec3::ZERO]);
        let sky = WorldSky {
            camera_pos: Vec3::ZERO,
            object_names: vec!["Sky_Japan00.SkyCube".into()],
        };
        assert!(!world_model_in_frame(&cube, Some(&bsp), Some(&sky), &baked));
    }

    #[test]
    fn missing_bsp_is_not_drawn() {
        let baked = BakedOverlapIndex::from_points(Vec::<Vec3>::new());
        let prop = placement("Orphan", Vec3::ZERO);
        assert!(!world_model_in_frame(&prop, None, None, &baked));
    }

    #[test]
    fn intro_keeps_car_drops_baked_wall_if_extracted() {
        let path = std::env::var("POINTMAN_INTRO_WORLD00P")
            .unwrap_or_else(|_| "/tmp/pointman-sky/Worlds/Release/Intro.World00p".into());
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        let world = WorldRender::parse(&bytes).unwrap();
        let models = WorldModels::parse(&bytes).unwrap();
        let objects = WorldObjects::parse(&bytes).unwrap();
        let baked = BakedOverlapIndex::from_render(&world);
        let in_frame: Vec<_> = objects
            .models
            .iter()
            .filter(|place| {
                world_model_in_frame(
                    place,
                    models.mesh_named(&place.name),
                    objects.sky.as_ref(),
                    &baked,
                )
            })
            .collect();
        assert!(
            in_frame.len() < objects.models.len(),
            "фильтр 1.2 должен срезать дубликаты стен, было {} стало {}",
            objects.models.len(),
            in_frame.len()
        );
        assert!(
            in_frame.iter().any(|p| p.name == "Car_WM"),
            "машина двора должна остаться в кадре"
        );
        assert!(
            in_frame
                .iter()
                .any(|p| p.name == "trash_can02.WorldModel01"),
            "урна — проп, не стена"
        );
        assert!(
            !in_frame.iter().any(|p| p.name == "WorldModel21"),
            "архитектурный BSP на запечённой стене не рисуем"
        );
        assert!(
            !in_frame
                .iter()
                .any(|p| p.name.to_ascii_lowercase().contains("shadow")),
            "пятно тени машины — не цветная болванка"
        );
    }
}
