//! Host motion samples and stationary gyro calibration. No SDL handles,
//! emulation state, filesystem or wall clock: other frontends can supply the
//! same units and explicit sample timestamps without depending on SDL.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SensorKind {
    Gyro,
    Accel,
}

#[derive(Clone, Copy, Debug)]
pub struct MotionSample {
    pub kind: SensorKind,
    /// Gyro rad/s; accelerometer m/s² including gravity. SDL device axes,
    /// not yet mapped to NetMerc angles.
    pub data: [f32; 3],
    /// Monotonic host event timestamp (ns), not emulated time or a pose angle.
    pub timestamp_ns: u64,
}

#[derive(Default)]
pub struct GyroCalibration {
    count: u32,
    mean: [f64; 3],
    m2: [f64; 3],
    last_timestamp: Option<u64>,
}

pub const MIN_CALIBRATION_SAMPLES: u32 = 100;
pub const MAX_GYRO_MEAN: f64 = 0.1;
pub const MAX_GYRO_DEVIATION: f64 = 0.02;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CalibrationRejection {
    TooFewSamples(u32),
    Mean { axis: usize, value: f64 },
    Deviation { axis: usize, value: f64 },
}

impl std::fmt::Display for CalibrationRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::TooFewSamples(count) => write!(
                f,
                "only {count} distinct gyro samples; need {MIN_CALIBRATION_SAMPLES}"
            ),
            Self::Mean { axis, value } => write!(
                f,
                "axis {} mean {value:.6} rad/s exceeds ±{MAX_GYRO_MEAN:.3}",
                ["X", "Y", "Z"][axis]
            ),
            Self::Deviation { axis, value } => write!(
                f,
                "axis {} standard deviation {value:.6} rad/s exceeds {MAX_GYRO_DEVIATION:.3}",
                ["X", "Y", "Z"][axis]
            ),
        }
    }
}

impl GyroCalibration {
    pub fn observe(&mut self, sample: MotionSample) {
        if sample.kind != SensorKind::Gyro
            || !sample.data.iter().all(|v| v.is_finite())
            || self
                .last_timestamp
                .is_some_and(|last| sample.timestamp_ns <= last)
        {
            return;
        }
        self.last_timestamp = Some(sample.timestamp_ns);
        self.count += 1;
        for (axis, value) in sample.data.into_iter().enumerate() {
            let delta = f64::from(value) - self.mean[axis];
            self.mean[axis] += delta / f64::from(self.count);
            self.m2[axis] += delta * (f64::from(value) - self.mean[axis]);
        }
    }
    pub fn samples(&self) -> u32 {
        self.count
    }
    pub fn bias(&self) -> Option<[f32; 3]> {
        self.result().ok()
    }

    pub fn mean(&self) -> Option<[f64; 3]> {
        (self.count > 0).then_some(self.mean)
    }

    pub fn standard_deviation(&self) -> Option<[f64; 3]> {
        (self.count >= 2).then(|| self.m2.map(|m2| (m2 / f64::from(self.count - 1)).sqrt()))
    }

    pub fn result(&self) -> Result<[f32; 3], CalibrationRejection> {
        // At least 100 distinct samples; reject movement, not pretend a moving
        // controller has a valid stationary bias. Limits are diagnostic policy.
        if self.count < MIN_CALIBRATION_SAMPLES {
            return Err(CalibrationRejection::TooFewSamples(self.count));
        }
        let deviation = self.standard_deviation().unwrap();
        for (axis, value) in deviation.into_iter().enumerate() {
            if self.mean[axis].abs() > MAX_GYRO_MEAN {
                return Err(CalibrationRejection::Mean {
                    axis,
                    value: self.mean[axis],
                });
            }
            if value > MAX_GYRO_DEVIATION {
                return Err(CalibrationRejection::Deviation { axis, value });
            }
        }
        Ok(self.mean.map(|v| v as f32))
    }
}

// Mahony-style proportional gravity feedback (Ki = 0); existing stationary
// gyro calibration owns the bias. These are host-estimator policy, not hardware.
const GRAVITY: f64 = 9.80665;
const GRAVITY_TOLERANCE: f64 = 0.15;
const GRAVITY_GAIN: f64 = 1.0;
const ACCEL_MAX_AGE_NS: u64 = 100_000_000;

