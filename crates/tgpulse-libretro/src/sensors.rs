//! Libretro-owned P1 motion acquisition. Host timing never enters save states.
use crate::{ffi, motion::{MotionSample, OrientationTracker, SensorKind}, mvd};
use std::time::Instant;
use crate::sensor_trace::{Trace, Sample};

const RATE: u32 = 120;
const PERIOD_NS: u64 = 1_000_000_000 / RATE as u64;
const GAP_NS: u64 = 250_000_000;

pub struct Sensors {
    environment: Option<ffi::Environment>,
    trace: Trace,
    trace_event: &'static str,
    interface: ffi::SensorInterface,
    requested: bool,
    gyro: bool,
    accel: bool,
    tracker: OrientationTracker,
    epoch: Instant,
    last_poll: Option<u64>,
}

impl Sensors {
    pub fn new(environment: Option<ffi::Environment>) -> Self {
        let mut interface = ffi::SensorInterface::default();
        if let Some(env) = environment {
            if !unsafe { env(ffi::GET_SENSOR_INTERFACE, (&mut interface as *mut ffi::SensorInterface).cast()) } {
                interface = ffi::SensorInterface::default();
            }
        }
        Self { environment, trace: Trace::default(), trace_event: "sample", interface, requested: false, gyro: false, accel: false,
            tracker: OrientationTracker::with_calibration_timing(1_000_000_000, 3_000_000_000),
            epoch: Instant::now(), last_poll: None }
    }

    pub fn stop(&mut self) {
        if let Some(notice) = self.trace.stop() { crate::notify(self.environment, &notice); }
        if let Some(set) = self.interface.set_sensor_state {
            if self.gyro { unsafe { set(0, 3, 0); } }
            if self.accel { unsafe { set(0, 1, 0); } }
        }
        self.requested = false;
        self.gyro = false;
        self.accel = false;
        self.last_poll = None;
        self.tracker.reset();
    }

    pub fn configure(&mut self, settings: mvd::Settings, pad: bool) -> Vec<String> {
        self.tracker.set_gravity_stabilization(settings.gravity);
        self.tracker.set_drift_compensation(settings.drift_compensation);
        let requested = pad && matches!(settings.mode, mvd::Mode::Auto | mvd::Mode::Sensors);
        if !requested { self.stop(); return Vec::new(); }
        if self.requested {
            return self.trace.configure(settings.sensor_diagnostics && self.gyro, self.environment).into_iter().collect();
        }
        self.requested = true;
        if self.interface.get_sensor_input.is_some() {
            if let Some(set) = self.interface.set_sensor_state {
                self.gyro = unsafe { set(0, 2, RATE) };
                if self.gyro { self.accel = unsafe { set(0, 0, RATE) }; }
            }
        }
        if !self.gyro {
            return vec!["MVD sensors unavailable; Auto uses the right stick, Sensors uses a fixed camera".into()];
        }
        let mut notices = vec![Self::start_notice().into()];
        if !self.accel { notices.push("MVD accelerometer unavailable; using gyroscope without gravity stabilization".into()); }
        notices.extend(self.trace.configure(settings.sensor_diagnostics, self.environment));
        notices
    }

    fn start_notice() -> &'static str { "MVD: Keep P1 controller still for 3 seconds" }

    pub fn commands(&mut self, commands: [bool; 2]) -> Vec<String> {
        let mut notices = Vec::new();
        if commands[0] {
            self.trace_event = "calibrate";
            self.tracker.reset();
            self.last_poll = None;
            notices.push(if self.gyro { Self::start_notice() } else { "MVD calibration unavailable: P1 sensors are not active" }.into());
        }
        if commands[1] {
            self.trace_event = "recenter";
            if self.tracker.angles().is_some() {
                self.tracker.recenter();
                notices.push("MVD recentered".into());
            } else { notices.push("MVD sensors not ready for recentering".into()); }
        }
        notices
    }

    pub fn poll(&mut self) -> Vec<String> {
        self.poll_at(self.epoch.elapsed().as_nanos().min(u64::MAX as u128) as u64)
    }

    fn poll_at(&mut self, timestamp_ns: u64) -> Vec<String> {
        if !self.gyro { return Vec::new(); }
        if self.last_poll.is_some_and(|last| timestamp_ns.saturating_sub(last) < PERIOD_NS) {
            return Vec::new();
        }
        let mut notices = Vec::new();
        let delta_ns = self.last_poll.map_or(0, |last| timestamp_ns.saturating_sub(last));
        if self.last_poll.is_some_and(|last| timestamp_ns.saturating_sub(last) > GAP_NS) {
            self.tracker.reset();
            self.trace_event = "sampling_gap";
            notices.push(Self::start_notice().into());
        }
        self.last_poll = Some(timestamp_ns);
        let get = self.interface.get_sensor_input.unwrap();
        let raw_gyro = unsafe { [get(0, 3), get(0, 4), get(0, 5)] };
        let gyro = raw_gyro;
        // Invert RetroArch SDL2's documented device-to-Libretro normalization.
        let gyro = [gyro[0], gyro[2], -gyro[1]];
        if !gyro.iter().all(|v| v.is_finite()) {
            self.stop();
            // Avoid repeatedly restarting a failing frontend every frame.
            self.requested = true;
            return vec!["MVD sensor input invalid; tracking stopped".into()];
        }
        let before = self.tracker.calibration_result();
        let raw_accel = if self.accel {
            let a = unsafe { [get(0, 0), get(0, 1), get(0, 2)] };
            let data = [a[0], a[2], a[1]].map(|v| v * 9.80665);
            self.tracker.observe(MotionSample { kind: SensorKind::Accel, data, timestamp_ns });
            Some(a)
        } else { None };
        self.tracker.observe(MotionSample { kind: SensorKind::Gyro, data: gyro, timestamp_ns });
        notices.extend(self.trace.record(Sample { timestamp_ns, delta_ns, event: self.trace_event,
            gyro: raw_gyro, accel: raw_accel, diagnostics: self.tracker.diagnostics(), angles: self.tracker.angles() }));
        self.trace_event = "sample";
        if before.is_none() {
            match self.tracker.calibration_result() {
                Some(Ok(_)) => notices.push("MVD calibration complete".into()),
                Some(Err(reason)) => notices.push(format!("MVD calibration rejected: {reason}; press MVD Calibrate to retry")),
                None => {},
            }
        }
        notices
    }

