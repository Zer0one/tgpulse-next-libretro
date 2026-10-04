//! Optional host-only sensor evidence. Never included in machine state or NVRAM.
use crate::{ffi, motion::MotionDiagnostics, mvd};
use std::{ffi::CStr, fs::{File, OpenOptions}, io::{BufWriter, Write}, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};

const MAX_ROWS: u64 = 120 * 60 * 30;
const MAX_DURATION_NS: u64 = 30 * 60 * 1_000_000_000;
const HEADER: &str = "timestamp_ns,delta_ns,event,phase,gyro_available,accel_available,gravity_enabled,drift_strength_pct,calibration_samples,raw_gyro_x,raw_gyro_y,raw_gyro_z,gyro_x_rad_s,gyro_y_rad_s,gyro_z_rad_s,raw_accel_x,raw_accel_y,raw_accel_z,accel_x_m_s2,accel_y_m_s2,accel_z_m_s2,calibration_mean_x,calibration_mean_y,calibration_mean_z,calibration_bias_x,calibration_bias_y,calibration_bias_z,learned_bias_x,learned_bias_y,learned_bias_z,calibration_deviation_x,calibration_deviation_y,calibration_deviation_z,stationary_seconds,corrected_x_rad_s,corrected_y_rad_s,corrected_z_rad_s,integrated_x_rad_s,integrated_y_rad_s,integrated_z_rad_s,pitch_rad,yaw_rad,roll_rad,pose_x,pose_y,pose_z,pose_orientation_0,pose_orientation_1,pose_orientation_2";

#[derive(Default)]
pub struct Trace {
    requested: bool,
    recording: Option<Recording>,
}

struct Recording {
    writer: BufWriter<File>,
    path: PathBuf,
    rows: u64,
    started: Option<u64>,
    last_flush: u64,
}

pub struct Sample<'a> {
    pub timestamp_ns: u64,
    pub delta_ns: u64,
    pub event: &'a str,
    pub gyro: [f32; 3],
    pub accel: Option<[f32; 3]>,
    pub diagnostics: MotionDiagnostics,
    pub angles: Option<[f64; 3]>,
}

impl Recording {
    fn open(directory: &Path) -> std::io::Result<Self> {
        std::fs::create_dir_all(directory)?;
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        let path = directory.join(format!("netmerc_mvd_{stamp}_{}.csv", std::process::id()));
        let file = OpenOptions::new().write(true).create_new(true).open(&path)?;
        let mut writer = BufWriter::new(file);
        writeln!(writer, "# TGPulse-Next MVD Sensor Trace,format=1")?;
        writeln!(writer, "# Source=Libretro P1 callbacks; controller model and hardware sample timestamps unavailable")?;
        writeln!(writer, "# raw gyro=rad/s; raw accel=g; bias/deviation=rad/s; timestamps=host monotonic session ns")?;
        writeln!(writer, "{HEADER}")?;
        writer.flush()?;
        Ok(Self { writer, path, rows: 0, started: None, last_flush: 0 })
    }

    fn write(&mut self, sample: Sample<'_>) -> std::io::Result<()> {
        let d = sample.diagnostics;
        let mut columns = vec![sample.timestamp_ns.to_string(), sample.delta_ns.to_string(),
            sample.event.into(), d.phase.into(), "true".into(), sample.accel.is_some().to_string(),
            d.gravity_enabled.to_string(), d.drift_strength.to_string(), d.calibration_samples.to_string()];
        let mut vector = |value: Option<[f64; 3]>| {
            columns.extend(value.map(|v| v.map(|x| x.to_string())).unwrap_or_default());
        };
        vector(Some(sample.gyro.map(f64::from)));
        vector(Some([sample.gyro[0], sample.gyro[2], -sample.gyro[1]].map(f64::from)));
        vector(sample.accel.map(|a| a.map(f64::from)));
        vector(sample.accel.map(|a| [a[0], a[2], a[1]].map(|x| f64::from(x * 9.80665))));
        vector(d.calibration_mean);
        vector(d.calibration_bias);
        vector(Some(d.learned_bias));
        vector(d.calibration_deviation);
        columns.push(d.stationary_seconds.to_string());
        for value in [d.corrected_rate, d.integrated_rate, sample.angles] {
            columns.extend(value.map(|v| v.map(|x| x.to_string())).unwrap_or_default());
        }
        let pose = sample.angles.map(mvd::sensor_pose).unwrap_or_default();
        columns.extend(pose.position.into_iter().chain(pose.orientation).map(|x| x.to_string()));
        writeln!(self.writer, "{}", columns.join(","))?;
        self.rows += 1;
        if sample.timestamp_ns.saturating_sub(self.last_flush) >= 1_000_000_000 {
            self.writer.flush()?;
            self.last_flush = sample.timestamp_ns;
        }
        Ok(())
    }
}

