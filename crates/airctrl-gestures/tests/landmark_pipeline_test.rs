//! Comprehensive Landmark Pipeline and Topology Test Suite.

use airctrl_core::types::gesture::Landmark3D;
use airctrl_gestures::error::GestureError;
use airctrl_gestures::frame::{FrameBuffer, PixelFormat};
use airctrl_gestures::topology::*;

#[test]
fn test_topology_constants_and_indices() {
    assert_eq!(WRIST, 0);
    assert_eq!(THUMB_CMC, 1);
    assert_eq!(THUMB_MCP, 2);
    assert_eq!(THUMB_IP, 3);
    assert_eq!(THUMB_TIP, 4);

    assert_eq!(INDEX_MCP, 5);
    assert_eq!(INDEX_PIP, 6);
    assert_eq!(INDEX_DIP, 7);
    assert_eq!(INDEX_TIP, 8);

    assert_eq!(MIDDLE_MCP, 9);
    assert_eq!(MIDDLE_PIP, 10);
    assert_eq!(MIDDLE_DIP, 11);
    assert_eq!(MIDDLE_TIP, 12);

    assert_eq!(RING_MCP, 13);
    assert_eq!(RING_PIP, 14);
    assert_eq!(RING_DIP, 15);
    assert_eq!(RING_TIP, 16);

    assert_eq!(PINKY_MCP, 17);
    assert_eq!(PINKY_PIP, 18);
    assert_eq!(PINKY_DIP, 19);
    assert_eq!(PINKY_TIP, 20);

    assert_eq!(TOTAL_LANDMARKS, 21);
    assert_eq!(
        FINGERTIP_INDICES,
        [THUMB_TIP, INDEX_TIP, MIDDLE_TIP, RING_TIP, PINKY_TIP]
    );
    assert_eq!(
        MCP_INDICES,
        [THUMB_CMC, INDEX_MCP, MIDDLE_MCP, RING_MCP, PINKY_MCP]
    );
}

#[test]
fn test_skeleton_connections_validity() {
    assert_eq!(SKELETON_CONNECTIONS.len(), 21);
    for &(j1, j2) in SKELETON_CONNECTIONS {
        assert!(j1 < TOTAL_LANDMARKS);
        assert!(j2 < TOTAL_LANDMARKS);
        assert_ne!(j1, j2);
    }
}

#[test]
fn test_landmark_ops_geometric_math() {
    let p1 = Landmark3D::new(0.0, 0.0, 0.0);
    let p2 = Landmark3D::new(3.0, 4.0, 0.0);
    assert!((p1.euclidean_dist_2d(&p2) - 5.0).abs() < 1e-5);

    let p3 = Landmark3D::new(1.0, 2.0, 2.0);
    assert!((p1.euclidean_dist_3d(&p3) - 3.0).abs() < 1e-5);

    // Bounds checking
    let valid = Landmark3D::new(0.5, 0.5, 0.0);
    assert!(valid.in_bounds());
    assert!(valid.is_valid());

    let out_of_bounds = Landmark3D::new(1.2, -0.1, 0.0);
    assert!(!out_of_bounds.in_bounds());
    assert!(out_of_bounds.is_valid());

    let nan_point = Landmark3D::new(f32::NAN, 0.5, 0.0);
    assert!(!nan_point.is_valid());

    let inf_point = Landmark3D::new(0.5, f32::INFINITY, 0.0);
    assert!(!inf_point.is_valid());
}

