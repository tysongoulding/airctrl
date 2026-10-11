//! Spatial Radial Dial Coordinate Mapper.

use airctrl_core::types::gesture::{DialCoordinates, Landmark3D};
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

/// Screen or window viewport bounds for pixel coordinate mapping.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScreenBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl ScreenBounds {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn map_normalized(&self, norm_x: f32, norm_y: f32) -> (f32, f32) {
        let px = self.x + norm_x.clamp(0.0, 1.0) * self.width;
        let py = self.y + norm_y.clamp(0.0, 1.0) * self.height;
        (px, py)
    }
}

/// State of the spatial radial pointer mapped from hand landmark coordinates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadialPointerState {
    /// True if pointer is within the center deadband zone.
    pub in_deadzone: bool,
    /// Polar angle in radians [0, 2pi).
    pub angle_rad: f32,
    /// Normalized radial distance from center [0.0, 1.0].
    pub normalized_radius: f32,
    /// Active radial dial sector index [0, sector_count - 1], or None if in deadzone.
    pub active_sector: Option<usize>,
    /// Mapped screen X pixel coordinate, if viewport is configured.
    pub screen_x: Option<f32>,
    /// Mapped screen Y pixel coordinate, if viewport is configured.
    pub screen_y: Option<f32>,
    /// True if selection gesture (e.g. pinch or dwell) is active.
    pub is_selected: bool,
}

impl RadialPointerState {
    /// Converts to airctrl-core `DialCoordinates` representation.
    pub fn to_dial_coordinates(&self) -> Option<DialCoordinates> {
        self.active_sector.map(|sector| DialCoordinates {
            angle_rad: self.angle_rad,
            normalized_radius: self.normalized_radius,
            active_sector: sector,
            is_selected: self.is_selected,
        })
    }
}

/// Configuration parameters for `SpatialCoordinateMapper`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapperConfig {
    pub sector_count: usize,
    pub center_x: f32,
    pub center_y: f32,
    pub deadzone_radius: f32,
    pub max_radius: f32,
    pub hysteresis_rad: f32,
    pub radial_hysteresis: f32,
    pub mirror_x: bool,
    pub sector_offset_rad: f32,
    pub viewport: Option<ScreenBounds>,
}

impl Default for MapperConfig {
    fn default() -> Self {
        Self {
            sector_count: 6,
            center_x: 0.5,
            center_y: 0.5,
            deadzone_radius: 0.10,
            max_radius: 0.50,
            hysteresis_rad: 0.0,
            radial_hysteresis: 0.0,
            mirror_x: false,
            sector_offset_rad: 0.0,
            viewport: None,
        }
    }
}

/// Maps normalized 3D hand landmark coordinates to polar radial dial sectors and screen coordinates.
#[derive(Debug, Clone)]
pub struct SpatialCoordinateMapper {
    config: MapperConfig,
    last_active_sector: Option<usize>,
    was_in_deadzone: bool,
}

impl Default for SpatialCoordinateMapper {
    fn default() -> Self {
        Self::new(6, (0.5, 0.5), 0.10)
    }
}

impl SpatialCoordinateMapper {
    /// Creates a new mapper with default parameters matching E2E harness specifications.
    pub fn new(sector_count: usize, center: (f32, f32), deadzone: f32) -> Self {
        Self {
            config: MapperConfig {
                sector_count,
                center_x: center.0,
                center_y: center.1,
                deadzone_radius: deadzone,
                ..Default::default()
            },
            last_active_sector: None,
            was_in_deadzone: true,
        }
    }

    /// Creates a mapper with full custom configuration.
    pub fn with_config(config: MapperConfig) -> Self {
        Self {
            config,
            last_active_sector: None,
            was_in_deadzone: true,
        }
    }

    /// Configures angular sector transition hysteresis in radians.
    pub fn with_hysteresis(mut self, hysteresis_rad: f32) -> Self {
        self.config.hysteresis_rad = hysteresis_rad.max(0.0);
        self
    }

    /// Configures horizontal mirroring (e.g. for webcam selfie view).
    pub fn with_mirroring(mut self, mirror_x: bool) -> Self {
        self.config.mirror_x = mirror_x;
        self
    }

    /// Configures screen bounds for pixel mapping.
    pub fn with_viewport(mut self, viewport: ScreenBounds) -> Self {
        self.config.viewport = Some(viewport);
        self
    }

