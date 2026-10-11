//! Real-Time CPU Benchmark Runner and Synthetic Video Stream Generator.

use crate::engine::HandLandmarkEngine;
use crate::error::{GestureError, Result};
use crate::frame::{FrameBuffer, PixelFormat};
use airctrl_core::types::gesture::Landmark3D;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;
use std::time::Instant;

/// Hand motion trajectory pattern for synthetic video stream generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandPath {
    /// Hand rotates in a circle around the dial center (radius 0.35).
    CircularOrbit,
    /// Hand moves horizontally across the frame (x: 0.1 -> 0.9).
    LinearSweep,
    /// Hand moves radially in and out of the center deadzone (radius 0.05 -> 0.40).
    RadialPulse,
    /// Hand maintains a static pose at a fixed position.
    StaticPose,
    /// Hand fluctuates rapidly near sector boundary to stress hysteresis.
    BoundaryFlutter,
}

/// Deterministic synthetic video stream generator for reproducible benchmarking.
pub struct SyntheticVideoStream {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub path: HandPath,
    pub total_frames: usize,
    pub current_index: usize,
}

impl SyntheticVideoStream {
    pub fn new(width: u32, height: u32, fps: f64, path: HandPath, total_frames: usize) -> Self {
        Self {
            width,
            height,
            fps,
            path,
            total_frames,
            current_index: 0,
        }
    }

    /// Generates next synthetic frame and expected ground-truth landmark.
    pub fn next_frame(&mut self) -> Option<(FrameBuffer, Landmark3D)> {
        if self.current_index >= self.total_frames {
            return None;
        }

        let idx = self.current_index;
        self.current_index += 1;

        let t = idx as f32 / self.total_frames as f32;
        let landmark = match self.path {
            HandPath::CircularOrbit => {
                let angle = t * 2.0 * PI * 2.0; // 2 complete orbits
                Landmark3D {
                    x: 0.5 + 0.35 * angle.cos(),
                    y: 0.5 + 0.35 * angle.sin(),
                    z: -0.1,
                }
            }
            HandPath::LinearSweep => Landmark3D {
                x: 0.1 + 0.8 * t,
                y: 0.5,
                z: -0.1,
            },
            HandPath::RadialPulse => {
                let pulse_r = 0.05 + 0.35 * (t * PI * 4.0).sin().abs();
                Landmark3D {
                    x: 0.5 + pulse_r,
                    y: 0.5,
                    z: -0.1,
                }
            }
            HandPath::StaticPose => Landmark3D {
                x: 0.5,
                y: 0.2,
                z: -0.1,
            },
            HandPath::BoundaryFlutter => {
                // Fluctuates around 60 degrees (PI / 3) +/- 0.02 radians
                let jitter = 0.02 * (idx as f32 * 1.5).sin();
                let angle = (PI / 3.0) + jitter;
                Landmark3D {
                    x: 0.5 + 0.35 * angle.cos(),
                    y: 0.5 + 0.35 * angle.sin(),
                    z: -0.1,
                }
            }
        };

        let mut frame = FrameBuffer::allocate(self.width, self.height, PixelFormat::Rgb8);
        // Set marker for pointing pose
        frame.set_pixel(0, 0, 255, 0, 0);

        Some((frame, landmark))
    }

    pub fn reset(&mut self) {
        self.current_index = 0;
    }
}

/// Latency and throughput performance budget specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkBudget {
    /// Minimum target frames per second (e.g. 30.0 or 60.0).
    pub target_fps: f64,
    /// Maximum allowed mean per-frame latency (e.g. 33.3ms for 30 FPS, 16.6ms for 60 FPS).
    pub max_mean_latency_ms: f64,
    /// Maximum allowed P95 latency (e.g. 35.0ms).
    pub max_p95_latency_ms: f64,
    /// Maximum allowed P99 latency (e.g. 40.0ms).
    pub max_p99_latency_ms: f64,
}

impl BenchmarkBudget {
    pub fn fps_30() -> Self {
        Self {
            target_fps: 30.0,
            max_mean_latency_ms: 33.33,
            max_p95_latency_ms: 35.0,
            max_p99_latency_ms: 40.0,
        }
    }

    pub fn fps_60() -> Self {
        Self {
            target_fps: 60.0,
            max_mean_latency_ms: 16.67,
            max_p95_latency_ms: 20.0,
            max_p99_latency_ms: 25.0,
        }
    }
}

