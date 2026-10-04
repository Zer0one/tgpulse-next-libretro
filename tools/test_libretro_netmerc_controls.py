#!/usr/bin/env python3
"""Verify NetMerc U2 through the existing isolated ABI host.

Checks published profile/descriptors, live MVD policy, command edges and
native-state preservation. Requires a user-owned ROM; never edits it or reads
personal frontend settings/saves. This is not physical-controller acceptance.
"""
import argparse
import ctypes as c
import hashlib
import json
from pathlib import Path

from libretro_nvram_capture import INPUT, Variable
from test_libretro_model1_scope import Host


class Descriptor(c.Structure):
    _fields_ = [('port', c.c_uint), ('device', c.c_uint), ('index', c.c_uint),
                ('id', c.c_uint), ('description', c.c_char_p)]


class Controller(c.Structure):
    _fields_ = [('description', c.c_char_p), ('id', c.c_uint)]


class Port(c.Structure):
    _fields_ = [('types', c.POINTER(Controller)), ('count', c.c_uint)]


class Message(c.Structure):
    _fields_ = [('message', c.c_char_p), ('frames', c.c_uint)]


class Display(c.Structure):
    _fields_ = [('key', c.c_char_p), ('visible', c.c_bool)]


class NetMercHost(Host):
    def __init__(self, path):
        self.values = {}
        self.changed = False
        self.inputs = {}
        self.descriptors = []
        self.profiles = []
        self.profile_types = []
        self.displays = {}
        self.messages = []
        self.message_frames = []
        super().__init__(path)
        self.input_callback = INPUT(lambda p, d, i, b: self.inputs.get((p, d, i, b), 0))
        self.lib.retro_set_input_state(self.input_callback)
        self.lib.retro_set_controller_port_device.argtypes = [c.c_uint, c.c_uint]

    def environment(self, command, data):
        if command == 15:
            variable = c.cast(data, c.POINTER(Variable)).contents
            if variable.key in self.values:
                variable.value = self.values[variable.key]
                return True
        elif command == 17:
            c.cast(data, c.POINTER(c.c_bool))[0] = self.changed
            self.changed = False
            return True
        elif command == 11:
            entries = c.cast(data, c.POINTER(Descriptor))
            self.descriptors = []
            i = 0
            while entries[i].description:
                e = entries[i]
                self.descriptors.append((e.port, e.device, e.index, e.id, e.description.decode()))
                i += 1
            return True
        elif command == 35:
            entries = c.cast(data, c.POINTER(Port))
            self.profiles = []
            self.profile_types = []
            i = 0
            while entries and entries[i].count:
                self.profiles.append(entries[i].types[0].description.decode())
                self.profile_types.append([(entries[i].types[j].description.decode(), entries[i].types[j].id)
                                           for j in range(entries[i].count)])
                i += 1
            return True
        elif command == 55:
            e = c.cast(data, c.POINTER(Display)).contents
            self.displays[e.key.decode()] = bool(e.visible)
            return True
        elif command == 6:
            message = c.cast(data, c.POINTER(Message)).contents
            self.messages.append(message.message.decode())
            self.message_frames.append(message.frames)
            return True
        return super().environment(command, data)

    def options(self, mode, horizontal=30, vertical=20):
        self.values.update({b'tgpulse_next_netmerc_mvd_input': mode.encode(),
                            b'tgpulse_next_netmerc_mvd_horizontal_range': str(horizontal).encode(),
                            b'tgpulse_next_netmerc_mvd_vertical_range': str(vertical).encode()})
        self.changed = True

    def command_messages(self):
        return [s for s in self.messages if 'calibration unavailable:' in s or 'not ready for recentering' in s]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ('core', 'rom', 'output'):
        p.add_argument('--' + name, type=Path, required=True)
    p.add_argument('--rom-dir', type=Path, help='Also verify profile variants and device transitions for all Model 1 sets')
    args = p.parse_args()
    if args.output.exists():
        p.error('Use a new output file')
    host = NetMercHost(args.core)
    checks = []
    try:
        if args.rom_dir:
            for name in ('vr','vformula','vf','wingwar','wingwarj','wingwaru','wingwar360','swa','swaj','netmerc'):
                host.load(args.rom_dir/(name+'.zip'))
                assert len(host.profile_types)==2
                for variants in host.profile_types:
                    assert len(variants)==2 and [v[1] for v in variants]==[1,257]
                    assert variants[0][0]==variants[1][0]+' + Test/Service Slots'
                original=list(host.descriptors)
                for port in range(2):
                    host.lib.retro_set_controller_port_device(port,257)
                    assert host.descriptors==[d for d in original if not (d[0]==port and d[1]==1 and d[3] in (14,15))], name
                    host.lib.retro_set_controller_port_device(port,0)
                    assert not any(d[0]==port for d in host.descriptors), name
                    host.lib.retro_set_controller_port_device(port,1)
                    assert host.descriptors==original, name
                host.lib.retro_unload_game()
            checks.append('Ten-set ABI: full/reduced names, IDs and per-port descriptor transitions')
        host.load(args.rom)
        assert host.profiles == ['Special: Sega NetMerc + Test/Service Slots'] * 2
        assert host.profile_types == [[('Special: Sega NetMerc + Test/Service Slots',1),
                                       ('Special: Sega NetMerc',257)]] * 2
        assert [d[4] for d in host.descriptors if d[0] == 1] == ['Test', 'Service']
        assert all(host.displays[k] for k in host.displays if 'netmerc_mvd_' in k)
        for button, label in ((0, 'Trigger Button'), (12, 'Trigger Button'), (13, 'Trigger Button'),
                              (8, 'Thumb Button'), (10, 'Thumb Button'), (11, 'Thumb Button'),
                              (3, 'MVD Holder'), (5, 'MVD Holder'),
                              (9, 'MVD Calibrate'), (1, 'MVD Recenter')):
            assert (0, 1, 0, button, label) in host.descriptors
        checks.append('Profile, alias labels, P2 operators and visibility')
        original=list(host.descriptors)
        for port in range(2):
            host.lib.retro_set_controller_port_device(port,257)
            assert host.descriptors == [d for d in original if not (d[0]==port and d[1]==1 and d[3] in (14,15))]
            host.lib.retro_set_controller_port_device(port,0)
            assert not any(d[0]==port for d in host.descriptors)
            host.lib.retro_set_controller_port_device(port,1)
            assert host.descriptors==original
        checks.append('Per-port full/reduced/None descriptor transitions preserve gameplay aliases')
        host.run(60)
        holder = [(text, frames) for text, frames in zip(host.messages, host.message_frames)
                  if text.startswith('MVD Holder: ')]
        assert holder == [('MVD Holder: Cleared', 180)], holder
        checks.append('Initial native Holder notification is published once for 180 frames')
        baseline = host.snapshot()

        def frame(mode='auto', ranges=(30, 20), inputs=None, device=1):
            host.restore(baseline)
            host.lib.retro_set_controller_port_device(0, device)
            host.options(mode, *ranges)
            host.inputs = inputs or {}
            output = host.run(1)
            return host.snapshot(), output

        forward = frame()
        for face, aliases in ((0, (12, 13)), (8, (10, 11))):
            expected = frame(inputs={(0, 1, 0, face): 1})
            assert expected[0] != forward[0]
            for alias in aliases:
                assert frame(inputs={(0, 1, 0, alias): 1}) == expected
                assert frame(inputs={(1, 1, 0, alias): 1}) == forward
                if alias in (12, 13):
                    assert frame(inputs={(0, 5, 2, alias): 32767}) == expected
                    assert frame(inputs={(0, 5, 2, alias): 16383}) == forward
        checks.append('Native Trigger B/L2/R2 and Thumb A/L1/R1 aliases, thresholds and P2 isolation')
        for mode in ('auto', 'right_stick'):
            for axis in (0, 1):
                for value in (-32768, 32767):
                    moved = frame(mode, inputs={(0, 5, 1, axis): value})
                    assert moved[0] != forward[0]
                    assert frame(mode, (0, 0), {(0, 5, 1, axis): value}) == forward
                    assert frame('off', inputs={(0, 5, 1, axis): value}) == forward
                    assert frame(mode, inputs={(0, 5, 1, axis): value}, device=0) == forward
            assert frame(mode, inputs={(1, 5, 1, 0): 32767}) == forward
        assert frame('right_stick', (90, 20), {(0, 5, 1, 0): 32767})[0] != frame(
            'right_stick', (30, 20), {(0, 5, 1, 0): 32767})[0]
        checks.append('Live modes/ranges, axis locks, P1-only pose and fixed fallback')
        host.inputs = {(0, 5, 1, 0): 32767}
        host.options('right_stick')
        host.lib.retro_set_controller_port_device(0, 1)
        host.restore(baseline)
        host.run(1)
        pose_state = host.snapshot()
        continuation = host.run(2)
        host.restore(pose_state)
        assert host.run(2) == continuation
        checks.append('MVD native state replay')
        for button in (9, 1):
            assert frame(inputs={(0, 1, 0, button): 1}) == forward
            host.inputs = {}
            host.run(1)
            count = len(host.command_messages())
            host.inputs = {(0, 1, 0, button): 1}
            host.run(3)
            assert len(host.command_messages()) == count + 1
            held_state = host.snapshot()
            host.restore(held_state)
            host.run(1)
            assert len(host.command_messages()) == count + 1
            host.lib.retro_reset()
            host.run(1)
            assert len(host.command_messages()) == count + 1
            host.inputs = {}
            host.run(1)
            host.inputs = {(0, 1, 0, button): 1}
            host.run(1)
            assert len(host.command_messages()) == count + 2
        checks.append('Virtual commands preserve cabinet state; hold/reset/restore edge discipline')
        host.lib.retro_unload_game()
        assert all(host.displays[k] for k in host.displays if 'netmerc_mvd_' in k)
        checks.append('General options remain visible after unload')
    finally:
        host.close()
    report = {'core_sha256': hashlib.sha256(args.core.read_bytes()).hexdigest(),
              'checks': checks, 'passed': True, 'physical_controller_acceptance': False}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