// Runtime offset learning follows the stationary-period/filter approach
// documented by xioTechnologies Fusion. Use explicit dt, a conservative
// 0.5 deg/s gate and optional gravity consistency rather than its 3 deg/s gate.
#[derive(Default)]
struct AdaptiveGyroBias {
    strength: u32,
    offset: [f64; 3],
    stationary_seconds: f64,
    gravity_anchor: Option<[f64; 3]>,
}
impl AdaptiveGyroBias {
    fn set_strength(&mut self, percent: u32) {
        let percent = percent.min(100);
        if percent != self.strength {
            self.strength = percent;
            self.stationary_seconds = 0.0;
            self.gravity_anchor = None;
            if percent == 0 { self.offset = [0.0; 3]; }
        }
    }
    fn correct(&mut self, omega: [f64; 3], dt: f64,
               gravity: Option<[f64; 3]>, accel_valid: bool) -> [f64; 3] {
        if self.strength == 0 { return omega; }
        let corrected = std::array::from_fn(|i| omega[i] - self.offset[i]);
        let steady_gyro = corrected.iter().map(|v| v * v).sum::<f64>()
            <= 0.5_f64.to_radians().powi(2);
        let steady_gravity = match (self.gravity_anchor, gravity) {
            (Some(anchor), Some(current)) => anchor.iter().zip(current)
                .map(|(a, b)| a * b).sum::<f64>() >= 1.0_f64.to_radians().cos(),
            _ => true,
        };
        if !steady_gyro || !steady_gravity || !accel_valid {
            self.stationary_seconds = 0.0;
            self.gravity_anchor = None;
            return corrected;
        }
        if self.gravity_anchor.is_none() { self.gravity_anchor = gravity; }
        self.stationary_seconds += dt;
        if self.stationary_seconds >= 3.0 {
            // 100%: cutoff 0.02 Hz, time constant about 8 seconds. Percent
            // scales learning speed, not motion sensitivity or the gate.
            let alpha = 1.0 - (-2.0 * std::f64::consts::PI * 0.02
                * f64::from(self.strength) / 100.0 * dt).exp();
            for (offset, residual) in self.offset.iter_mut().zip(corrected) {
                *offset += alpha * residual;
            }
        }
        corrected
    }
}

fn cross([a, b, c]: [f64; 3], [x, y, z]: [f64; 3]) -> [f64; 3] {
    [b * z - c * y, c * x - a * z, a * y - b * x]
}

fn rotate([w, x, y, z]: [f64; 4], vector: [f64; 3]) -> [f64; 3] {
    let twice = cross([x, y, z], vector).map(|v| 2.0 * v);
    let second = cross([x, y, z], twice);
    std::array::from_fn(|i| vector[i] + w * twice[i] + second[i])
}

fn normalized(vector: [f64; 3]) -> Option<[f64; 3]> {
    let norm = vector.iter().map(|v| v * v).sum::<f64>().sqrt();
    (norm.is_finite() && norm > 1e-6).then(|| vector.map(|v| v / norm))
}

/// Relative orientation from explicit IMU samples. No device, wall clock,
/// filesystem or emulated state: another frontend can drive this same helper.
pub struct OrientationTracker {
    warmup_ns: u64,
    total_ns: u64,
    first: Option<u64>,
    last: Option<u64>,
    calibration: GyroCalibration,
    result: Option<Result<[f32; 3], CalibrationRejection>>,
    /// Unit quaternion, w/x/y/z, body-to-initial-controller orientation.
    rotation: [f64; 4],
    gravity_enabled: bool,
    accel_timestamp: Option<u64>,
    accel: Option<[f64; 3]>,
    gravity_sum: [f64; 3],
    gravity_reference: Option<[f64; 3]>,
    drift: AdaptiveGyroBias,
    corrected_rate: Option<[f64; 3]>,
    integrated_rate: Option<[f64; 3]>,
}