    /// Resets internal hysteresis and state tracking.
    pub fn reset(&mut self) {
        self.last_active_sector = None;
        self.was_in_deadzone = true;
    }

    /// Maps normalized coordinates (x, y) into radial pointer state.
    pub fn map_coords(&mut self, x: f32, y: f32) -> RadialPointerState {
        let effective_x = if self.config.mirror_x { 1.0 - x } else { x };
        let effective_y = y;

        let dx = effective_x - self.config.center_x;
        let dy = effective_y - self.config.center_y;
        let radius = (dx * dx + dy * dy).sqrt();

        // Normalized radius clamped to 1.0
        let normalized_radius = (radius / self.config.max_radius).min(1.0);

        // Deadband evaluation with optional radial hysteresis
        let in_deadzone = if self.config.radial_hysteresis > 0.0 {
            if self.was_in_deadzone {
                normalized_radius < (self.config.deadzone_radius + self.config.radial_hysteresis)
            } else {
                normalized_radius
                    < (self.config.deadzone_radius - self.config.radial_hysteresis).max(0.0)
            }
        } else {
            normalized_radius < self.config.deadzone_radius
        };

        self.was_in_deadzone = in_deadzone;

        let (screen_x, screen_y) = if let Some(ref vp) = self.config.viewport {
            let (sx, sy) = vp.map_normalized(effective_x, effective_y);
            (Some(sx), Some(sy))
        } else {
            (None, None)
        };

        if in_deadzone {
            self.last_active_sector = None;
            return RadialPointerState {
                in_deadzone: true,
                angle_rad: 0.0,
                normalized_radius: (normalized_radius * 1000.0).round() / 1000.0,
                active_sector: None,
                screen_x,
                screen_y,
                is_selected: false,
            };
        }

        // Calculate angle [0, 2pi)
        let mut angle = dy.atan2(dx);
        if angle < 0.0 {
            angle += 2.0 * PI;
        }

        // Apply sector offset if configured
        let adjusted_angle = (angle - self.config.sector_offset_rad).rem_euclid(2.0 * PI);

        let sector_width = (2.0 * PI) / (self.config.sector_count as f32);
        let nominal_sector =
            ((adjusted_angle / sector_width).floor() as usize) % self.config.sector_count;

        // Apply angular hysteresis if enabled and there is a previous active sector
        let active_sector = if self.config.hysteresis_rad > 0.0 {
            if let Some(prev_sector) = self.last_active_sector {
                if self.is_within_sector_hysteresis(adjusted_angle, prev_sector, sector_width) {
                    prev_sector
                } else {
                    nominal_sector
                }
            } else {
                nominal_sector
            }
        } else {
            nominal_sector
        };

        self.last_active_sector = Some(active_sector);

        RadialPointerState {
            in_deadzone: false,
            angle_rad: (angle * 10000.0).round() / 10000.0,
            normalized_radius: (normalized_radius * 1000.0).round() / 1000.0,
            active_sector: Some(active_sector),
            screen_x,
            screen_y,
            is_selected: false,
        }
    }

    /// Evaluates whether an angle is within the hysteresis-expanded window of a sector.
    fn is_within_sector_hysteresis(&self, angle: f32, sector: usize, sector_width: f32) -> bool {
        let sector_center = (sector as f32 + 0.5) * sector_width;
        let delta = (angle - sector_center)
            .sin()
            .atan2((angle - sector_center).cos())
            .abs();
        let half_width = sector_width / 2.0;
        delta <= (half_width + self.config.hysteresis_rad)
    }

    /// Maps a 3D hand landmark (e.g. index fingertip) to radial pointer state.
    pub fn map_landmark(&mut self, landmark: &Landmark3D) -> RadialPointerState {
        self.map_coords(landmark.x, landmark.y)
    }

    /// Maps pointing ray from MCP knuckle through index tip to radial pointer state.
    pub fn map_pointing_ray(&mut self, mcp: &Landmark3D, tip: &Landmark3D) -> RadialPointerState {
        let dir_x = tip.x - mcp.x;
        let dir_y = tip.y - mcp.y;
        let len = (dir_x * dir_x + dir_y * dir_y).sqrt();

        if len < 1e-4 {
            return self.map_landmark(tip);
        }

        // Project ray forward by normalized distance
        let proj_x = tip.x + (dir_x / len) * 0.15;
        let proj_y = tip.y + (dir_y / len) * 0.15;

        self.map_coords(proj_x, proj_y)
    }
}
