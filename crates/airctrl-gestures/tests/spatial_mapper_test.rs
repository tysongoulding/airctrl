//! Spatial Radial Dial Coordinate Mapper Test Suite.

use airctrl_core::types::gesture::Landmark3D;
use airctrl_gestures::mapper::{ScreenBounds, SpatialCoordinateMapper};
use std::f32::consts::PI;

#[test]
fn test_center_deadband_and_boundary_partitions() {
    let mut mapper = SpatialCoordinateMapper::new(6, (0.5, 0.5), 0.10);

    // 1. Exact Center (0.5, 0.5)
    let state_center = mapper.map_coords(0.5, 0.5);
    assert!(state_center.in_deadzone);
    assert_eq!(state_center.active_sector, None);
    assert_eq!(state_center.normalized_radius, 0.0);
    assert_eq!(state_center.angle_rad, 0.0);

    // 2. Deep inside deadzone (0.50, 0.51) -> normalized radius 0.02 < 0.10
    let state_dead = mapper.map_coords(0.50, 0.51);
    assert!(state_dead.in_deadzone);
    assert_eq!(state_dead.active_sector, None);

    // 3. Just below deadzone threshold: dy = 0.049 -> radius 0.049 / 0.5 = 0.098 < 0.10
    let state_under = mapper.map_coords(0.50, 0.549);
    assert!(state_under.in_deadzone);
    assert_eq!(state_under.active_sector, None);

    // 4. Just above deadzone threshold: dy = 0.052 -> radius 0.052 / 0.5 = 0.104 >= 0.10
    let state_active = mapper.map_coords(0.50, 0.552);
    assert!(!state_active.in_deadzone);
    assert!(state_active.active_sector.is_some());
}

#[test]
fn test_six_sector_polar_conversion() {
    let mut mapper = SpatialCoordinateMapper::new(6, (0.5, 0.5), 0.10);

    // Sector 0: Directly Right (0.85, 0.50) -> theta = 0.0
    let s0 = mapper.map_coords(0.85, 0.50);
    assert!(!s0.in_deadzone);
    assert_eq!(s0.active_sector, Some(0));
    assert_eq!(s0.angle_rad, 0.0);

    // Sector 1: Directly Down (0.50, 0.85) -> theta = pi/2 = 1.5708
    let s1 = mapper.map_coords(0.50, 0.85);
    assert!(!s1.in_deadzone);
    assert_eq!(s1.active_sector, Some(1));

    // Sector 2: Down-Left (0.25, 0.80) -> theta ~ 2.3 rad
    let s2 = mapper.map_coords(0.25, 0.80);
    assert!(!s2.in_deadzone);
    assert_eq!(s2.active_sector, Some(2));

    // Sector 3: Directly Left (0.15, 0.50) -> theta = pi = 3.1416
    let s3 = mapper.map_coords(0.15, 0.50);
    assert!(!s3.in_deadzone);
    assert_eq!(s3.active_sector, Some(3));

    // Sector 4: Directly Up (0.50, 0.15) -> theta = 3pi/2 = 4.7124
    let s4 = mapper.map_coords(0.50, 0.15);
    assert!(!s4.in_deadzone);
    assert_eq!(s4.active_sector, Some(4));

    // Sector 5: Up-Right (0.80, 0.25) -> theta ~ 5.6 rad
    let s5 = mapper.map_coords(0.80, 0.25);
    assert!(!s5.in_deadzone);
    assert_eq!(s5.active_sector, Some(5));

    // Verify all 6 sectors covered systematically: (i * 2*pi / 6) + 0.1
    for i in 0..6 {
        let angle = (i as f32 * 2.0 * PI / 6.0) + 0.1;
        let x = 0.5 + 0.35 * angle.cos();
        let y = 0.5 + 0.35 * angle.sin();
        let res = mapper.map_coords(x, y);
        assert_eq!(res.active_sector, Some(i));
    }
}

#[test]
fn test_radius_clamping_at_outer_perimeter() {
    let mut mapper = SpatialCoordinateMapper::new(6, (0.5, 0.5), 0.10);

    // (1.0, 1.0) diagonal distance is sqrt(0.5^2 + 0.5^2) / 0.5 = 1.414 -> clamped to 1.0
    let s_edge = mapper.map_coords(1.0, 1.0);
    assert_eq!(s_edge.normalized_radius, 1.0);

    // Far out coordinate
    let s_far = mapper.map_coords(2.5, 3.0);
    assert_eq!(s_far.normalized_radius, 1.0);
}

#[test]
fn test_angular_hysteresis_boundary_flutter_elimination() {
    // Sector boundary between 0 and 1 is theta = pi/3 ~ 1.047 rad
    // Without hysteresis: fluttering across boundary causes sector to oscillate 0 <-> 1
    let mut mapper_no_hyst = SpatialCoordinateMapper::new(6, (0.5, 0.5), 0.10);
    let s_a = mapper_no_hyst.map_coords(0.5 + 0.35 * 1.04_f32.cos(), 0.5 + 0.35 * 1.04_f32.sin());
    assert_eq!(s_a.active_sector, Some(0));
    let s_b = mapper_no_hyst.map_coords(0.5 + 0.35 * 1.06_f32.cos(), 0.5 + 0.35 * 1.06_f32.sin());
    assert_eq!(s_b.active_sector, Some(1));

    // With angular hysteresis (0.05 rad):
    let mut mapper_hyst = SpatialCoordinateMapper::new(6, (0.5, 0.5), 0.10).with_hysteresis(0.05);

    // Initialize in sector 0
    let s1 = mapper_hyst.map_coords(0.5 + 0.35 * 1.00_f32.cos(), 0.5 + 0.35 * 1.00_f32.sin());
    assert_eq!(s1.active_sector, Some(0));

    // Move just across nominal boundary (1.06 rad), within hysteresis margin -> stays sector 0!
    let s2 = mapper_hyst.map_coords(0.5 + 0.35 * 1.06_f32.cos(), 0.5 + 0.35 * 1.06_f32.sin());
    assert_eq!(s2.active_sector, Some(0));

    // Move far past hysteresis margin (1.15 rad) -> transitions to sector 1
    let s3 = mapper_hyst.map_coords(0.5 + 0.35 * 1.15_f32.cos(), 0.5 + 0.35 * 1.15_f32.sin());
    assert_eq!(s3.active_sector, Some(1));

    // Flutter slightly back into sector 0 (1.04 rad) -> stays sector 1 due to hysteresis!
    let s4 = mapper_hyst.map_coords(0.5 + 0.35 * 1.04_f32.cos(), 0.5 + 0.35 * 1.04_f32.sin());
    assert_eq!(s4.active_sector, Some(1));
}

#[test]
fn test_screen_viewport_mapping() {
    let viewport = ScreenBounds::new(100.0, 200.0, 1920.0, 1080.0);
    let mut mapper =
        SpatialCoordinateMapper::new(6, (0.5, 0.5), 0.10).with_viewport(viewport);

    let state = mapper.map_coords(0.5, 0.5);
    assert_eq!(state.screen_x, Some(100.0 + 0.5 * 1920.0));
    assert_eq!(state.screen_y, Some(200.0 + 0.5 * 1080.0));

    // Landmark mapping helper
    let lm = Landmark3D::new(0.85, 0.5, 0.0);
    let lm_state = mapper.map_landmark(&lm);
    assert_eq!(lm_state.active_sector, Some(0));
}
