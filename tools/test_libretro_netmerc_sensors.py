#!/usr/bin/env python3
"""Exercise U5 using the existing isolated ABI host and mock P1 sensors.

Calibration uses real monotonic time. No ROMs, personal saves, controller
configuration or physical devices are modified; this is not hardware acceptance.
"""
import argparse
import ctypes as c
import csv
import hashlib
import json
import time
from pathlib import Path
from test_libretro_netmerc_controls import NetMercHost

SET = c.CFUNCTYPE(c.c_bool, c.c_uint, c.c_uint, c.c_uint)
GET = c.CFUNCTYPE(c.c_float, c.c_uint, c.c_uint)


class Interface(c.Structure):
    _fields_ = [('set', SET), ('get', GET)]


class MessageExt(c.Structure):
    _fields_ = [('message', c.c_char_p), ('duration', c.c_uint),
                ('priority', c.c_uint), ('level', c.c_uint), ('target', c.c_uint),
                ('type', c.c_uint), ('progress', c.c_int8)]


class SensorHost(NetMercHost):
    def __init__(self, core, save_directory):
        self.save_directory = c.create_string_buffer(str(save_directory).encode())
        self.save_directory_available = True
        self.sensor_calls = []
        self.reads = []
        self.motion = [0., 0., 0.]
        self.gyro_available = True
        self.accel_available = True
        self.modern_messages = []
        self.sensor_callbacks = (SET(self.set_sensor), GET(self.get_sensor))
        super().__init__(core)

    def set_sensor(self, port, action, rate):
        assert port == 0
        self.sensor_calls.append((port, action, rate))
        return self.accel_available if action == 0 else self.gyro_available if action == 2 else True

    def get_sensor(self, port, axis):
        assert port == 0
        self.reads.append((port, axis))
        return self.motion[axis - 3] if axis >= 3 else 1. if axis == 2 else 0.

    def environment(self, command, data):
        if command == 31:
            if not self.save_directory_available:
                return False
            c.cast(data, c.POINTER(c.c_void_p))[0] = c.addressof(self.save_directory)
            return True
        if command == (25 | 0x10000):
            c.cast(data, c.POINTER(Interface))[0] = Interface(*self.sensor_callbacks)
            return True
        if command == 59:
            c.cast(data, c.POINTER(c.c_uint))[0] = 1
            return True
        if command == 60:
            m = c.cast(data, c.POINTER(MessageExt)).contents
            assert m.type == 0 and m.target == 0 and m.duration > 0
            self.modern_messages.append(m.message.decode())
            return True
        return super().environment(command, data)

    def paced(self, seconds):
        started = time.monotonic()
        deadline = started
        while time.monotonic() - started < seconds:
            self.run(1)
            deadline += 1 / 60
            time.sleep(max(0, deadline - time.monotonic()))


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ('core', 'rom', 'output'):
        p.add_argument('--' + name, type=Path, required=True)
    a = p.parse_args()
    if a.output.exists():
        p.error('Use a new evidence file')
    trace_root = a.output.parent / (a.output.stem + '-captures')
    if trace_root.exists():
        p.error('Use a fresh diagnostic capture directory')
    host = SensorHost(a.core, trace_root)
    checks = []
    try:
        host.options('off')
        host.load(a.rom)
        host.run(60)
        assert not host.sensor_calls and not host.reads
        host.options('sensors')
        host.paced(3.35)
        assert host.sensor_calls == [(0, 2, 120), (0, 0, 120)]
        assert sum('calibration complete' in m for m in host.modern_messages) == 1, host.modern_messages
        assert not host.messages, 'Modern messages unexpectedly fell back to legacy'
        checks.append('Real-time three-second calibration, P1 negotiation and queued modern OSD')
        calibration_count = sum('calibration complete' in m for m in host.modern_messages)
        negotiation = list(host.sensor_calls)
        for strength in (b'10', b'100', b'off'):
            host.values[b'tgpulse_next_netmerc_mvd_drift_compensation'] = strength
            host.changed = True
            host.paced(0.05)
            assert host.sensor_calls == negotiation
            assert sum('calibration complete' in m for m in host.modern_messages) == calibration_count
        assert host.displays.get('tgpulse_next_netmerc_mvd_drift_compensation', True)
        checks.append('Live drift-strength changes keep calibration and sensor negotiation')
        assert not trace_root.exists(), 'Disabled diagnostics created files'
        host.values[b'tgpulse_next_netmerc_mvd_drift_compensation'] = b'50'
        host.values[b'tgpulse_next_netmerc_mvd_sensor_diagnostics'] = b'enabled'
        host.changed = True
        host.paced(0.1)
        host.motion = [0., 0., 0.002]
        host.paced(0.15)
        host.values[b'tgpulse_next_netmerc_mvd_sensor_diagnostics'] = b'disabled'
        host.changed = True
        host.paced(0.05)
        captures = list(trace_root.glob('tgpulse-next/diagnostics/*.csv'))
        assert len(captures) == 1, captures
        with captures[0].open() as f:
            rows = list(csv.DictReader(line for line in f if not line.startswith('#')))
        assert len(rows) >= 5 and all(len(row) == 49 for row in rows)
        timestamps = [int(row['timestamp_ns']) for row in rows]
        assert timestamps == sorted(set(timestamps))
        assert all(row['phase'] == 'ready' and row['drift_strength_pct'] == '50' for row in rows)
        assert all(int(row['delta_ns']) > 0 for row in rows)
        moving = next(row for row in rows if float(row['raw_gyro_z']) > 0.001)
        assert abs(float(moving['gyro_y_rad_s']) - 0.002) < 1e-8
        assert abs(float(moving['corrected_y_rad_s']) - 0.002) < 1e-8
        assert float(moving['accel_y_m_s2']) > 9.8
        assert moving['pose_orientation_0'] and float(moving['yaw_rad']) > 0
        assert abs(float(moving['roll_rad'])) < 1e-8
        assert any(str(captures[0]) in m and 'recording:' in m for m in host.modern_messages)
        assert any(str(captures[0]) in m and 'saved:' in m for m in host.modern_messages)
        assert sum('calibration complete' in m for m in host.modern_messages) == calibration_count
        host.motion = [0., 0., 0.]
        checks.append('Optional CSV capture: disabled silence, raw axes/units, corrections, timestamps and flush/path OSD')
        host.save_directory_available = False
        host.values[b'tgpulse_next_netmerc_mvd_sensor_diagnostics'] = b'enabled'
        host.changed = True
        host.paced(0.1)
        assert sum('save directory missing' in m for m in host.modern_messages) == 1
        assert len(list(trace_root.glob('tgpulse-next/diagnostics/*.csv'))) == 1
        host.values[b'tgpulse_next_netmerc_mvd_sensor_diagnostics'] = b'disabled'
        host.changed = True
        host.paced(0.05)
        host.save_directory_available = True
        checks.append('Missing frontend save directory is notified once and tracking continues')
        host.inputs = {(0, 1, 0, 1): 1}
        host.run(3)
        assert host.modern_messages.count('MVD recentered') == 1
        host.inputs = {}
        host.run(1)
        state = host.snapshot()
        host.restore(state)
        assert host.sensor_calls[-2:] == [(0, 3, 0), (0, 1, 0)]
        host.run(1)
        host.lib.retro_reset()
        assert host.sensor_calls[-2:] == [(0, 3, 0), (0, 1, 0)]
        host.run(1)
        host.lib.retro_set_controller_port_device(0, 0)
        assert host.sensor_calls[-2:] == [(0, 3, 0), (0, 1, 0)]
        before = len(host.reads)
        host.run(2)
        assert len(host.reads) == before
        checks.append('Recenter edge, state/reset/device shutdown and disabled-port silence')
        host.lib.retro_set_controller_port_device(0, 257)
        host.accel_available = False
        host.run(1)
        assert any('without gravity stabilization' in m for m in host.modern_messages)
        host.options('right_stick')
        host.run(1)
        assert host.sensor_calls[-1] == (0, 3, 0)
        before = len(host.reads)
        host.run(2)
        assert len(host.reads) == before
        checks.append('Reduced profile, gyroscope-only capability and live stick-mode shutdown')
        host.gyro_available = False
        host.options('auto')
        host.run(1)
        unavailable = sum('sensors unavailable;' in m for m in host.modern_messages)
        calls = len(host.sensor_calls)
        host.run(3)
        assert unavailable == 1 and len(host.sensor_calls) == calls
        checks.append('Unavailable gyroscope fallback is notified once without retry spam')
        host.options('off')
        host.run(1)
        host.gyro_available = host.accel_available = True
        host.options('sensors')
        host.run(1)
        host.lib.retro_unload_game()
        assert host.sensor_calls[-2:] == [(0, 3, 0), (0, 1, 0)]
        checks.append('Unload releases both frontend sensors')
    finally:
        host.close()
    report = {'passed': True, 'core_sha256': hashlib.sha256(a.core.read_bytes()).hexdigest(),
              'checks': checks, 'messages': host.modern_messages,
              'physical_sensor_acceptance': False, 'sensor_captures': [str(path) for path in captures]}
    a.output.parent.mkdir(parents=True, exist_ok=True)
    a.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
