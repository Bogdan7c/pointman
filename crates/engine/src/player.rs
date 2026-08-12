//! Движение игрока по fallback CMoveMgr из официального Fear-SDK-1.08.
//! Капсула и пол живут в `clip`. Здесь — скорости, прыжок, lean и масштаб см → мир.

use glam::{Vec2, Vec3};
use pointman_render::Camera;

/// Бег (AlwaysRun / полный стик на 360), см/с.
pub const RUN_SPEED_CM: f32 = 400.0;
/// Ходьба PC (клавиша Walk). На 360 не используем — стик всегда бег.
#[allow(dead_code)]
pub const WALK_SPEED_CM: f32 = 280.0;
/// Присед / crawl, см/с.
pub const CROUCH_SPEED_CM: f32 = 200.0;
/// Гравитация игрока, не мира: SDK `DEFAULT_PLAYER_GRAVITY`.
pub const PLAYER_GRAVITY_CM: f32 = 2000.0;
/// Стартовая вертикальная скорость: JumpSpeed 550 × JumpVelMul 0.81.
pub const JUMP_VEL_CM: f32 = 445.5;
/// Радиус капсулы = half-X/Z AABB FEAR.
pub const CAPSULE_RADIUS_CM: f32 = 40.0;
/// Глаза стоя: ~0.75×halfY от центра + пол. Наши 160 см уже стоят в спавне.
pub const STAND_EYE_CM: f32 = 160.0;
/// Глаза в приседе (half-Y 61 см).
pub const CROUCH_EYE_CM: f32 = 105.0;
/// Максимальный roll камеры. ModelsDB в SDK нет — прототип.
pub const LEAN_MAX_ROLL_RAD: f32 = 20.0 * std::f32::consts::PI / 180.0;
/// Смещение глаз в сторону: sin(20°) × 70 см рычага торс→глаз.
pub const LEAN_MAX_OFFSET_CM: f32 = 24.0;
/// Время выхода в укрытие.
pub const LEAN_OUT_S: f32 = 0.20;
/// Время возврата в центр.
pub const LEAN_IN_S: f32 = 0.15;
/// Дальность узкого фонарика Player_Narrow (цифр в SDK нет).
pub const FLASH_RADIUS_CM: f32 = 1400.0;
/// Яркость фонарика, как у текущей point-заглушки.
pub const FLASH_INTENSITY: f32 = 12.0;
/// cos(22.5°) — половина внешнего угла ~45° FOV.
pub const FLASH_OUTER_COS: f32 = 0.9238795;

/// Перевод LithTech-сантиметров в мировые единицы симуляции.
/// `unit = 100` на уровне (1 мир = 1 см), `unit = 1` в метровом коридоре.
pub fn scale_cm(unit: f32, cm: f32) -> f32 {
    cm * unit / 100.0
}

/// Высота глаз над ступнями в см мира.
pub fn eye_height(crouch: bool, unit: f32) -> f32 {
    let cm = if crouch {
        CROUCH_EYE_CM
    } else {
        STAND_EYE_CM
    };
    scale_cm(unit, cm)
}

/// Насколько поднять глаз при вставании из приседа.
pub fn stand_raise(unit: f32) -> f32 {
    scale_cm(unit, STAND_EYE_CM - CROUCH_EYE_CM)
}

/// Горизонтальная wish-скорость: на 360 полный стик = бег, без порога 0.9.
pub fn wish_velocity(
    forward: Vec3,
    right: Vec3,
    move_axis: Vec2,
    crouch: bool,
    unit: f32,
) -> Vec3 {
    let mut wish = forward * move_axis.y + right * move_axis.x;
    wish.y = 0.0;
    let mag = move_axis.length().min(1.0);
    if wish.length_squared() <= 1e-8 || mag <= 0.0 {
        return Vec3::ZERO;
    }
    let speed = if crouch {
        CROUCH_SPEED_CM
    } else {
        RUN_SPEED_CM
    };
    wish.normalize() * scale_cm(unit, speed) * mag
}

/// Прыжок только с пола, не из приседа, без coyote. `None` — не прыгаем.
pub fn jump_impulse(grounded: bool, crouch: bool, jump_pressed: bool, unit: f32) -> Option<f32> {
    if jump_pressed && grounded && !crouch {
        Some(scale_cm(unit, JUMP_VEL_CM))
    } else {
        None
    }
}

/// Lean — hold, только стоя на месте. Капсула не едет, двигается камера в draw.
pub fn step_lean(current: f32, hold: f32, moving: bool, grounded: bool, dt: f32) -> f32 {
    let target = if !grounded || moving {
        0.0
    } else {
        hold.clamp(-1.0, 1.0)
    };
    let seconds = if target.abs() > current.abs() {
        LEAN_OUT_S
    } else {
        LEAN_IN_S
    };
    let max_step = dt / seconds;
    let delta = target - current;
    current + delta.clamp(-max_step, max_step)
}

/// Смещение и roll только для камеры кадра. `eye` в Simulation не трогаем.
pub fn apply_lean(eye_camera: &Camera, lean: f32, unit: f32) -> Camera {
    let mut camera = eye_camera.clone();
    camera.position += eye_camera.right() * (lean * scale_cm(unit, LEAN_MAX_OFFSET_CM));
    camera.roll = lean * LEAN_MAX_ROLL_RAD;
    camera
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    #[test]
    fn walk_speed_exists_for_pc_later() {
        assert_eq!(WALK_SPEED_CM, 280.0);
    }

    #[test]
    fn full_stick_run_is_400_cm_per_s() {
        let wish = wish_velocity(Vec3::Z, Vec3::X, Vec2::new(0.0, 1.0), false, 100.0);
        assert!((wish.z - 400.0).abs() < 0.1, "got {wish:?}");
        assert!(wish.x.abs() < 0.1 && wish.y.abs() < 0.1);
    }

    #[test]
    fn crouch_run_is_200_cm_per_s() {
        let wish = wish_velocity(Vec3::Z, Vec3::X, Vec2::Y, true, 100.0);
        assert!((wish.z - 200.0).abs() < 0.1, "got {wish:?}");
    }

    #[test]
    fn half_stick_scales_run() {
        let wish = wish_velocity(Vec3::Z, Vec3::X, Vec2::new(0.0, 0.5), false, 100.0);
        assert!((wish.z - 200.0).abs() < 0.1, "got {wish:?}");
    }

    #[test]
    fn jump_only_on_ground_standing() {
        assert!(jump_impulse(true, false, true, 100.0).is_some());
        assert!(jump_impulse(false, false, true, 100.0).is_none());
        assert!(jump_impulse(true, true, true, 100.0).is_none());
        assert!(jump_impulse(true, false, false, 100.0).is_none());
        let vel = jump_impulse(true, false, true, 100.0).unwrap();
        assert!((vel - 445.5).abs() < 0.01);
    }

    #[test]
    fn lean_holds_then_returns() {
        let mut lean = 0.0;
        for _ in 0..20 {
            lean = step_lean(lean, 1.0, false, true, 0.02);
        }
        assert!((lean - 1.0).abs() < 0.05, "did not reach lean, {lean}");
        lean = step_lean(lean, 1.0, true, true, 0.2);
        assert!(lean.abs() < 0.05, "moving should cancel lean, {lean}");
    }
}