impl Trace {
    pub fn configure(&mut self, enabled: bool, environment: Option<ffi::Environment>) -> Option<String> {
        if !enabled {
            self.requested = false;
            return self.close();
        }
        if self.requested { return None; }
        self.requested = true;
        let mut directory: *const std::ffi::c_char = std::ptr::null();
        let available = environment.is_some_and(|env| unsafe {
            env(ffi::GET_SAVE_DIRECTORY, (&mut directory as *mut *const std::ffi::c_char).cast())
        });
        if !available || directory.is_null() {
            return Some("MVD sensor diagnostics unavailable: frontend save directory missing".into());
        }
        let directory = unsafe { CStr::from_ptr(directory) }.to_string_lossy();
        if directory.is_empty() {
            return Some("MVD sensor diagnostics unavailable: frontend save directory empty".into());
        }
        match Recording::open(&Path::new(directory.as_ref()).join("tgpulse-next/diagnostics")) {
            Ok(recording) => {
                let notice = format!("MVD sensor diagnostics recording: {}", recording.path.display());
                self.recording = Some(recording);
                Some(notice)
            },
            Err(error) => Some(format!("MVD sensor diagnostics could not start: {error}")),
        }
    }

    pub fn record(&mut self, sample: Sample<'_>) -> Option<String> {
        let recording = self.recording.as_mut()?;
        let started = *recording.started.get_or_insert(sample.timestamp_ns);
        if recording.rows >= MAX_ROWS || sample.timestamp_ns.saturating_sub(started) >= MAX_DURATION_NS {
            return self.close().map(|notice| format!("30-minute MVD recording limit reached. {notice}"));
        }
        if let Err(error) = recording.write(sample) {
            let path = recording.path.clone();
            self.recording = None;
            return Some(format!("MVD sensor diagnostics write failed: {error}; partial CSV: {}", path.display()));
        }
        None
    }

    pub fn close(&mut self) -> Option<String> {
        let mut recording = self.recording.take()?;
        Some(match recording.writer.flush() {
            Ok(()) => format!("MVD sensor diagnostics saved: {} ({} samples)", recording.path.display(), recording.rows),
            Err(error) => format!("MVD sensor diagnostics flush failed: {error}; partial CSV: {}", recording.path.display()),
        })
    }

    pub fn stop(&mut self) -> Option<String> {
        self.requested = false;
        self.close()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled_and_missing_directory_never_retry_or_create_files() {
        let mut trace = Trace::default();
        assert!(trace.configure(false, None).is_none());
        assert!(trace.configure(true, None).unwrap().contains("save directory missing"));
        assert!(trace.configure(true, None).is_none());
        assert!(trace.configure(false, None).is_none());
        assert!(trace.configure(true, None).is_some());
    }
    #[test]
    fn csv_shape_and_time_limit_flush_without_reopening() {
        let directory = std::env::temp_dir().join(format!("tgpulse-trace-test-{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        let recording = Recording::open(&directory).unwrap();
        let path = recording.path.clone();
        let mut trace = Trace { requested: true, recording: Some(recording) };
        let make_sample = |timestamp_ns| Sample { timestamp_ns, delta_ns: 16_666_667, event: "sample", gyro: [0.0, 0.0, 0.002], accel: None,
            diagnostics: crate::motion::OrientationTracker::default().diagnostics(), angles: None };
        assert!(trace.record(make_sample(0)).is_none());
        assert!(trace.record(make_sample(MAX_DURATION_NS)).unwrap().contains("limit reached"));
        assert!(trace.configure(true, None).is_none());
        let csv = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<_> = csv.lines().filter(|line| !line.starts_with('#')).collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].split(',').count(), 49);
        assert_eq!(lines[1].split(',').count(), 49);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
