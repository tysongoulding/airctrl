use serde::{Deserialize, Serialize};

/// State of the center-focus radial dial overlay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadialDialState {
    pub is_visible: bool,
    pub active_sector: Option<usize>,
    pub sector_count: usize,
    pub timeout_secs: f32,
}

/// In-memory state machine for the center-focus radial dial pointer overlay (`Ctrl+Alt`).
#[derive(Debug, Clone)]
pub struct RadialDialOverlay {
    is_visible: bool,
    active_sector: Option<usize>,
    sector_count: usize,
    timeout_secs: f32,
}

impl RadialDialOverlay {
    pub const DEFAULT_SECTOR_COUNT: usize = 6;
    pub const DEFAULT_TIMEOUT_SECS: f32 = 3.0;

    pub fn new() -> Self {
        Self {
            is_visible: false,
            active_sector: None,
            sector_count: Self::DEFAULT_SECTOR_COUNT,
            timeout_secs: Self::DEFAULT_TIMEOUT_SECS,
        }
    }

    pub fn with_sectors(sector_count: usize) -> Self {
        Self {
            is_visible: false,
            active_sector: None,
            sector_count,
            timeout_secs: Self::DEFAULT_TIMEOUT_SECS,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    pub fn active_sector(&self) -> Option<usize> {
        self.active_sector
    }

    pub fn sector_count(&self) -> usize {
        self.sector_count
    }

    /// Dynamically updates the sector count while preserving visibility.
    /// Resets active_sector to None if the previously hovered sector is now out of bounds.
    pub fn set_sector_count(&mut self, count: usize) {
        self.sector_count = count;
        if self.active_sector.is_some_and(|s| s >= count) {
            self.active_sector = None;
        }
    }

    pub fn timeout_secs(&self) -> f32 {
        self.timeout_secs
    }

    /// Toggles dial visibility. If hiding, clears active sector.
    pub fn toggle(&mut self) {
        self.is_visible = !self.is_visible;
        if !self.is_visible {
            self.active_sector = None;
        }
    }

    pub fn show(&mut self) {
        self.is_visible = true;
    }

    pub fn hide(&mut self) {
        self.is_visible = false;
        self.active_sector = None;
    }

    /// Updates the active sector if dial is visible and sector index is in valid range [0, sector_count).
    /// Out-of-bounds indices are ignored.
    pub fn hover_sector(&mut self, sector_idx: i32) {
        if self.is_visible && sector_idx >= 0 && (sector_idx as usize) < self.sector_count {
            self.active_sector = Some(sector_idx as usize);
        }
    }

    /// Inactivity timeout dismisses overlay and clears selection.
    pub fn on_inactivity_timeout(&mut self) {
        self.is_visible = false;
        self.active_sector = None;
    }

    pub fn snapshot(&self) -> RadialDialState {
        RadialDialState {
            is_visible: self.is_visible,
            active_sector: self.active_sector,
            sector_count: self.sector_count,
            timeout_secs: self.timeout_secs,
        }
    }
}

impl Default for RadialDialOverlay {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_sector_count() {
        let mut dial = RadialDialOverlay::new();
        dial.show();
        dial.hover_sector(5);
        assert_eq!(dial.active_sector(), Some(5));
        assert!(dial.is_visible());

        // Shrink sector count so active_sector is out of bounds (5 >= 4)
        dial.set_sector_count(4);
        assert_eq!(dial.sector_count(), 4);
        assert!(dial.is_visible());
        assert_eq!(dial.active_sector(), None);

        // Hover new valid sector within [0, 4)
        dial.hover_sector(3);
        assert_eq!(dial.active_sector(), Some(3));

        // Expand sector count (3 < 8, so active_sector remains Some(3))
        dial.set_sector_count(8);
        assert_eq!(dial.sector_count(), 8);
        assert!(dial.is_visible());
        assert_eq!(dial.active_sector(), Some(3));
    }
}