    pub fn angles(&self) -> Option<[f64; 3]> { self.tracker.angles() }
}

impl Drop for Sensors { fn drop(&mut self) { self.stop(); } }

#[cfg(test)]
mod tests {
    use super::*;
    unsafe extern "C" fn stationary(_: u32, id: u32) -> f32 { if id == 2 { 1.0 } else { 0.0 } }
    fn ready() -> Sensors {
        let mut s = Sensors::new(None);
        s.interface.get_sensor_input = Some(stationary);
        s.gyro = true; s.accel = true; s.requested = true;
        s
    }
    #[test]
    fn three_seconds_preserve_threshold_and_reject_under_sampling() {
        let mut s = ready();
        for i in 0..180 { assert!(s.poll_at(i * 16_666_667).is_empty()); }
        assert_eq!(s.poll_at(3_000_000_100), ["MVD calibration complete"]);
        assert!(s.angles().is_some());
        assert_eq!(s.commands([false, true]), ["MVD recentered"]);
        assert_eq!(s.commands([true, false]), [Sensors::start_notice()]);
        assert!(s.angles().is_none());
        // Reset retains the three-second policy.
        for i in 0..181 { s.poll_at(4_000_000_000 + i * 16_666_667); }
        assert!(s.angles().is_some());
        let mut sparse = ready();
        for i in 0..30 { sparse.poll_at(i * 100_000_000); }
        assert!(sparse.poll_at(3_000_000_000)[0].contains("need 100"));
        assert!(sparse.angles().is_none());
    }
    #[test]
    fn gap_and_disabled_mode_discard_host_orientation() {
        let mut s = ready();
        for i in 0..181 { s.poll_at(i * 16_666_667); }
        assert!(s.angles().is_some());
        assert_eq!(s.poll_at(4_000_000_000), [Sensors::start_notice()]);
        assert!(s.angles().is_none());
        s.configure(mvd::Settings { mode: mvd::Mode::Off, ..Default::default() }, true);
        assert!(!s.gyro);
        assert!(s.poll_at(5_000_000_000).is_empty());
    }

    #[test]
    fn drift_strength_updates_live_without_restarting_calibration() {
        let mut s = ready();
        for i in 0..181 { s.poll_at(i * 16_666_667); }
        let angles = s.angles().unwrap();
        for strength in [10, 50, 100, 0] {
            assert!(s.configure(mvd::Settings { drift_compensation: strength, ..Default::default() }, true).is_empty());
            assert_eq!(s.angles().unwrap(), angles);
        }
        assert_eq!(s.commands([true, false]), [Sensors::start_notice()]);
        assert!(s.angles().is_none());
    }

    #[test]
    fn frontend_axis_mapping_negotiation_and_shutdown() {
        use std::sync::{Mutex, atomic::{AtomicU32, Ordering}};
        static CALLS: Mutex<Vec<(u32,u32,u32)>> = Mutex::new(Vec::new());
        static AXIS: AtomicU32 = AtomicU32::new(9);
        unsafe extern "C" fn get(_: u32, id: u32) -> f32 {
            if id == AXIS.load(Ordering::Relaxed) { 0.3 } else { 0.0 }
        }
        unsafe extern "C" fn set(p: u32, action: u32, rate: u32) -> bool {
            CALLS.lock().unwrap().push((p,action,rate));
            action != 0 // Gyroscope-only frontend is supported.
        }
        unsafe extern "C" fn env(command: u32, data: *mut std::ffi::c_void) -> bool {
            assert_eq!(command, ffi::GET_SENSOR_INTERFACE);
            unsafe { *data.cast::<ffi::SensorInterface>() = ffi::SensorInterface {
                set_sensor_state: Some(set), get_sensor_input: Some(get) }; }
            true
        }
        for (id, channel, positive) in [(3,2,false),(4,1,false),(5,0,false)] {
            AXIS.store(9, Ordering::Relaxed);
            let mut s = Sensors::new(Some(env));
            assert_eq!(s.configure(mvd::Settings::default(), true).len(), 2);
            assert!(s.configure(mvd::Settings::default(), true).is_empty());
            for i in 0..181 { s.poll_at(i * 16_666_667); }
            AXIS.store(id, Ordering::Relaxed);
            for i in 181..241 { s.poll_at(i * 16_666_667); }
            let pose = mvd::sensor_pose(s.angles().unwrap());
            let neutral = tgpulse_core::model1io2::HmdPose::default();
            let delta = i32::from(pose.orientation[channel]) - i32::from(neutral.orientation[channel]);
            // Roll wraps at the native +180-degree neutral offset.
            if channel == 1 { assert!(delta < 0); }
            else { assert_eq!(delta > 0, positive); }
            s.stop();
            assert!(s.angles().is_none());
        }
        let calls = CALLS.lock().unwrap();
        assert_eq!(&*calls, &[(0,2,120),(0,0,120),(0,3,0)].repeat(3));
    }
}