/// Latest host estimator diagnostics, never part of emulated machine state.
pub struct MotionDiagnostics {
    pub phase: &'static str,
    pub calibration_samples: u32,
    pub calibration_mean: Option<[f64; 3]>,
    pub calibration_bias: Option<[f64; 3]>,
    pub calibration_deviation: Option<[f64; 3]>,
    pub learned_bias: [f64; 3],
    pub stationary_seconds: f64,
    pub corrected_rate: Option<[f64; 3]>,
    pub integrated_rate: Option<[f64; 3]>,
    pub drift_strength: u32,
    pub gravity_enabled: bool,
}
impl Default for OrientationTracker {
    fn default() -> Self {
        Self {
            warmup_ns: 2_000_000_000,
            total_ns: 5_000_000_000,
            first: None,
            last: None,
            calibration: Default::default(),
            result: None,
            rotation: [1.0, 0.0, 0.0, 0.0],
            gravity_enabled: false,
            accel_timestamp: None,
            accel: None,
            gravity_sum: [0.0; 3],
            gravity_reference: None,
            drift: Default::default(),
            corrected_rate: None,
            integrated_rate: None,
        }
    }
}
impl OrientationTracker {
    /// Frontend policy; default timing remains the standalone five seconds.
    pub fn with_calibration_timing(warmup_ns: u64, total_ns: u64) -> Self {
        assert!(total_ns > warmup_ns);
        Self { warmup_ns, total_ns, ..Self::default() }
    }
    pub fn set_gravity_stabilization(&mut self, enabled: bool) {
        self.gravity_enabled = enabled;
    }
    /// Optional host-estimator policy. Zero retains the upstream fixed bias.
    /// Changing strength never recenters or restarts the initial calibration.
    pub fn set_drift_compensation(&mut self, percent: u32) {
        self.drift.set_strength(percent);
    }
    pub fn diagnostics(&self) -> MotionDiagnostics {
        MotionDiagnostics {
            phase: match self.result { None => "calibrating", Some(Ok(_)) => "ready", Some(Err(_)) => "rejected" },
            calibration_samples: self.calibration.samples(),
            calibration_mean: self.calibration.mean(),
            calibration_bias: self.result.and_then(Result::ok).map(|bias| bias.map(f64::from)),
            calibration_deviation: self.calibration.standard_deviation(),
            learned_bias: self.drift.offset,
            stationary_seconds: self.drift.stationary_seconds,
            corrected_rate: self.corrected_rate,
            integrated_rate: self.integrated_rate,
            drift_strength: self.drift.strength,
            gravity_enabled: self.gravity_enabled,
        }
    }
    pub fn reset(&mut self) {
        let enabled = self.gravity_enabled;
        let drift = self.drift.strength;
        *self = Self::with_calibration_timing(self.warmup_ns, self.total_ns);
        self.gravity_enabled = enabled;
        self.set_drift_compensation(drift);
    }
    pub fn observe(&mut self, sample: MotionSample) -> bool {
        if sample.kind == SensorKind::Accel {
            if self
                .accel_timestamp
                .is_some_and(|last| sample.timestamp_ns <= last)
            {
                return false;
            }
            self.accel_timestamp = Some(sample.timestamp_ns);
            let vector = sample.data.map(f64::from);
            let norm = vector.iter().map(|v| v * v).sum::<f64>().sqrt();
            self.accel = if norm.is_finite() && (norm / GRAVITY - 1.0).abs() <= GRAVITY_TOLERANCE {
                normalized(vector)
            } else {
                None // Do not reuse an old good sample during a detected acceleration.
            };
            if self.result.is_none()
                && self.first.is_some_and(|first| {
                    (self.warmup_ns..self.total_ns)
                        .contains(&sample.timestamp_ns.saturating_sub(first))
                })
            {
                if let Some(gravity) = self.accel {
                    for (sum, value) in self.gravity_sum.iter_mut().zip(gravity) {
                        *sum += value;
                    }
                }
            }
            // Only gyro events establish live orientation/freshness in the host.
            return false;
        }
        if sample.kind != SensorKind::Gyro
            || !sample.data.iter().all(|v| v.is_finite())
            || self.last.is_some_and(|last| sample.timestamp_ns <= last)
        {
            return false;
        }
        let dt = self
            .last
            .map_or(0.0, |last| (sample.timestamp_ns - last) as f64 / 1e9);
        // Never integrate across disconnect/pause/host scheduling gaps.
        if dt > 0.25 {
            self.reset();
        }
        let first = *self.first.get_or_insert(sample.timestamp_ns);
        self.last = Some(sample.timestamp_ns);
        let elapsed = sample.timestamp_ns - first;
        if elapsed < self.warmup_ns {
            return true;
        }
        if elapsed < self.total_ns {
            self.calibration.observe(sample);
            return true;
        }
        if self.result.is_none() {
            self.result = Some(self.calibration.result());
            self.gravity_reference = normalized(self.gravity_sum);
            return true; // Neutral is the controller orientation at acceptance.
        }
        if let Some(Ok(bias)) = self.result {
            let mut omega: [f64; 3] =
                std::array::from_fn(|axis| f64::from(sample.data[axis] - bias[axis]));
            let fresh_accel = self.accel_timestamp.is_some_and(|timestamp| {
                sample.timestamp_ns.checked_sub(timestamp)
                    .is_some_and(|age| age <= ACCEL_MAX_AGE_NS)
            });
            omega = self.drift.correct(omega, dt,
                if fresh_accel { self.accel } else { None },
                self.accel_timestamp.is_none() || (fresh_accel && self.accel.is_some()));
            self.corrected_rate = Some(omega);
            if self.gravity_enabled
                && self.accel_timestamp.is_some_and(|timestamp| {
                    sample
                        .timestamp_ns
                        .checked_sub(timestamp)
                        .is_some_and(|age| age <= ACCEL_MAX_AGE_NS)
                })
            {
                if let Some(measured) = self.accel {
                    // Late accelerometer availability establishes a reference without
                    // jumping the current pose. Calibrated neutral need not be level.
                    let reference = *self
                        .gravity_reference
                        .get_or_insert_with(|| rotate(self.rotation, measured));
                    let [w, x, y, z] = self.rotation;
                    let predicted = rotate([w, -x, -y, -z], reference);
                    let error = cross(measured, predicted);
                    for (rate, correction) in omega.iter_mut().zip(error) {
                        *rate += GRAVITY_GAIN * correction;
                    }
                }
            }
            self.integrated_rate = Some(omega);
            let speed = omega.iter().map(|v| v * v).sum::<f64>().sqrt();
            if speed > 0.0 {
                let half = speed * dt * 0.5;
                let scale = half.sin() / speed;
                let [a, b, c, d] = self.rotation;
                let [x, y, z] = omega.map(|v| v * scale);
                let w = half.cos();
                self.rotation = [
                    a * w - b * x - c * y - d * z,
                    a * x + b * w + c * z - d * y,
                    a * y - b * z + c * w + d * x,
                    a * z + b * y - c * x + d * w,
                ];
                let norm = self.rotation.iter().map(|v| v * v).sum::<f64>().sqrt();
                self.rotation = self.rotation.map(|v| v / norm);
            }
        }
        true
    }
    pub fn recenter(&mut self) {
        // Express the same physical gravity in the new neutral frame. Otherwise
        // the correction would slowly undo a user's deliberately tilted center.
        if let Some(reference) = self.gravity_reference {
            let [w, x, y, z] = self.rotation;
            self.gravity_reference = Some(rotate([w, -x, -y, -z], reference));
        }
        self.rotation = [1.0, 0.0, 0.0, 0.0];
    }
    pub fn calibration_result(&self) -> Option<Result<[f32; 3], CalibrationRejection>> {
        self.result
    }
    /// Pitch/yaw/roll, radians, Y-X-Z decomposition of the relative rotation.
    pub fn angles(&self) -> Option<[f64; 3]> {
        if !matches!(self.result, Some(Ok(_))) {
            return None;
        }
        let [w, x, y, z] = self.rotation;
        Some([
            (-2.0 * (y * z - x * w)).clamp(-1.0, 1.0).asin(),
            (2.0 * (x * z + y * w)).atan2(1.0 - 2.0 * (x * x + y * y)),
            (2.0 * (x * y + z * w)).atan2(1.0 - 2.0 * (x * x + z * z)),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptive_bias_reduces_residual_drift_with_percent_and_keeps_current_view() {
        let mut rates = Vec::new();
        for strength in [0, 10, 50, 100] {
            let mut tracker = OrientationTracker::with_calibration_timing(1_000_000_000, 3_000_000_000);
            tracker.set_drift_compensation(strength);
            for n in 0..=300 { imu(&mut tracker, n, [0.0; 3], [0.0, GRAVITY as f32, 0.0]); }
            let mut before_last_second = 0.0;
            for n in 301..=6300 {
                imu(&mut tracker, n, [0.0, 0.002, 0.0], [0.0, GRAVITY as f32, 0.0]);
                if n == 6200 { before_last_second = tracker.angles().unwrap()[1]; }
            }
            let angle = tracker.angles().unwrap()[1];
            let residual = angle - before_last_second;
            println!("drift strength={strength}% yaw={angle:.8} final_rate={residual:.8} rad/s");
            assert!(angle > 0.0); // Learning never snaps an accumulated angle to zero.
            rates.push(residual);
            let orientation = tracker.angles().unwrap();
            tracker.set_drift_compensation(50);
            assert_eq!(tracker.angles().unwrap(), orientation);
            tracker.recenter();
            assert!(tracker.angles().unwrap().iter().all(|v| v.abs() < 1e-12));
            tracker.reset();
            assert_eq!(tracker.drift.strength, 50);
            assert_eq!(tracker.drift.offset, [0.0; 3]);
            assert!(tracker.angles().is_none());
        }
        assert!((rates[0] - 0.002).abs() < 1e-8);
        assert!(rates.windows(2).all(|pair| pair[1] < pair[0]));
        assert!(rates[3] < rates[0] * 0.001);
    }

    #[test]
    fn adaptive_bias_preserves_slow_turns_above_gate_and_rejects_acceleration() {
        let mut tracker = OrientationTracker::with_calibration_timing(1_000_000_000, 3_000_000_000);
        tracker.set_drift_compensation(100);
        for n in 0..=300 { imu(&mut tracker, n, [0.0; 3], [0.0, GRAVITY as f32, 0.0]); }
        for n in 301..=3300 {
            imu(&mut tracker, n, [0.0, 1.0_f32.to_radians(), 0.0], [0.0, GRAVITY as f32, 0.0]);
        }
        assert!((tracker.angles().unwrap()[1].to_degrees() - 30.0).abs() < 1e-5);
        assert_eq!(tracker.drift.offset, [0.0; 3]);
        for n in 3301..=3900 { imu(&mut tracker, n, [0.0, 0.002, 0.0], [0.0, 20.0, 0.0]); }
        assert_eq!(tracker.drift.offset, [0.0; 3]);
        for n in 3901..=4100 { imu(&mut tracker, n, [0.0, 0.002, 0.0], [0.0, GRAVITY as f32, 0.0]); }
        assert_eq!(tracker.drift.offset, [0.0; 3]); // Fresh dwell required after movement.
        for n in 4101..=5000 { imu(&mut tracker, n, [0.0, 0.002, 0.0], [0.0, GRAVITY as f32, 0.0]); }
        assert!(tracker.drift.offset[1] > 0.0);
        let view = tracker.angles().unwrap();
        tracker.set_drift_compensation(0);
        assert_eq!(tracker.drift.offset, [0.0; 3]);
        assert_eq!(tracker.angles().unwrap(), view);
    }

    #[test]
    fn adaptive_bias_uses_elapsed_time_and_supports_gyro_only() {
        let mut residuals = Vec::new();
        for hz in [60, 120] {
            let mut drift = AdaptiveGyroBias::default();
            drift.set_strength(100);
            let mut residual = [0.0; 3];
            for _ in 0..hz * 60 {
                residual = drift.correct([0.0, 0.002, 0.0], 1.0 / f64::from(hz), None, true);
            }
            residuals.push(residual[1]);
        }
        assert!((residuals[0] - residuals[1]).abs() < 1e-8);
        assert!(residuals[0] < 0.00001);
    }

    fn imu(tracker: &mut OrientationTracker, n: u64, gyro: [f32; 3], accel: [f32; 3]) {
        let timestamp_ns = n * 10_000_000;
        assert!(!tracker.observe(MotionSample {
            kind: SensorKind::Accel,
            data: accel,
            timestamp_ns
        }));
        tracker.observe(MotionSample {
            kind: SensorKind::Gyro,
            data: gyro,
            timestamp_ns,
        });
    }

    #[test]
    fn gravity_limits_tilt_drift_but_does_not_invent_a_yaw_reference() {
        for axis in 0..3 {
            let mut gyro = OrientationTracker::default();
            let mut fused = OrientationTracker::default();
            fused.set_gravity_stabilization(true);
            for n in 0..=500 {
                imu(&mut gyro, n, [0.0; 3], [0.0, GRAVITY as f32, 0.0]);
                imu(&mut fused, n, [0.0; 3], [0.0, GRAVITY as f32, 0.0]);
            }
            let mut bias_drift = [0.0; 3];
            bias_drift[axis] = 0.02;
            for n in 501..=3500 {
                imu(&mut gyro, n, bias_drift, [0.0, GRAVITY as f32, 0.0]);
                imu(&mut fused, n, bias_drift, [0.0, GRAVITY as f32, 0.0]);
            }
            assert!((gyro.angles().unwrap()[axis] - 0.6).abs() < 1e-6);
            let corrected = fused.angles().unwrap()[axis];
            if axis == 1 {
                assert!((corrected - 0.6).abs() < 1e-6);
            } else {
                assert!((corrected - 0.02).abs() < 0.001);
            }
        }
    }

    #[test]
    fn gravity_preserves_real_tilt_tilted_neutral_and_recenter() {
        let mut tracker = OrientationTracker::default();
        tracker.set_gravity_stabilization(true);
        let neutral = [
            0.0,
            (GRAVITY * 0.4_f64.cos()) as f32,
            (-GRAVITY * 0.4_f64.sin()) as f32,
        ];
        for n in 0..=500 {
            imu(&mut tracker, n, [0.0; 3], neutral);
        }
        for n in 501..=600 {
            let angle = (n - 500) as f64 * 0.005;
            let accel = [
                0.0,
                (GRAVITY * (0.4 + angle).cos()) as f32,
                (-GRAVITY * (0.4 + angle).sin()) as f32,
            ];
            imu(&mut tracker, n, [0.5, 0.0, 0.0], accel);
        }
        let held = [
            0.0,
            (GRAVITY * 0.9_f64.cos()) as f32,
            (-GRAVITY * 0.9_f64.sin()) as f32,
        ];
        for n in 601..=1600 {
            imu(&mut tracker, n, [0.0; 3], held);
        }
        assert!((tracker.angles().unwrap()[0] - 0.5).abs() < 1e-5);
        tracker.recenter();
        assert_eq!(tracker.angles(), Some([0.0; 3]));
        for n in 1601..=2600 {
            imu(&mut tracker, n, [0.0; 3], held);
        }
        assert!(tracker.angles().unwrap()[0].abs() < 1e-5);
        tracker.reset();
        assert!(tracker.gravity_enabled);
        assert_eq!(tracker.angles(), None);
        assert!(tracker.gravity_reference.is_none());
    }

    #[test]
    fn gravity_missing_stale_or_accelerating_falls_back_to_gyro_and_can_be_disabled_live() {
        let mut gyro = OrientationTracker::default();
        let mut fused = OrientationTracker::default();
        fused.set_gravity_stabilization(true);
        // No accelerometer: exactly the original gyro-only behavior.
        for n in 0..=600 {
            let sample = MotionSample {
                kind: SensorKind::Gyro,
                data: if n <= 500 { [0.0; 3] } else { [0.02, 0.0, 0.0] },
                timestamp_ns: n * 10_000_000,
            };
            gyro.observe(sample);
            fused.observe(sample);
        }
        assert_eq!(gyro.angles(), fused.angles());
        // Capture gravity matching the current physical tilt, then let it go stale.
        let accel = [
            0.0,
            (GRAVITY * 0.02_f64.cos()) as f32,
            (-GRAVITY * 0.02_f64.sin()) as f32,
        ];
        imu(&mut gyro, 601, [0.0; 3], accel);
        imu(&mut fused, 601, [0.0; 3], accel);
        for n in 612..=650 {
            let sample = MotionSample {
                kind: SensorKind::Gyro,
                data: [0.02, 0.0, 0.0],
                timestamp_ns: n * 10_000_000,
            };
            gyro.observe(sample);
            fused.observe(sample);
        }
        assert_eq!(gyro.angles(), fused.angles());
        for n in 651..=750 {
            imu(&mut gyro, n, [0.02, 0.0, 0.0], [20.0, 0.0, 0.0]);
            imu(&mut fused, n, [0.02, 0.0, 0.0], [20.0, 0.0, 0.0]);
        }
        assert_eq!(gyro.angles(), fused.angles());
        fused.set_gravity_stabilization(false);
        for n in 751..=850 {
            imu(&mut gyro, n, [0.02, 0.0, 0.0], [0.0, GRAVITY as f32, 0.0]);
            imu(&mut fused, n, [0.02, 0.0, 0.0], [0.0, GRAVITY as f32, 0.0]);
        }
        assert_eq!(gyro.angles(), fused.angles());
    }
    #[test]
    fn stationary_bias_requires_distinct_finite_samples_and_rejects_motion() {
        let mut stable = GyroCalibration::default();
        let mut moving = GyroCalibration::default();
        assert_eq!(stable.result(), Err(CalibrationRejection::TooFewSamples(0)));
        assert_eq!(stable.mean(), None);
        assert_eq!(stable.standard_deviation(), None);
        for i in 0..120 {
            let sample = MotionSample {
                kind: SensorKind::Gyro,
                data: [0.01, -0.02, 0.005],
                timestamp_ns: i,
            };
            stable.observe(sample);
            stable.observe(sample); // duplicate timestamp
            moving.observe(MotionSample {
                data: [if i % 2 == 0 { 0.3 } else { -0.3 }, 0.0, 0.0],
                ..sample
            });
        }
        assert_eq!(stable.samples(), 120);
        assert_eq!(stable.bias(), Some([0.01, -0.02, 0.005]));
        assert_eq!(stable.standard_deviation(), Some([0.0; 3]));
        assert_eq!(moving.bias(), None);
        assert!(matches!(
            moving.result(),
            Err(CalibrationRejection::Deviation { axis: 0, .. })
        ));
        let mut turning = GyroCalibration::default();
        for timestamp_ns in 0..120 {
            turning.observe(MotionSample {
                kind: SensorKind::Gyro,
                data: [0.0, 0.2, 0.0],
                timestamp_ns,
            });
        }
        assert!(matches!(
            turning.result(),
            Err(CalibrationRejection::Mean { axis: 1, .. })
        ));
    }

    #[test]
    fn orientation_calibrates_integrates_each_axis_and_recenters_without_recalibrating() {
        for axis in 0..3 {
            let mut tracker = OrientationTracker::default();
            for n in 0..=500 {
                tracker.observe(MotionSample {
                    kind: SensorKind::Gyro,
                    data: [0.0; 3],
                    timestamp_ns: n * 10_000_000,
                });
            }
            assert_eq!(tracker.angles(), Some([0.0; 3]));
            for n in 501..=600 {
                let mut data = [0.0; 3];
                data[axis] = 0.5;
                tracker.observe(MotionSample {
                    kind: SensorKind::Gyro,
                    data,
                    timestamp_ns: n * 10_000_000,
                });
            }
            let angles = tracker.angles().unwrap();
            for (index, value) in angles.into_iter().enumerate() {
                assert!((value - if index == axis { 0.5 } else { 0.0 }).abs() < 1e-6);
            }
            tracker.recenter();
            assert_eq!(tracker.angles(), Some([0.0; 3]));
            assert!(!tracker.observe(MotionSample {
                kind: SensorKind::Gyro,
                data: [1.0; 3],
                timestamp_ns: 6_000_000_000
            }));
            tracker.observe(MotionSample {
                kind: SensorKind::Gyro,
                data: [1.0; 3],
                timestamp_ns: 7_000_000_000,
            });
            assert_eq!(tracker.angles(), None); // Gap starts fresh calibration, not a jump.
        }
    }
}
