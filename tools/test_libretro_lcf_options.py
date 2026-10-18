#!/usr/bin/env python3
"""Focused LCF registration/profile lifecycle checks using the existing ABI host.

Reads user-owned ROMs; no personal settings, saves or devices are accessed.
This is callback evidence, not actual RetroArch menu or hardware acceptance.
"""
import argparse
import ctypes as c
import hashlib
import json
from pathlib import Path

from test_libretro_netmerc_controls import NetMercHost
from test_libretro_netmerc_lcd import Options
from libretro_nvram_capture import Variable

UPDATE = c.CFUNCTYPE(c.c_bool)


class LCFHost(NetMercHost):
    def __init__(self, core, legacy=False, saved=None):
        self.definitions = {}
        self.update_display = None
        self.publication_reentry = False
        self.legacy = legacy
        self.saved = saved or {}
        self.legacy_definitions = {}
        super().__init__(core)

    def environment(self, command, data):
        if command == 52:
            c.cast(data, c.POINTER(c.c_uint))[0] = 1 if self.legacy else 2
            return True
        if command == 16:
            values = c.cast(data, c.POINTER(Variable))
            i = 0
            while values[i].key:
                self.legacy_definitions[values[i].key] = values[i].value
                i += 1
            return True
        if command == 67:
            definitions = c.cast(data, c.POINTER(Options)).contents.definitions
            self.definitions = {}
            i = 0
            while definitions[i].key:
                d = definitions[i]
                values = []
                for value in d.values:
                    if not value.value:
                        break
                    values.append((value.value, value.label))
                self.definitions[d.key] = (d.desc, values, d.default_value)
                i += 1
            return True
        if command == 69:
            self.update_display = UPDATE(c.cast(data, c.POINTER(c.c_void_p))[0])
            return True
        if command == 15:
            value = c.cast(data, c.POINTER(Variable)).contents
            if value.key in self.saved and value.key not in self.values:
                value.value = self.saved[value.key]
                return True
            if value.key not in self.values and value.key in self.definitions:
                value.value = self.definitions[value.key][2]
                return True
        if command == 70:
            value = c.cast(data, c.POINTER(Variable)).contents
            self.values[value.key] = value.value
            return True
        if command == 35 and self.publication_reentry:
            # Profiles can cause frontend re-entry while the menu is paused.
            self.lib.retro_get_memory_size(0)
        return super().environment(command, data)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--core', type=Path, required=True)
    parser.add_argument('--rom-dir', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    host = LCFHost(args.core)
    assert not host.displays['tgpulse_next_nvram_settings']
    timing = host.definitions[b'tgpulse_next_av_timing']
    assert timing[2] == b'60hz'
    assert timing[1] == [(b'60hz', b'60 Hz (Default)'), (b'native', b'57.524 Hz (Native Model 1)')]
    records = []
    for name in ('vr', 'vformula', 'wingwar', 'wingwaru', 'wingwarj', 'wingwar360'):
        key = f'tgpulse_next_automatic_network_{name}'.encode()
        values = host.definitions[key][1]
        expected = [b'-1', b'0', b'1', b'2'] if name.startswith('wingwar') else [b'-1', b'0'] + [str(i).encode() for i in range(1, 9)] + [b'r', b'l', b's']
        assert [raw for raw, _ in values] == expected
        assert host.definitions[key][2] == b'0'
        records.append({'set': name, 'canonical_values': [raw.decode() for raw in expected]})
    for name in ('vr', 'vf', 'swa'):
        host.load(args.rom_dir / f'{name}.zip')
        assert host.displays['tgpulse_next_nvram_settings']
        assert all(len(types) == 1 for types in host.profile_types)
        for raw, full in ((b'normal', False), (b'test_service', True), (b'unknown', True)):
            host.values[b'tgpulse_next_controller_profile_type'] = raw
            host.publication_reentry = True
            host.update_display()
            host.publication_reentry = False
            assert all(label.endswith(' + Test/Service Slots') == full for label in host.profiles)
            assert any(d[3] == 14 and d[4] == 'Service' for d in host.descriptors) == full
            assert any(d[3] == 15 and d[4] == 'Test' for d in host.descriptors) == full
            host.lib.retro_set_controller_port_device(0, 257)  # Historical reduced subclass.
            assert any(d[0] == 0 and d[4] == 'Service' for d in host.descriptors) == full
            host.changed = True
            host.run(2)
        host.lib.retro_set_controller_port_device(0, 0)
        assert not any(d[0] == 0 for d in host.descriptors)
        host.lib.retro_unload_game()
        assert not host.displays['tgpulse_next_nvram_settings']
        host.lib.retro_set_controller_port_device(0, 1)
    host.lib.retro_deinit()
    for legacy in (False, True):
        old = LCFHost(args.core, legacy, {b'tgpulse_next_automatic_network_vr': b'red'})
        if legacy:
            assert old.legacy_definitions[b'tgpulse_next_av_timing'].endswith(b'60hz|native')
            assert old.legacy_definitions[b'tgpulse_next_controller_profile_type'].endswith(b'test_service|normal')
            assert b'; 0|-1|1|2|3|4|5|6|7|8|r|l|s' in old.legacy_definitions[b'tgpulse_next_automatic_network_vr']
        else:
            assert b'red' not in [raw for raw, _ in old.definitions[b'tgpulse_next_automatic_network_vr'][1]]
        assert old.saved[b'tgpulse_next_automatic_network_vr'] == b'red'
        old.lib.retro_deinit()
    result = {'core_sha256': hashlib.sha256(args.core.read_bytes()).hexdigest(),
              'canonical_registration': records, 'profile_sets': ['vr', 'vf', 'swa'],
              'paused_callback_reentry': 'pass', 'legacy_device_and_none': 'pass',
              'nvram_load_unload_visibility': 'pass', 'evidence': 'isolated ABI callbacks'}
    result['modern_and_legacy_published_ans_registration'] = 'pass'
    (args.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
