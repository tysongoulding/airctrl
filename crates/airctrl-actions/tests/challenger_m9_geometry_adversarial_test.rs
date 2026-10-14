use airctrl_actions::geometry::WindowGeometryEngine;
use airctrl_core::error::CoreError;
use airctrl_core::platform::mock::MockWindowOps;
use airctrl_core::platform::PlatformWindowOps;
use airctrl_core::types::{
    DimensionDelta, ResizeMode, SnapPosition, WindowInfo, WindowRect,
};

/// 1. Stress-test WindowGeometryEngine across extreme display work areas and all 9 snap positions.
#[test]
fn test_adversarial_extreme_display_resolutions_all_9_snap_positions() {
    let test_displays = [
        ("8K UHD", WindowRect { x: 0, y: 0, width: 7680, height: 4320 }),
        ("4K UHD", WindowRect { x: 0, y: 0, width: 3840, height: 2160 }),
        ("Small VGA", WindowRect { x: 0, y: 0, width: 640, height: 480 }),
        ("Ultra-wide DQHD", WindowRect { x: 0, y: 0, width: 5120, height: 1440 }),
        ("Multi-monitor Left Offset", WindowRect { x: -1920, y: 0, width: 1920, height: 1080 }),
        ("Multi-monitor Right Offset", WindowRect { x: 1920, y: 0, width: 1920, height: 1080 }),
        ("Multi-monitor Top Offset", WindowRect { x: 0, y: -1080, width: 1920, height: 1080 }),
        ("Taskbar Inset Offset", WindowRect { x: 64, y: 32, width: 1856, height: 1048 }),
    ];

    let all_snap_positions = [
        SnapPosition::Left,
        SnapPosition::Right,
        SnapPosition::Top,
        SnapPosition::Bottom,
        SnapPosition::Center,
        SnapPosition::Maximize,
        SnapPosition::Minimize,
        SnapPosition::Restore,
        SnapPosition::Fullscreen,
    ];

    for (desc, work_area) in test_displays {
        let engine = WindowGeometryEngine::for_work_area(work_area);

        for snap in all_snap_positions {
            let rect = engine.compute_snap(snap);

            match snap {
                SnapPosition::Left => {
                    assert_eq!(rect.x, work_area.x, "{}: Left snap x mismatch", desc);
                    assert_eq!(rect.y, work_area.y, "{}: Left snap y mismatch", desc);
                    assert_eq!(rect.width, work_area.width / 2, "{}: Left snap width mismatch", desc);
                    assert_eq!(rect.height, work_area.height, "{}: Left snap height mismatch", desc);
                }
                SnapPosition::Right => {
                    let half_w = work_area.width / 2;
                    assert_eq!(rect.x, work_area.x + half_w as i32, "{}: Right snap x mismatch", desc);
                    assert_eq!(rect.y, work_area.y, "{}: Right snap y mismatch", desc);
                    assert_eq!(rect.width, work_area.width - half_w, "{}: Right snap width mismatch", desc);
                    assert_eq!(rect.height, work_area.height, "{}: Right snap height mismatch", desc);
                    // Perfect seam: left half + right half equals exact screen width
                    assert_eq!(half_w + rect.width, work_area.width, "{}: Left + Right width sum seam", desc);
                }
                SnapPosition::Top => {
                    assert_eq!(rect.x, work_area.x, "{}: Top snap x mismatch", desc);
                    assert_eq!(rect.y, work_area.y, "{}: Top snap y mismatch", desc);
                    assert_eq!(rect.width, work_area.width, "{}: Top snap width mismatch", desc);
                    assert_eq!(rect.height, work_area.height / 2, "{}: Top snap height mismatch", desc);
                }
                SnapPosition::Bottom => {
                    let half_h = work_area.height / 2;
                    assert_eq!(rect.x, work_area.x, "{}: Bottom snap x mismatch", desc);
                    assert_eq!(rect.y, work_area.y + half_h as i32, "{}: Bottom snap y mismatch", desc);
                    assert_eq!(rect.width, work_area.width, "{}: Bottom snap width mismatch", desc);
                    assert_eq!(rect.height, work_area.height - half_h, "{}: Bottom snap height mismatch", desc);
                    assert_eq!(half_h + rect.height, work_area.height, "{}: Top + Bottom height sum seam", desc);
                }
                SnapPosition::Center => {
                    let expected_w = ((work_area.width as u64 * 70) / 100) as u32;
                    let expected_h = ((work_area.height as u64 * 70) / 100) as u32;
                    let expected_x = work_area.x + ((work_area.width - expected_w) / 2) as i32;
                    let expected_y = work_area.y + ((work_area.height - expected_h) / 2) as i32;
                    assert_eq!(rect.x, expected_x, "{}: Center snap x mismatch", desc);
                    assert_eq!(rect.y, expected_y, "{}: Center snap y mismatch", desc);
                    assert_eq!(rect.width, expected_w, "{}: Center snap width mismatch", desc);
                    assert_eq!(rect.height, expected_h, "{}: Center snap height mismatch", desc);
                    // Verify centered window remains strictly within work area
                    assert!(rect.x >= work_area.x);
                    assert!(rect.y >= work_area.y);
                    assert!(rect.x + rect.width as i32 <= work_area.x + work_area.width as i32);
                    assert!(rect.y + rect.height as i32 <= work_area.y + work_area.height as i32);
                }
                SnapPosition::Maximize => {
                    assert_eq!(rect.x, work_area.x, "{}: Maximize x mismatch", desc);
                    assert_eq!(rect.y, work_area.y, "{}: Maximize y mismatch", desc);
                    assert_eq!(rect.width, work_area.width, "{}: Maximize width mismatch", desc);
                    assert_eq!(rect.height, work_area.height, "{}: Maximize height mismatch", desc);
                }
                SnapPosition::Minimize => {
                    assert_eq!(rect.width, 0, "{}: Minimize width must be 0", desc);
                    assert_eq!(rect.height, 0, "{}: Minimize height must be 0", desc);
                }
                SnapPosition::Restore => {
                    assert_eq!(rect.x, work_area.x + 100, "{}: Restore x mismatch", desc);
                    assert_eq!(rect.y, work_area.y + 100, "{}: Restore y mismatch", desc);
                    assert_eq!(rect.width, 1024.min(work_area.width), "{}: Restore width mismatch", desc);
                    assert_eq!(rect.height, 768.min(work_area.height), "{}: Restore height mismatch", desc);
                }
                SnapPosition::Fullscreen => {
                    assert_eq!(rect.x, work_area.x, "{}: Fullscreen x mismatch", desc);
                    assert_eq!(rect.y, work_area.y, "{}: Fullscreen y mismatch", desc);
                    assert_eq!(rect.width, work_area.width, "{}: Fullscreen width mismatch", desc);
                    assert_eq!(rect.height, work_area.height, "{}: Fullscreen height mismatch", desc);
                }
            }
        }
    }
}