#[test]
fn test_normalized_landmark_list_array_conversion_and_accessors() {
    let mut arr = [Landmark3D::default(); TOTAL_LANDMARKS];
    arr[WRIST] = Landmark3D::new(0.5, 0.8, 0.0);
    arr[THUMB_TIP] = Landmark3D::new(0.35, 0.60, 0.0);
    arr[INDEX_TIP] = Landmark3D::new(0.48, 0.50, 0.0);
    arr[MIDDLE_TIP] = Landmark3D::new(0.55, 0.52, 0.0);
    arr[RING_TIP] = Landmark3D::new(0.62, 0.55, 0.0);
    arr[PINKY_TIP] = Landmark3D::new(0.68, 0.60, 0.0);

    let list = NormalizedLandmarkList::from_array(arr);
    assert_eq!(list.len(), 21);
    assert!(!list.is_empty());
    assert_eq!(list.wrist(), Some(&arr[WRIST]));
    assert_eq!(list.thumb_tip(), Some(&arr[THUMB_TIP]));
    assert_eq!(list.index_tip(), Some(&arr[INDEX_TIP]));
    assert_eq!(list.middle_tip(), Some(&arr[MIDDLE_TIP]));
    assert_eq!(list.ring_tip(), Some(&arr[RING_TIP]));
    assert_eq!(list.pinky_tip(), Some(&arr[PINKY_TIP]));

    // Roundtrip
    let back_to_arr = list.to_array().unwrap();
    assert_eq!(back_to_arr, arr);

    // Palm center calculation
    let center = list.palm_center().unwrap();
    assert!(center.x > 0.0 && center.x < 1.0);
    assert!(center.y > 0.0 && center.y < 1.0);

    // Invalid length error
    let invalid_len = vec![Landmark3D::default(); 10];
    let err = NormalizedLandmarkList::new(invalid_len).unwrap_err();
    match err {
        GestureError::InsufficientLandmarks(n) => assert_eq!(n, 10),
        _ => panic!("Expected InsufficientLandmarks error"),
    }
}

#[test]
fn test_frame_buffer_validation_and_zero_copy_view() {
    // 1. Dimensions validation
    let zero_dims = FrameBuffer::new(vec![0; 100], 0, 100, 100, PixelFormat::Rgb8);
    assert!(matches!(
        zero_dims,
        Err(GestureError::InvalidFrameDimensions { .. })
    ));

    // 2. Buffer length validation
    let small_buffer = FrameBuffer::new(vec![0; 10], 64, 64, 64 * 3, PixelFormat::Rgb8);
    assert!(matches!(
        small_buffer,
        Err(GestureError::InvalidBufferLength { .. })
    ));

    // 3. Valid RGB8 FrameBuffer
    let mut fb = FrameBuffer::allocate(100, 100, PixelFormat::Rgb8);
    assert_eq!(fb.stride, 300);
    assert_eq!(fb.data.len(), 30000);

    fb.set_pixel(10, 20, 255, 128, 64);
    assert_eq!(fb.get_pixel(10, 20), Some((255, 128, 64)));

    // Out of bounds get/set
    assert_eq!(fb.get_pixel(200, 200), None);
    fb.set_pixel(200, 200, 1, 2, 3); // gracefully ignored

    // 4. Zero-copy view
    let fb_ref = fb.as_ref();
    assert_eq!(fb_ref.width, 100);
    assert_eq!(fb_ref.height, 100);
    assert_eq!(fb_ref.get_pixel(10, 20), Some((255, 128, 64)));

    // 5. PixelFormat variations
    let mut bgr_frame = FrameBuffer::allocate(10, 10, PixelFormat::Bgr8);
    bgr_frame.set_pixel(0, 0, 255, 0, 0); // Red
    assert_eq!(bgr_frame.get_pixel(0, 0), Some((255, 0, 0)));
    // In raw bytes, BGR should have B=0 at offset 0, G=0 at offset 1, R=255 at offset 2
    assert_eq!(bgr_frame.data[0], 0);
    assert_eq!(bgr_frame.data[1], 0);
    assert_eq!(bgr_frame.data[2], 255);

    let mut rgba_frame = FrameBuffer::allocate(10, 10, PixelFormat::Rgba8);
    assert_eq!(rgba_frame.format.bytes_per_pixel(), 4);
    assert!(rgba_frame.format.has_alpha());
    rgba_frame.set_pixel(1, 1, 10, 20, 30);
    assert_eq!(rgba_frame.get_pixel(1, 1), Some((10, 20, 30)));
    // to_rgb8
    let rgb_bytes = rgba_frame.as_ref().to_rgb8();
    assert_eq!(rgb_bytes.len(), 10 * 10 * 3);
}
