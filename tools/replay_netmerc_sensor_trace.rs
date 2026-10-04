//! Offline replay of a full format-1 50% sensor capture using the actual tracker.
//! No host sensor acquisition or changes to the installed core.
//! rustc --edition=2024 tools/replay_netmerc_sensor_trace.rs -o /private/tmp/replay_netmerc_sensor_trace
//! /private/tmp/replay_netmerc_sensor_trace capture.csv > replay.csv
//! Requires capture from tracker startup, including calibration/reset events.
#[allow(dead_code)]
#[path = "../crates/tgpulse/src/input/motion.rs"]
mod motion;
use motion::{OrientationTracker, MotionSample, SensorKind};
fn main() {
 let args: Vec<String> = std::env::args().collect();
 let data=std::fs::read_to_string(&args[1]).unwrap();
 let mut lines=data.lines().filter(|line| !line.starts_with('#'));
 let header: Vec<_>=lines.next().unwrap().split(',').collect();
 let idx=|key:&str| header.iter().position(|h| *h==key).unwrap();
 let strength=idx("drift_strength_pct");
 let i=idx("timestamp_ns");let event=idx("event"); let accel=idx("accel_available"); let gravity=idx("gravity_enabled");
 let angles=[idx("pitch_rad"),idx("yaw_rad"),idx("roll_rad")];
 let gyros=[idx("gyro_x_rad_s"),idx("gyro_y_rad_s"),idx("gyro_z_rad_s")];
 let accels=[idx("accel_x_m_s2"),idx("accel_y_m_s2"),idx("accel_z_m_s2")];
 let mut trackers: Vec<_>=[0,50,100].map(|strength| {
  let mut t=OrientationTracker::with_calibration_timing(1_000_000_000,3_000_000_000);t.set_drift_compensation(strength);t
 }).into_iter().collect();
 let mut first=None; let mut max_error=0.0_f64;
 println!("time_s,event,yaw_off_deg,yaw_50_deg,yaw_100_deg,dwell_50_s,dwell_100_s");
 for line in lines {
  let cols:Vec<_>=line.split(',').collect();
  assert_eq!(cols.len(),header.len(),"CSV row/header mismatch");
  assert_eq!(cols[strength],"50","This comparison requires a constant 50% capture");
  let timestamp_ns=cols[i].parse::<u64>().unwrap();
  let first=*first.get_or_insert(timestamp_ns);let time=(timestamp_ns-first) as f64/1e9;
  for t in &mut trackers {
   t.set_gravity_stabilization(cols[gravity]=="true");
   if cols[event]=="calibrate" { t.reset(); }
   if cols[event]=="recenter" { t.recenter(); }
   if cols[accel]=="true" { t.observe(MotionSample {kind:SensorKind::Accel, data:accels.map(|j| cols[j].parse().unwrap()),timestamp_ns}); }
   t.observe(MotionSample {kind:SensorKind::Gyro, data:gyros.map(|j| cols[j].parse().unwrap()),timestamp_ns});
  }
  if let Some(actual)=trackers[1].angles() {
   for (n,j) in angles.into_iter().enumerate() {
    max_error=max_error.max((actual[n]-cols[j].parse::<f64>().unwrap()).abs());
   }
   println!("{time},{},{},{},{},{},{}",cols[event],trackers[0].angles().unwrap()[1].to_degrees(),actual[1].to_degrees(),trackers[2].angles().unwrap()[1].to_degrees(),trackers[1].diagnostics().stationary_seconds,trackers[2].diagnostics().stationary_seconds);
  }
 }
 eprintln!("maximum replay error in radians: {max_error}");
 assert!(max_error<1e-9,"50% replay did not match captured orientation");
}