/// 2. Stress-test Dynamic Resizing: Explicit mode clamping and screen boundary containment.
#[test]
fn test_adversarial_explicit_resizing_clamping_and_boundaries() {
    let engine = WindowGeometryEngine::new(1920, 1080);
    let initial = WindowRect { x: 500, y: 300, width: 800, height: 600 };

    // Explicit dimensions below 200px floor must clamp up to 200px
    let small_req = engine.compute_resize(initial, &ResizeMode::Explicit { width: 50, height: 20 });
    assert_eq!(small_req.width, 200, "Explicit width 50 must clamp to 200px floor");
    assert_eq!(small_req.height, 200, "Explicit height 20 must clamp to 200px floor");

    // Explicit dimensions of 0px must clamp up to 200px
    let zero_req = engine.compute_resize(initial, &ResizeMode::Explicit { width: 0, height: 0 });
    assert_eq!(zero_req.width, 200);
    assert_eq!(zero_req.height, 200);

    // Explicit dimensions exceeding display must clamp to screen dimensions
    let huge_req = engine.compute_resize(initial, &ResizeMode::Explicit { width: 100_000, height: 50_000 });
    assert_eq!(huge_req.width, 1920);
    assert_eq!(huge_req.height, 1080);
    // Origin must be clamped to 0,0 so window does not hang off the screen
    assert_eq!(huge_req.x, 0);
    assert_eq!(huge_req.y, 0);

    // Window pushed near bottom-right edge: position must clamp so window remains on screen
    let near_edge = WindowRect { x: 1800, y: 1000, width: 100, height: 50 };
    let clamped_edge = engine.compute_resize(near_edge, &ResizeMode::Explicit { width: 400, height: 300 });
    assert_eq!(clamped_edge.width, 400);
    assert_eq!(clamped_edge.height, 300);
    assert_eq!(clamped_edge.x, 1920 - 400);
    assert_eq!(clamped_edge.y, 1080 - 300);

    // Window with negative coordinates (offscreen left/top): must clamp to screen origin
    let offscreen = WindowRect { x: -500, y: -300, width: 400, height: 300 };
    let clamped_offscreen = engine.compute_resize(offscreen, &ResizeMode::Explicit { width: 400, height: 300 });
    assert_eq!(clamped_offscreen.x, 0);
    assert_eq!(clamped_offscreen.y, 0);
}

