#!/usr/bin/env python3
"""U6 rumble callback/lifecycle checks using the existing isolated ABI host.

Requires user-owned NetMerc content. Physical motor response is not measured.
Native synthetic-firmware tests separately cover positive same-frame pulses.
"""
import argparse
import ctypes as c
import hashlib
import json
from pathlib import Path
from test_libretro_netmerc_controls import NetMercHost

SET = c.CFUNCTYPE(c.c_bool, c.c_uint, c.c_uint, c.c_uint16)


class Interface(c.Structure):
    _fields_ = [('set', SET)]


class RumbleHost(NetMercHost):
    def __init__(self, core):
        self.rumble_calls = []
        self.rumble_callback = SET(self.rumble)
        super().__init__(core)
        self.options('off')

    def rumble(self, port, effect, strength):
        self.rumble_calls.append((port, effect, strength))
        return True

    def environment(self, command, data):
        if command == 23:
            c.cast(data, c.POINTER(Interface))[0] = Interface(self.rumble_callback)
            return True
        return super().environment(command, data)

    def set_enabled(self, enabled):
        self.values[b'tgpulse_next_gamepad_rumble'] = b'enabled' if enabled else b'disabled'
        self.changed = True

    def stopped(self):
        assert self.rumble_calls[-2:] == [(0, 0, 0), (0, 1, 0)]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ('core', 'rom', 'output'):
        p.add_argument('--' + name, type=Path, required=True)
    a = p.parse_args()
    if a.output.exists():
        p.error('Use a new evidence file')
    host = RumbleHost(a.core)
    checks = []
    try:
        host.load(a.rom)
        host.run(1200)
        # Bounded coin/Holder/Trigger delivery; no forced native output writes.
        for inputs, frames in [({(0, 1, 0, 2): 1}, 3), ({}, 60),
                               ({(0, 1, 0, 3): 1}, 3), ({}, 60),
                               ({(0, 1, 0, 0): 1}, 60), ({}, 60)]:
            host.inputs = inputs
            host.run(frames)
        levels = {value for _, _, value in host.rumble_calls}
        assert levels <= {0, 39321}, levels
        assert all(port == 0 and effect in (0, 1) for port, effect, _ in host.rumble_calls)
        assert len(host.rumble_calls) >= 2400
        positive_callbacks = sum(value != 0 for _, _, value in host.rumble_calls)
        checks.append('Real NetMerc frames use P1 strong/weak callbacks and upstream binary levels only')
        host.inputs = {}
        state = host.snapshot()
        host.rumble_calls.clear()
        continuation = host.run(30)
        effects = list(host.rumble_calls)
        host.restore(state)
        host.stopped()
        host.rumble_calls.clear()
        assert host.run(30) == continuation
        assert host.rumble_calls == effects
        checks.append('State continuation preserves machine output; restore clears both frontend motors')
        host.set_enabled(False)
        host.rumble_calls.clear()
        host.run(1)
        host.stopped()
        count = len(host.rumble_calls)
        host.run(10)
        assert len(host.rumble_calls) == count
        host.set_enabled(True)
        host.run(2)
        assert len(host.rumble_calls) == count + 4
        checks.append('Live disable stops both motors; reenable resumes current native output')
        for device in (257, 0, 1):
            host.lib.retro_set_controller_port_device(0, device)
            host.stopped()
            count = len(host.rumble_calls)
            host.run(2)
            assert len(host.rumble_calls) == count + (0 if device == 0 else 4)
        checks.append('Full/reduced/None P1 transitions stop output and respect device availability')
        host.lib.retro_reset()
        host.stopped()
        host.run(1)
        host.lib.retro_unload_game()
        host.stopped()
        assert host.displays['tgpulse_next_gamepad_rumble']
        checks.append('Reset/unload shutdown and always-visible global option')
    finally:
        host.close()
    report = {'passed': True, 'checks': checks,
              'core_sha256': hashlib.sha256(a.core.read_bytes()).hexdigest(),
              'positive_callbacks_in_bounded_native_session': positive_callbacks,
              'physical_rumble_acceptance': False}
    a.output.parent.mkdir(parents=True, exist_ok=True)
    a.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
