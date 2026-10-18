#!/usr/bin/env python3
"""F14 network authority/visibility/persistence through the existing ABI host.

Only user-owned ROMs are read; no frontend settings or personal saves are used.
Device topology, native EEPROM and frontend selector publication are separate
assertions. Frame-polled UI and physical gameplay remain separate evidence.
"""
import argparse
import binascii
import ctypes as c
import json
from pathlib import Path
import sys

from test_libretro_lcf_options import LCFHost
from libretro_nvram_capture import validate_save

SUPPORTED = ('vr', 'vformula', 'wingwar', 'wingwaru', 'wingwarj', 'wingwar360')
UNSUPPORTED = ('vf', 'swa', 'swaj', 'netmerc')


class NetworkHost(LCFHost):
    def environment(self, command, data):
        if command == 15:
            # RetroArch consumes the update flag on every GET_VARIABLE,
            # including unrelated profile reads and paused display callbacks.
            self.changed = False
        if command == 70:
            # Publishing a selector may synchronously re-enter the core.
            self.lib.retro_get_memory_size(0)
        return super().environment(command, data)


def save(host, name):
    raw = c.string_at(host.lib.retro_get_memory_data(0), host.lib.retro_get_memory_size(0))
    validate_save(raw, name)
    ee = raw[64 + 65536:]
    if name in SUPPORTED:
        if name.startswith('wingwar'):
            crc = binascii.crc_hqx(ee[10:66], 14873)
            assert int.from_bytes(ee[8:10], 'little') == crc
            assert ee[66:120] == ee[6:60]
        else:
            crc = binascii.crc_hqx(bytes(ee[i ^ 1] for i in range(10, 128)), 0)
            assert int.from_bytes(ee[8:10], 'big') == crc
    return raw, ee