/// 3. Stress-test Dynamic Resizing: Delta adjustments underflow, overflow, and safety floors.
#[test]
fn test_adversarial_delta_resizing_underflow_overflow_and_floors() {
    let engine = WindowGeometryEngine::new(1920, 1080);
    let cur = WindowRect { x: 400, y: 200, width: 250, height: 250 };

    // 1. Extreme shrinkage resulting in negative sizes: must clamp to 200px floor without underflow
    let smaller_huge = engine.compute_resize(
        cur,
        &ResizeMode::Delta {
            dimension: DimensionDelta::Smaller,
            delta_pixels: 50_000,
        },
    );
    assert_eq!(smaller_huge.width, 200, "Delta Smaller 50,000px must clamp to 200px floor");
    assert_eq!(smaller_huge.height, 200, "Delta Smaller 50,000px must clamp to 200px floor");

    // 2. Narrower extreme shrinkage
    let narrower_huge = engine.compute_resize(
        cur,
        &ResizeMode::Delta {
            dimension: DimensionDelta::Narrower,
            delta_pixels: 100_000,
        },
    );
    assert_eq!(narrower_huge.width, 200);
    assert_eq!(narrower_huge.height, 250);

    // 3. Shorter extreme shrinkage
    let shorter_huge = engine.compute_resize(
        cur,
        &ResizeMode::Delta {
            dimension: DimensionDelta::Shorter,
            delta_pixels: 100_000,
        },
    );
    assert_eq!(shorter_huge.width, 250);
    assert_eq!(shorter_huge.height, 200);

    // 4. Extreme enlargement resulting in > screen dimensions: must clamp to screen dimensions
    let larger_huge = engine.compute_resize(
        cur,
        &ResizeMode::Delta {
            dimension: DimensionDelta::Larger,
            delta_pixels: 50_000,
        },
    );
    assert_eq!(larger_huge.width, 1920);
    assert_eq!(larger_huge.height, 1080);
    // Position clamped to fit
    assert_eq!(larger_huge.x, 0);
    assert_eq!(larger_huge.y, 0);

    // 5. Window located far outside screen bounds (x = 3000, y = 2000)
    let cur_outside = WindowRect { x: 3000, y: 2000, width: 400, height: 300 };
    let delta_outside = engine.compute_resize(
        cur_outside,
        &ResizeMode::Delta {
            dimension: DimensionDelta::Wider,
            delta_pixels: 100,
        },
    );
    assert_eq!(delta_outside.width, 500);
    assert_eq!(delta_outside.x, 1920 - 500);
    assert_eq!(delta_outside.y, 1080 - 300);
}

