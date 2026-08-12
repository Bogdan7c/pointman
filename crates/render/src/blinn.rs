//! Jupiter EX Blinn-Phong: spec.rgb * pow(N·H, spec.a * fMaxSpecularPower).
//! Не PBR. Нужен, чтобы тесты ловили влияние карт, а не только парсинг файлов.

use glam::Vec3;

/// D3D9 A8R8G8B8 в памяти лежит как BGRA. В шейдере .xyz — это R,G,B = tangent XYZ.
pub fn bgra_to_tangent(bgra: [u8; 4]) -> Vec3 {
    let r = f32::from(bgra[2]);
    let g = f32::from(bgra[1]);
    let b = f32::from(bgra[0]);
    Vec3::new(r, g, b) / 255.0 * 2.0 - 1.0
}

/// Столбцы TBN * n_ts, как `mat3(T, B, N) * n_ts` в gbuffer.frag.
pub fn tangent_to_world(tangent: Vec3, binormal: Vec3, normal: Vec3, n_ts: Vec3) -> Vec3 {
    (tangent * n_ts.x + binormal * n_ts.y + normal * n_ts.z).normalize_or_zero()
}

pub fn blinn_specular(n: Vec3, light: Vec3, view: Vec3, spec_rgb: Vec3, gloss: f32, max_power: f32) -> Vec3 {
    let h = (light + view).normalize_or_zero();
    let ndoth = n.dot(h).max(0.0);
    let power = (gloss * max_power).max(1.0);
    spec_rgb * ndoth.powf(power)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wood_floor_bgra_is_almost_flat_up() {
        // Intro Wood_Floor06_N pix0 BGRA = [254, 127, 127, 30]
        let n = bgra_to_tangent([254, 127, 127, 30]);
        assert!(n.z > 0.95, "expected +Z tangent normal, got {n}");
        assert!(n.length() > 0.9);
    }

    #[test]
    fn flat_nmap_keeps_vertex_normal() {
        let n = Vec3::Y;
        let t = Vec3::X;
        let b = Vec3::NEG_Z;
        let n_ts = bgra_to_tangent([255, 128, 128, 255]); // B=255,G=128,R=128 → (0,0,1)
        let world = tangent_to_world(t, b, n, n_ts);
        assert!(
            world.dot(Vec3::Y) > 0.95,
            "flat nmap must leave world +Y, got {world}"
        );
    }

    #[test]
    fn tilted_nmap_changes_lighting_normal() {
        let n = Vec3::Y;
        let t = Vec3::X;
        let b = Vec3::NEG_Z;
        let n_ts = bgra_to_tangent([250, 152, 103, 220]); // Asphalt01_N pix0
        let world = tangent_to_world(t, b, n, n_ts);
        assert!(
            world.dot(Vec3::Y) < 0.99,
            "tilted nmap must move the world normal, got {world}"
        );
        assert!(world.y > 0.7, "still mostly up, got {world}");
    }

    #[test]
    fn black_spec_kills_highlight() {
        let n = Vec3::Y;
        let l = Vec3::new(0.0, 1.0, 1.0).normalize();
        let v = l;
        let lit = blinn_specular(n, l, v, Vec3::ZERO, 1.0, 64.0);
        assert!(lit.length() < 1e-6, "black spec.rgb must be zero, got {lit}");
    }

    #[test]
    fn high_gloss_tightens_the_highlight_lobe() {
        let n = Vec3::Y;
        let l = Vec3::new(0.6, 1.0, 0.0).normalize();
        let v = Vec3::new(0.6, 1.0, 0.0).normalize();
        let tight = blinn_specular(n, l, v, Vec3::ONE, 1.0, 64.0);
        let broad = blinn_specular(n, l, v, Vec3::ONE, 0.05, 64.0);
        assert!(
            tight.x < broad.x * 0.5,
            "high gloss must fall off faster off-peak ({tight} vs {broad})"
        );
        let peak = blinn_specular(n, Vec3::Y, Vec3::Y, Vec3::ONE, 1.0, 64.0);
        assert!(peak.x > 0.99);
    }
}