def switch(host, name, phase, values, display=True):
    print(f'F14_CASE {name} {phase}', file=sys.stderr, flush=True)
    host.values.update(values)
    host.changed = True
    if display:
        host.update_display()
        host.update_display()  # Repeated visibility refresh must retain the edit.
    host.run(1)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ('core', 'rom-dir', 'output'):
        p.add_argument('--' + name, type=Path, required=True)
    a = p.parse_args()
    a.output.mkdir(parents=True, exist_ok=False)
    results = []
    for name in SUPPORTED + UNSUPPORTED:
        host = NetworkHost(a.core)
        assert b'tgpulse_next_network_board' not in host.definitions
        host.values[b'tgpulse_next_renderer'] = b'software'
        host.load(a.rom_dir / (name + '.zip'))
        host.run(1)
        visible = [k for k, shown in host.displays.items() if shown and
                   (k.startswith('tgpulse_next_linked_cabinets_') or
                    k.startswith('tgpulse_next_automatic_network_'))]
        assert len(visible) == 2, (name, visible)
        if name in UNSUPPORTED:
            assert all(k.endswith('_unsupported') for k in visible)
            for k in visible:
                assert host.definitions[k.encode()][1] == [(b'Not Supported', b'Not Supported')]
            results.append({'set': name, 'status': 'Not Supported', 'visible_controls': visible})
            host.lib.retro_unload_game(); host.lib.retro_deinit()
            continue

        prefix = f'tgpulse_next_nvram_{name}_'
        linked = f'tgpulse_next_linked_cabinets_{name}'.encode()
        role = f'tgpulse_next_automatic_network_{name}'.encode()
        gate = b'tgpulse_next_nvram_settings'
        native_role = (prefix + ('network' if name.startswith('wingwar') else 'link_id')).encode()
        difficulty = (prefix + ('game_difficulty' if name.startswith('wingwar') else 'difficulty')).encode()
        difficulty_offset = 23 if name.startswith('wingwar') else 33
        assert host.definitions[linked][2] == b'-1'
        assert host.definitions[role][2] == b'0'
        assert host.lib.retro_serialize_size() > 0  # Default topology disconnects board.
        switch(host, name, 'unrelated', {gate: b'enabled', difficulty: b'4' if name.startswith('wingwar') else b'HARDEST'})
        _, original = save(host, name)
        assert original[difficulty_offset] == 3
        switch(host, name, 'anrs_without_nvram', {gate: b'disabled', role: b'2'})
        _, ee = save(host, name)
        assert ee[difficulty_offset] == 3
        assert host.values[native_role] == b'SLAVE'  # Synchronized even while hidden.
        if name.startswith('wingwar'):
            assert ee[36:38] == bytes([1, 2])
        else:
            assert ee[13] == 2
            assert ee[35] == (1 if name == 'vformula' else 7)
        assert host.lib.retro_serialize_size() > 0  # Slave is not converted offline.
        assert not host.displays[native_role.decode()]
        native_id = None
        if not name.startswith('wingwar'):
            native_id = (prefix + ('car_number' if name == 'vformula' else 'car_color')).encode()
            expected_id = b'NO.2 (BLUE)' if name == 'vformula' else b'ORANGE'
            assert host.values[native_id] == expected_id
            assert not host.displays[native_id.decode()]

        switch(host, name, 'hidden_manual_competition', {gate: b'enabled', native_role: b'MASTER'})
        assert host.values[native_role] == b'SLAVE'
        switch(host, name, 'handover', {role: b'-1'})
        assert host.values[native_role] == b'SLAVE'
        assert host.displays[native_role.decode()]
        if native_id:
            assert host.displays[native_id.decode()]
            assert host.values[native_id] == expected_id
        switch(host, name, 'manual_without_display_callback', {native_role: b'MASTER'}, display=False)
        _, ee = save(host, name)
        assert (ee[37] if name.startswith('wingwar') else ee[13]) == 1
        switch(host, name, 'nvram_off_manual', {gate: b'disabled', native_role: b'SLAVE'})
        _, ee = save(host, name)
        assert (ee[37] if name.startswith('wingwar') else ee[13]) == 1
        for alias in (b'0', b'1'):
            switch(host, name, 'disabled_alias_' + alias.decode(), {linked: alias})
            assert host.lib.retro_serialize_size() > 0
        # Topology and managed role change together: exactly one construction/reset.
        switch(host, name, 'coalesced', {linked: b'2', role: b'2'})
        assert host.lib.retro_serialize_size() == 0
        assert host.values[native_role] == b'SLAVE'
        switch(host, name, 'unchanged_resume', {})
        switch(host, name, 'disconnect_independent', {linked: b'-1'})
        assert host.lib.retro_serialize_size() > 0
        assert host.values[native_role] == b'SLAVE'
        saved, _ = save(host, name)
        host.lib.retro_unload_game()
        host.load(a.rom_dir / (name + '.zip'))
        c.memmove(host.lib.retro_get_memory_data(0), saved, len(saved))
        host.run(1)
        _, ee = save(host, name)
        assert (ee[37] if name.startswith('wingwar') else ee[13]) == 2
        assert ee[difficulty_offset] == 3
        # A valid saved Slave image does not override the default Standalone
        # automation, even when the general NVRAM Settings switch is off.
        host.lib.retro_unload_game()
        host.values[role] = b'0'
        host.load(a.rom_dir / (name + '.zip'))
        c.memmove(host.lib.retro_get_memory_data(0), saved, len(saved))
        host.run(1)
        _, ee = save(host, name)
        assert (ee[36] if name.startswith('wingwar') else ee[13]) == 0
        assert ee[difficulty_offset] == 3
        switch(host, name, 'invalid', {linked: b'99', role: b'relay'})
        _, ee = save(host, name)
        assert (ee[36] if name.startswith('wingwar') else ee[13]) == 0
        assert sum('Invalid ' in message for message in host.messages) >= 2
        if not name.startswith('wingwar'):
            for token in (b'r', b'l', b's'):
                switch(host, name, 'special_' + token.decode(), {linked: b'-1', role: token})
                _, ee = save(host, name)
                assert ee[13] == 3
                assert host.values[native_role] == b'LIVE'
        results.append({'set': name, 'authority_handover': 'pass', 'native_integrity': 'pass',
                        'role_topology_independent': 'pass', 'persistence': 'pass'})
        host.lib.retro_unload_game(); host.lib.retro_deinit()

    for legacy in (False, True):
        h = NetworkHost(a.core, legacy)
        if legacy:
            assert b'; -1|' in h.legacy_definitions[b'tgpulse_next_linked_cabinets_vr']
            assert h.legacy_definitions[b'tgpulse_next_automatic_network_vr'].startswith(b'Auto Network Role Settings; 0|')
            assert b'tgpulse_next_network_board' not in h.legacy_definitions
        else:
            assert dict(h.definitions[b'tgpulse_next_automatic_network_vformula'][1])[b'2'] == b'No.2 (Blue) (Slave)'
        h.lib.retro_deinit()
    (a.output / 'result.json').write_text(json.dumps(results, indent=2) + '\n')
    print('PASS: ten workbook sets, authority handover, topology, persistence, aliases and registration')


if __name__ == '__main__':
    main()