/// 4. Stress-test Dynamic Resizing: Percentage scaling edge cases (sub-10%, over-100%, and containment).
#[test]
fn test_adversarial_percentage_resizing_clamping_and_containment() {
    let engine = WindowGeometryEngine::new(1920, 1080);
    let initial = WindowRect { x: 200, y: 150, width: 800, height: 600 };

    // Sub-10% percentages (0%, 1%, 5%): clamp to 10%
    let p_0 = engine.compute_resize(initial, &ResizeMode::Percentage { percentage: 0 });
    let p_1 = engine.compute_resize(initial, &ResizeMode::Percentage { percentage: 1 });
    let p_5 = engine.compute_resize(initial, &ResizeMode::Percentage { percentage: 5 });
    let p_10 = engine.compute_resize(initial, &ResizeMode::Percentage { percentage: 10 });

    assert_eq!(p_0, p_10, "Percentage 0% must clamp to 10%");
    assert_eq!(p_1, p_10, "Percentage 1% must clamp to 10%");
    assert_eq!(p_5, p_10, "Percentage 5% must clamp to 10%");
    assert_eq!(p_10.width, 192);
    assert_eq!(p_10.height, 108);

    // Over-100% percentages (150%, 250%, 1000%): clamp to 100%
    let p_150 = engine.compute_resize(initial, &ResizeMode::Percentage { percentage: 150 });
    let p_250 = engine.compute_resize(initial, &ResizeMode::Percentage { percentage: 250 });
    let p_255 = engine.compute_resize(initial, &ResizeMode::Percentage { percentage: 255 });
    let p_100 = engine.compute_resize(initial, &ResizeMode::Percentage { percentage: 100 });

    assert_eq!(p_150, p_100, "Percentage 150% must clamp to 100%");
    assert_eq!(p_250, p_100, "Percentage 250% must clamp to 100%");
    assert_eq!(p_255, p_100, "Percentage 255% must clamp to 100%");
    assert_eq!(p_100.width, 1920);
    assert_eq!(p_100.height, 1080);
    assert_eq!(p_100.x, 0);
    assert_eq!(p_100.y, 0);

    // Centering verification: window at 50% must be centered horizontally and vertically
    let p_50 = engine.compute_resize(initial, &ResizeMode::Percentage { percentage: 50 });
    assert_eq!(p_50.width, 960);
    assert_eq!(p_50.height, 540);
    assert_eq!(p_50.x, (1920 - 960) / 2);
    assert_eq!(p_50.y, (1080 - 540) / 2);

    // Large initial window (taller than screen, e.g. 1200px on 1080p display) must not panic
    let taller_window = WindowRect { x: 100, y: 50, width: 800, height: 1200 };
    let p_taller = engine.compute_resize(taller_window, &ResizeMode::Percentage { percentage: 70 });
    assert_eq!(p_taller.width, (1920 * 70) / 100);
    assert_eq!(p_taller.height, (1080 * 70) / 100);
}

/// 5. Stress-test PlatformWindowOps bounds mutations, state consistency, and error handling.
#[test]
fn test_adversarial_platform_window_traits_and_bounds_mutations() {
    let mock = MockWindowOps::new();
    mock.set_work_area(WindowRect { x: 0, y: 0, width: 2560, height: 1440 });

    // Initial state: no windows
    assert!(mock.enumerate_windows().unwrap().is_empty());
    assert!(mock.get_active_window().is_err());

    // Populate mock windows
    let w1 = WindowInfo {
        id: 101,
        process_id: 1001,
        process_name: "alacritty.exe".into(),
        title: "Terminal".into(),
        bounds: WindowRect { x: 50, y: 50, width: 800, height: 600 },
        is_focused: true,
        is_minimized: false,
    };
    let w2 = WindowInfo {
        id: 102,
        process_id: 1002,
        process_name: "code.exe".into(),
        title: "Editor".into(),
        bounds: WindowRect { x: 100, y: 100, width: 1200, height: 800 },
        is_focused: false,
        is_minimized: false,
    };
    let w3 = WindowInfo {
        id: 103,
        process_id: 1003,
        process_name: "firefox.exe".into(),
        title: "Browser".into(),
        bounds: WindowRect { x: 200, y: 200, width: 1400, height: 900 },
        is_focused: false,
        is_minimized: false,
    };

    mock.set_windows(vec![w1.clone(), w2.clone(), w3.clone()]);
    assert_eq!(mock.enumerate_windows().unwrap().len(), 3);
    assert_eq!(mock.get_active_window().unwrap().id, 101);

    // 1. Bounds mutation on non-existent window must fail with WindowNotFound
    let err = mock.set_window_bounds(999, WindowRect { x: 0, y: 0, width: 500, height: 500 });
    assert!(matches!(err, Err(CoreError::WindowNotFound(999))));

    // 2. Valid bounds mutation
    let new_bounds = WindowRect { x: 300, y: 300, width: 900, height: 700 };
    mock.set_window_bounds(101, new_bounds).unwrap();
    assert_eq!(mock.get_active_window().unwrap().bounds, new_bounds);
    let history = mock.get_bounds_history();
    assert_eq!(history.last(), Some(&(101, new_bounds)));

    // 3. Maximize window uses configured work area
    mock.maximize_window(102).unwrap();
    let w2_max = mock.enumerate_windows().unwrap().into_iter().find(|w| w.id == 102).unwrap();
    assert_eq!(w2_max.bounds, WindowRect { x: 0, y: 0, width: 2560, height: 1440 });

    // 4. Restore window uses 1024x768 at (100, 100)
    mock.restore_window(102).unwrap();
    let w2_restore = mock.enumerate_windows().unwrap().into_iter().find(|w| w.id == 102).unwrap();
    assert_eq!(w2_restore.bounds, WindowRect { x: 100, y: 100, width: 1024, height: 768 });

    // 5. Minimize sets 0x0 and is_minimized = true
    mock.minimize_window(103).unwrap();
    let w3_min = mock.enumerate_windows().unwrap().into_iter().find(|w| w.id == 103).unwrap();
    assert!(w3_min.is_minimized);
    assert_eq!(w3_min.bounds, WindowRect { x: 0, y: 0, width: 0, height: 0 });

    // 6. Close window removes from windows and z_order
    mock.close_window(103).unwrap();
    assert_eq!(mock.enumerate_windows().unwrap().len(), 2);
    assert_eq!(mock.get_z_order(), vec![101, 102]);
    // Closing already closed window returns WindowNotFound
    assert!(matches!(mock.close_window(103), Err(CoreError::WindowNotFound(103))));
}

