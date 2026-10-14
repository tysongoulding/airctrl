use airctrl_core::types::{DimensionDelta, ResizeMode, SnapPosition, WindowRect};

/// Pure-math geometry engine computing deterministic window coordinates
/// for 9-way snapping and dynamic resizing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowGeometryEngine {
    pub screen_w: u32,
    pub screen_h: u32,
    pub origin_x: i32,
    pub origin_y: i32,
}

impl Default for WindowGeometryEngine {
    fn default() -> Self {
        Self::new(1920, 1080)
    }
}

impl WindowGeometryEngine {
    pub fn new(screen_w: u32, screen_h: u32) -> Self {
        Self {
            screen_w,
            screen_h,
            origin_x: 0,
            origin_y: 0,
        }
    }

    pub fn for_work_area(work_area: WindowRect) -> Self {
        Self {
            screen_w: work_area.width,
            screen_h: work_area.height,
            origin_x: work_area.x,
            origin_y: work_area.y,
        }
    }

    /// Computes target WindowRect for each of the 9 snap positions.
    pub fn compute_snap(&self, position: SnapPosition) -> WindowRect {
        let half_w = self.screen_w / 2;
        let half_h = self.screen_h / 2;

        match position {
            SnapPosition::Left => WindowRect {
                x: self.origin_x,
                y: self.origin_y,
                width: half_w,
                height: self.screen_h,
            },
            SnapPosition::Right => WindowRect {
                x: self.origin_x + half_w as i32,
                y: self.origin_y,
                width: self.screen_w - half_w,
                height: self.screen_h,
            },
            SnapPosition::Top => WindowRect {
                x: self.origin_x,
                y: self.origin_y,
                width: self.screen_w,
                height: half_h,
            },
            SnapPosition::Bottom => WindowRect {
                x: self.origin_x,
                y: self.origin_y + half_h as i32,
                width: self.screen_w,
                height: self.screen_h - half_h,
            },
            SnapPosition::Center => {
                let w = ((self.screen_w as u64 * 70) / 100) as u32;
                let h = ((self.screen_h as u64 * 70) / 100) as u32;
                WindowRect {
                    x: self.origin_x + ((self.screen_w - w) / 2) as i32,
                    y: self.origin_y + ((self.screen_h - h) / 2) as i32,
                    width: w,
                    height: h,
                }
            }
            SnapPosition::Maximize => WindowRect {
                x: self.origin_x,
                y: self.origin_y,
                width: self.screen_w,
                height: self.screen_h,
            },
            SnapPosition::Minimize => WindowRect {
                x: self.origin_x,
                y: self.origin_y,
                width: 0,
                height: 0,
            },
            SnapPosition::Restore => WindowRect {
                x: self.origin_x + 100,
                y: self.origin_y + 100,
                width: 1024.min(self.screen_w),
                height: 768.min(self.screen_h),
            },
            SnapPosition::Fullscreen => WindowRect {
                x: self.origin_x,
                y: self.origin_y,
                width: self.screen_w,
                height: self.screen_h,
            },
        }
    }

    /// Computes new WindowRect based on dynamic resizing mode and bounds safety floors.
    pub fn compute_resize(&self, current: WindowRect, mode: &ResizeMode) -> WindowRect {
        let x = current.x;
        let y = current.y;
        let mut w = current.width;
        let mut h = current.height;

        match mode {
            ResizeMode::Explicit { width, height } => {
                let new_w = (*width).clamp(200, self.screen_w);
                let new_h = (*height).clamp(200, self.screen_h);
                let new_x = if x + (new_w as i32) > self.origin_x + (self.screen_w as i32) {
                    self.origin_x + (self.screen_w - new_w) as i32
                } else {
                    x.max(self.origin_x)
                };
                let new_y = if y + (new_h as i32) > self.origin_y + (self.screen_h as i32) {
                    self.origin_y + (self.screen_h - new_h) as i32
                } else {
                    y.max(self.origin_y)
                };
                WindowRect {
                    x: new_x,
                    y: new_y,
                    width: new_w,
                    height: new_h,
                }
            }
            ResizeMode::Percentage { percentage } => {
                let pct = (*percentage).clamp(10, 100) as u64;
                let new_w = ((self.screen_w as u64 * pct) / 100) as u32;
                let new_h = ((self.screen_h as u64 * pct) / 100) as u32;
                let new_x = self.origin_x + ((self.screen_w - new_w) / 2) as i32;
                let new_y = self.origin_y + ((self.screen_h - new_h) / 2) as i32;
                WindowRect {
                    x: new_x,
                    y: new_y,
                    width: new_w,
                    height: new_h,
                }
            }
            ResizeMode::Delta { dimension, delta_pixels } => {
                let delta = *delta_pixels;
                match dimension {
                    DimensionDelta::Larger => {
                        w = w.saturating_add(delta);
                        h = h.saturating_add(delta);
                    }
                    DimensionDelta::Smaller => {
                        w = w.saturating_sub(delta);
                        h = h.saturating_sub(delta);
                    }
                    DimensionDelta::Wider => {
                        w = w.saturating_add(delta);
                    }
                    DimensionDelta::Narrower => {
                        w = w.saturating_sub(delta);
                    }
                    DimensionDelta::Taller => {
                        h = h.saturating_add(delta);
                    }
                    DimensionDelta::Shorter => {
                        h = h.saturating_sub(delta);
                    }
                }

                // Minimum safety floor: 200x200px; Maximum ceiling: screen dimensions
                let clamped_w = w.clamp(200, self.screen_w);
                let clamped_h = h.clamp(200, self.screen_h);

                // Prevent window from extending beyond screen boundaries
                let clamped_x = if x + (clamped_w as i32) > self.origin_x + (self.screen_w as i32) {
                    (self.origin_x + (self.screen_w - clamped_w) as i32).max(self.origin_x)
                } else {
                    x.max(self.origin_x)
                };
                let clamped_y = if y + (clamped_h as i32) > self.origin_y + (self.screen_h as i32) {
                    (self.origin_y + (self.screen_h - clamped_h) as i32).max(self.origin_y)
                } else {
                    y.max(self.origin_y)
                };

                WindowRect {
                    x: clamped_x,
                    y: clamped_y,
                    width: clamped_w,
                    height: clamped_h,
                }
            }
        }
    }
}
