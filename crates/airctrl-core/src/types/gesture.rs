use serde::{Deserialize, Serialize};

/// 3D landmark coordinate in normalized coordinate space.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Landmark3D {
    pub x: f32, // Normalized [0.0, 1.0] across image width
    pub y: f32, // Normalized [0.0, 1.0] across image height
    pub z: f32, // Relative depth coordinate
}

impl Landmark3D {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Handedness {
    Left,
    Right,
}

/// 21 MediaPipe hand landmarks topology.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HandLandmarks {
    pub landmarks: [Landmark3D; 21],
    pub handedness: Handedness,
    pub confidence: f32,
}

impl HandLandmarks {
    pub fn new(landmarks: [Landmark3D; 21], handedness: Handedness, confidence: f32) -> Self {
        Self {
            landmarks,
            handedness,
            confidence,
        }
    }
}

impl Default for HandLandmarks {
    fn default() -> Self {
        Self {
            landmarks: [Landmark3D::default(); 21],
            handedness: Handedness::Right,
            confidence: 1.0,
        }
    }
}

pub type NormalizedLandmark = Landmark3D;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GestureType {
    Neutral,
    Pointing,
    Pinch,
    OpenPalm,
    Fist,
    SwipeLeft,
    SwipeRight,
    SwipeUp,
    SwipeDown,
    TwoFingerScroll,
    TwoFingerResize,
}

/// Polar coordinates mapped to the radial dial overlay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialCoordinates {
    pub angle_rad: f32,         // Angle in radians [0, 2pi)
    pub normalized_radius: f32, // Radial distance from center [0.0, 1.0]
    pub active_sector: usize,   // Active radial sector index 0..N
    pub is_selected: bool,      // Whether selection is confirmed via pinch or dwell
}

/// Concurrent multi-hand landmark representation supporting up to 2 hands.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MultiHandLandmarks {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left_hand: Option<HandLandmarks>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right_hand: Option<HandLandmarks>,
    #[serde(default)]
    pub hands: Vec<HandLandmarks>,
    #[serde(default)]
    pub timestamp_ms: u64,
}

impl MultiHandLandmarks {
    /// Creates a multi-hand container from optional left and right hands.
    pub fn new(left_hand: Option<HandLandmarks>, right_hand: Option<HandLandmarks>) -> Self {
        let mut hands = Vec::with_capacity(2);
        if let Some(ref l) = left_hand {
            hands.push(l.clone());
        }
        if let Some(ref r) = right_hand {
            hands.push(r.clone());
        }
        Self {
            left_hand,
            right_hand,
            hands,
            timestamp_ms: 0,
        }
    }

    /// Constructs a multi-hand container from a collection of tracked hands.
    pub fn from_hands(hands: Vec<HandLandmarks>) -> Self {
        let mut left_hand = None;
        let mut right_hand = None;
        for hand in &hands {
            match hand.handedness {
                Handedness::Left => left_hand = Some(hand.clone()),
                Handedness::Right => right_hand = Some(hand.clone()),
            }
        }
        Self {
            left_hand,
            right_hand,
            hands,
            timestamp_ms: 0,
        }
    }

    /// Alias for `from_hands` to construct from a collection of hands.
    pub fn with_hands(hands: Vec<HandLandmarks>) -> Self {
        Self::from_hands(hands)
    }

    /// Number of distinct hands detected (0, 1, or 2).
    #[inline]
    pub fn hand_count(&self) -> usize {
        let mut count = 0;
        if self.left_hand.is_some() {
            count += 1;
        }
        if self.right_hand.is_some() {
            count += 1;
        }
        count.max(self.hands.len())
    }

    /// Total landmarks across all active hands (0, 21, or 42).
    #[inline]
    pub fn total_landmarks_count(&self) -> usize {
        let mut count = 0;
        if let Some(ref l) = self.left_hand {
            count += l.landmarks.len();
        }
        if let Some(ref r) = self.right_hand {
            count += r.landmarks.len();
        }
        if count == 0 {
            for h in &self.hands {
                count += h.landmarks.len();
            }
        }
        count
    }

    /// Returns true if no hands are detected in the frame.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.hand_count() == 0
    }

    /// Returns the primary active hand (Right hand preferred, Left hand fallback).
    pub fn primary_hand(&self) -> Option<&HandLandmarks> {
        self.right_hand
            .as_ref()
            .or(self.left_hand.as_ref())
            .or_else(|| self.hands.first())
    }
}

/// Gesture tracking mode configuration (SingleHand vs DualHand).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GestureTrackingMode {
    SingleHand,
    #[default]
    DualHand,
}

impl std::fmt::Display for GestureTrackingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SingleHand => write!(f, "SingleHand"),
            Self::DualHand => write!(f, "DualHand"),
        }
    }
}

impl std::str::FromStr for GestureTrackingMode {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().replace(['_', '-'], "").as_str() {
            other if other == "singlehand" || other == "single" => Ok(Self::SingleHand),
            other if other == "dualhand" || other == "dual" => Ok(Self::DualHand),
            _ => Err(format!(
                "Unknown tracking mode '{}'. Expected 'SingleHand' or 'DualHand'",
                s
            )),
        }
    }
}

/// Pointing ray data snapshot for hands-free UI targeting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PointingRayData {
    pub dx: f32,
    pub dy: f32,
    pub dz: f32,
    pub pitch_rad: f32,
    pub yaw_rad: f32,
}

/// Pinch gesture state snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PinchData {
    pub is_pinching: bool,
    pub distance: f32,
}

impl Default for PinchData {
    fn default() -> Self {
        Self {
            is_pinching: false,
            distance: 1.0,
        }
    }
}

/// Two-finger gesture state snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TwoFingerData {
    pub scroll_position: f32,
    pub is_active: bool,
}

impl Default for TwoFingerData {
    fn default() -> Self {
        Self {
            scroll_position: 0.0,
            is_active: false,
        }
    }
}

/// Active gestures summary snapshot across hands.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ActiveGesturesSnapshot {
    pub pointing_ray: Option<PointingRayData>,
    pub pinch: PinchData,
    pub velocity_swipe: Option<String>,
    pub two_finger: TwoFingerData,
}