/// 6. Stress-test z-order cycling matrix across empty, single-window, and multi-window configurations.
#[test]
fn test_adversarial_z_order_cycling_matrix() {
    let mock = MockWindowOps::new();

    // Case A: Empty window manager
    let err_fwd = mock.cycle_window(true);
    assert!(err_fwd.is_err(), "Cycling with 0 windows must return error");
    let err_bwd = mock.cycle_window(false);
    assert!(err_bwd.is_err(), "Cycling backwards with 0 windows must return error");

    // Case B: Single window
    let single = WindowInfo {
        id: 42,
        process_id: 1042,
        process_name: "sole_app.exe".into(),
        title: "Sole Window".into(),
        bounds: WindowRect { x: 0, y: 0, width: 800, height: 600 },
        is_focused: true,
        is_minimized: false,
    };
    mock.set_windows(vec![single.clone()]);
    let cycled_single_fwd = mock.cycle_window(true).unwrap();
    assert_eq!(cycled_single_fwd.id, 42);
    let cycled_single_bwd = mock.cycle_window(false).unwrap();
    assert_eq!(cycled_single_bwd.id, 42);

    // Case C: 5 Windows full cycling ring
    let windows: Vec<WindowInfo> = (1..=5)
        .map(|i| WindowInfo {
            id: i,
            process_id: 1000 + i as u32,
            process_name: format!("app{}.exe", i),
            title: format!("Window {}", i),
            bounds: WindowRect { x: 0, y: 0, width: 500, height: 500 },
            is_focused: i == 1,
            is_minimized: false,
        })
        .collect();
    mock.set_windows(windows);

    // Verify initial active is 1
    assert_eq!(mock.get_active_window().unwrap().id, 1);

    // Cycle forward: 1 -> 2 -> 3 -> 4 -> 5 -> 1
    let expected_fwd = [2, 3, 4, 5, 1];
    for expected_id in expected_fwd {
        let active = mock.cycle_window(true).unwrap();
        assert_eq!(active.id, expected_id, "Forward cycle expected id {}", expected_id);
        assert_eq!(mock.get_active_window().unwrap().id, expected_id);
    }

    // Cycle backward: 1 -> 5 -> 4 -> 3 -> 2 -> 1
    let expected_bwd = [5, 4, 3, 2, 1];
    for expected_id in expected_bwd {
        let active = mock.cycle_window(false).unwrap();
        assert_eq!(active.id, expected_id, "Backward cycle expected id {}", expected_id);
        assert_eq!(mock.get_active_window().unwrap().id, expected_id);
    }

    // Case D: Active window is not in windows list (e.g. desktop or unmanaged window active)
    mock.clear();
    let unmanaged = WindowInfo {
        id: 999,
        process_id: 9999,
        process_name: "unmanaged.exe".into(),
        title: "Unmanaged Desktop".into(),
        bounds: WindowRect { x: 0, y: 0, width: 1920, height: 1080 },
        is_focused: true,
        is_minimized: false,
    };
    let managed_wins: Vec<WindowInfo> = (1..=3)
        .map(|i| WindowInfo {
            id: i,
            process_id: 1000 + i as u32,
            process_name: format!("app{}.exe", i),
            title: format!("Window {}", i),
            bounds: WindowRect { x: 0, y: 0, width: 500, height: 500 },
            is_focused: false,
            is_minimized: false,
        })
        .collect();
    mock.set_windows(managed_wins);
    mock.set_active_window(unmanaged); // Active window is not one of managed_wins
    let recovered_cycle = mock.cycle_window(true).unwrap();
    // Must gracefully fall back to indexing first managed window rather than crashing
    assert!(recovered_cycle.id <= 3);
}
