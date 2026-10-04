//! Read-only real-ROM camera trace. Link against an existing release core library.
//! Uses constant neutral HMD input, scripted cabinet inputs and no host devices.
//! Fine steps cover frames 254..256 of the reproduced startup; other NVRAM or
//! firmware revisions may require a different observation window. Snapshot
//! offsets below are verified against the current CPU and DPRAM encodings.
use tgpulse_core::{loader, model1::Model1System, model1io2::HmdPose};
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let roms = loader::load_model1_zip(&path).unwrap();
    let mut m = Model1System::new(&roms).unwrap();
    if std::env::args().nth(2).as_deref() == Some("--verify-bootstrap") {
        verify_bootstrap(&mut m);
        return;
    }
    m.inputs.analog[0] = 127;
    m.inputs.analog[2] = 127;
    let mut track_prev = (false, 0u8);
    let mut first_record = false;
    let mut prev = [0i16; 6];
    let mut mismatches = 0;
    let mut pulses = 0;
    let mut latched = 0;
    for frame in 0..1000 {
        // Same neutral tracking input as Off/Fixed Camera; no physical device.
        m.ioboard.set_hmd_pose(HmdPose::default());
        m.inputs.in0 = if (900..904).contains(&frame) {
            254
        } else {
            255
        };
        m.inputs.in1 = if (1200..1260).contains(&frame) {
            251
        } else if (1560..1564).contains(&frame) || frame >= 1620 {
            254
        } else {
            255
        };
        if frame == 254 {
            m.main_cpu.trace_cap = 10000;
            m.main_cpu.trace_lo = 0;
            m.main_cpu.trace_hi = 0xffffff;
        }
        if (254..257).contains(&frame) {
            for cycle in 0..tgpulse_core::model1::CYCLES_PER_FRAME {
                let before = m.ioboard.dpram()[0x80..0x8c].to_vec();
                let io_state = m.ioboard.advanced_cpu_state().unwrap().0;
                let io_before = io_state.pc;
                m.run_slice(1).unwrap();
                let after = m.ioboard.dpram()[0x80..0x8c].to_vec();
                if before != after {
                    let io_after = m.ioboard.advanced_cpu_state().unwrap().0.pc;
                    let state = m.ioboard.advanced_cpu_state().unwrap().0;
                    let cpu = bincode::serialize(&state).unwrap();
                    let saved = bincode::serialize(&m.ioboard.snapshot()).unwrap();
                    assert_eq!(&saved[4..4 + cpu.len()], &cpu);
                    let ram_start = 4 + cpu.len();
                    assert_eq!(
                        &saved[ram_start + 0x2000..ram_start + 0x2800],
                        &*m.ioboard.dpram()
                    );
                    let stack = ram_start + usize::from(state.sp - 0xe000);
                    let returns: Vec<u16> = saved[stack..(stack + 16).min(ram_start + 0x2000)]
                        .chunks_exact(2)
                        .map(|x| u16::from_le_bytes([x[0], x[1]]))
                        .collect();
                    println!(
                        "CPU before={io_state:?} STACK sp={:04x} returns={returns:04x?}",
                        state.sp
                    );

                    let recent = &m.main_cpu.trace[m.main_cpu.trace.len().saturating_sub(8)..];
                    println!("WRITE frame={frame} cycle={cycle} before={before:02x?} after={after:02x?} io_pc={io_before:04x}->{io_after:04x} v60_pc={:06x} v60_recent={recent:02x?}",m.main_cpu.ppc);
                    let lo = io_before.saturating_sub(8) as usize;
                    let hi = (io_after as usize + 8).min(roms.iocpu.len());
                    if lo < hi {
                        println!("IO_ROM {lo:04x}: {:02x?}", &roms.iocpu[lo..hi]);
                    }
                }
            }
            m.trigger_vblank();
        } else {
            m.run_frame().unwrap();
        }
        m.sound.samples.clear();
        let track = m.ioboard.tracking_status().unwrap();
        if (track.streaming, track.last_command) != track_prev
            || (!first_record && track.records > 0)
        {
            println!(
                "TRACK frame={frame} streaming={} last_command={:02x} records={}",
                track.streaming, track.last_command, track.records
            );
            track_prev = (track.streaming, track.last_command);
            first_record |= track.records > 0;
        }
        let ram = m.ioboard.dpram();
        let pose =
            std::array::from_fn(|i| i16::from_le_bytes([ram[0x80 + i * 2], ram[0x81 + i * 2]]));
        drop(ram);
        if pose != prev {
            println!("frame={frame} dpram_pose={pose:?}");
            prev = pose;
        }
        if pose != HmdPose::default().words() {
            mismatches += 1;
        }
        let end = m.ioboard.netmerc_motor().unwrap();
        let seen = end;
        if seen && !end {
            pulses += 1;
            if pulses <= 10 {
                println!("frame={frame} pulse_on=true final_latch=false");
            }
        }
        if end {
            latched += 1;
        }
    }
    println!("SUMMARY mismatching_pose_frames={mismatches} pulse_only_frames={pulses} latched_on_frames={latched}");
}

fn verify_bootstrap(m: &mut Model1System) {
    fn pose(m: &Model1System) -> [i16; 6] {
        let ram = m.ioboard.dpram();
        std::array::from_fn(|i| i16::from_le_bytes([ram[0x80 + i * 2], ram[0x81 + i * 2]]))
    }
    fn step(m: &mut Model1System, frame: usize) {
        let input = if frame < 940 {
            HmdPose::default()
        } else if frame < 980 {
            HmdPose {
                position: [4, -8, 120],
                orientation: [12868, 25736, -12868],
            }
        } else {
            HmdPose {
                position: [0; 3],
                orientation: [0; 3],
            }
        };
        m.ioboard.set_hmd_pose(input);
        m.run_frame().unwrap();
        m.sound.samples.clear();
        let actual = pose(m);
        if frame < 940 || frame == 960 || frame == 999 {
            assert_eq!(actual, input.words(), "frame {frame}");
        }
    }
    for frame in 0..1000 {
        if frame == 900 || frame == 945 {
            let before = m.save_state().unwrap();
            assert_eq!(u32::from_le_bytes(before[8..12].try_into().unwrap()), 5);
            for replay_frame in frame..frame + 10 {
                step(m, replay_frame);
            }
            let expected = m.save_state().unwrap();
            m.load_state(&before).unwrap();
            for replay_frame in frame..frame + 10 {
                step(m, replay_frame);
            }
            assert_eq!(
                m.save_state().unwrap(),
                expected,
                "replay from frame {frame}"
            );
            m.load_state(&before).unwrap();
            // A version-4 envelope must fail without changing the live machine.
            let mut old = before.clone();
            old[8..12].copy_from_slice(&4u32.to_le_bytes());
            assert!(m.load_state(&old).is_err());
            assert_eq!(m.save_state().unwrap(), before);
            println!("STATE frame={frame} replay=pass legacy_rejection=atomic");
        }
        step(m, frame);
        if [0, 255, 926, 928, 960, 999].contains(&frame) {
            println!("BOOTSTRAP frame={frame} pose={:?}", pose(m));
        }
    }
    println!("SUMMARY neutral_startup=pass live_pose=pass zero_pose=pass state_replay=pass");
}