/// Comprehensive benchmark report with statistical distribution of per-frame latency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub total_frames: usize,
    pub total_duration_ms: f64,
    pub effective_fps: f64,
    pub min_latency_ms: f64,
    pub max_latency_ms: f64,
    pub mean_latency_ms: f64,
    pub std_dev_latency_ms: f64,
    pub p50_latency_ms: f64,
    pub p90_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub p99_9_latency_ms: f64,
    pub satisfies_30_fps: bool,
    pub satisfies_60_fps: bool,
    pub meets_budget: bool,
}

/// Programmatic benchmark runner measuring CPU pipeline processing latency.
#[derive(Debug, Clone)]
pub struct BenchmarkRunner {
    pub warmup_frames: usize,
    pub budget: BenchmarkBudget,
}

impl BenchmarkRunner {
    pub fn new(budget: BenchmarkBudget) -> Self {
        Self {
            warmup_frames: 10,
            budget,
        }
    }

    pub fn with_warmup(mut self, warmup_frames: usize) -> Self {
        self.warmup_frames = warmup_frames;
        self
    }

    /// Executes latency profiling against an inference engine on a synthetic video stream.
    pub fn run(
        &self,
        engine: &dyn HandLandmarkEngine,
        stream: &mut SyntheticVideoStream,
    ) -> Result<BenchmarkReport> {
        stream.reset();

        // 1. Warmup phase
        for _ in 0..self.warmup_frames {
            if let Some((frame, _)) = stream.next_frame() {
                let _ = engine.process_frame(&frame.as_ref());
            }
        }
        stream.reset();

        // 2. Measurement phase
        let mut latencies_us: Vec<f64> = Vec::with_capacity(stream.total_frames);
        let start_total = Instant::now();

        while let Some((frame, _)) = stream.next_frame() {
            let start_frame = Instant::now();
            let _ = engine.process_frame(&frame.as_ref());
            let elapsed_us = start_frame.elapsed().as_micros() as f64;
            latencies_us.push(elapsed_us);
        }

        let total_duration_ms = start_total.elapsed().as_secs_f64() * 1000.0;
        let frame_count = latencies_us.len();

        if frame_count == 0 {
            return Err(GestureError::InferenceError(
                "Benchmark stream yielded zero frames".into(),
            ));
        }

        // 3. Statistical Analysis
        latencies_us.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let min_latency_ms = latencies_us[0] / 1000.0;
        let max_latency_ms = latencies_us[frame_count - 1] / 1000.0;
        let sum_us: f64 = latencies_us.iter().sum();
        let mean_us = sum_us / (frame_count as f64);
        let mean_latency_ms = mean_us / 1000.0;

        let variance_us = latencies_us
            .iter()
            .map(|&x| (x - mean_us).powi(2))
            .sum::<f64>()
            / (frame_count as f64);
        let std_dev_latency_ms = variance_us.sqrt() / 1000.0;

        let percentile = |pct: f64| -> f64 {
            let rank = ((pct / 100.0) * (frame_count - 1) as f64).round() as usize;
            latencies_us[rank.min(frame_count - 1)] / 1000.0
        };

        let p50_latency_ms = percentile(50.0);
        let p90_latency_ms = percentile(90.0);
        let p95_latency_ms = percentile(95.0);
        let p99_latency_ms = percentile(99.0);
        let p99_9_latency_ms = percentile(99.9);

        let sum_sec = sum_us / 1_000_000.0;
        let effective_fps = if sum_sec > 0.0 {
            (frame_count as f64) / sum_sec
        } else {
            100_000.0
        };

        let satisfies_30_fps = mean_latency_ms <= 33.33 && effective_fps >= 30.0;
        let satisfies_60_fps = mean_latency_ms <= 16.67 && effective_fps >= 60.0;

        let meets_budget = effective_fps >= self.budget.target_fps
            && mean_latency_ms <= self.budget.max_mean_latency_ms
            && p95_latency_ms <= self.budget.max_p95_latency_ms
            && p99_latency_ms <= self.budget.max_p99_latency_ms;

        Ok(BenchmarkReport {
            total_frames: frame_count,
            total_duration_ms,
            effective_fps,
            min_latency_ms,
            max_latency_ms,
            mean_latency_ms,
            std_dev_latency_ms,
            p50_latency_ms,
            p90_latency_ms,
            p95_latency_ms,
            p99_latency_ms,
            p99_9_latency_ms,
            satisfies_30_fps,
            satisfies_60_fps,
            meets_budget,
        })
    }
}
