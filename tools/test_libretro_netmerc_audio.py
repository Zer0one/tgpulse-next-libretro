#!/usr/bin/env python3
"""Verify NetMerc donor resources and states using the existing isolated ABI host.

Requires user-owned complete game ZIPs. Writes PCM-only fixtures in a new output
directory; never edits original ROMs, personal saves or frontend configuration.
Tests actual resource identity, fallback, restart semantics and manual gains.
"""
import argparse
import ctypes as c
import hashlib
import json
import os
import shutil
from pathlib import Path
from zipfile import ZipFile, ZIP_STORED

from test_libretro_netmerc_controls import NetMercHost


class MessageExt(c.Structure):
    _fields_ = [('message', c.c_char_p), ('duration', c.c_uint), ('priority', c.c_uint),
                ('level', c.c_uint), ('target', c.c_uint), ('message_type', c.c_uint),
                ('progress', c.c_int8)]

KEY = b'tgpulse_next_netmerc_audio_donor'
DONORS = ('vf', 'vr', 'swa', 'wingwar')


class AudioHost(NetMercHost):
    def __init__(self, core, system):
        self.system = str(system).encode()
        self.extended = False
        self.extended_messages = []
        super().__init__(core)

    def environment(self, command, data):
        if command == 59 and self.extended:
            c.cast(data, c.POINTER(c.c_uint))[0] = 1
            return True
        if command == 60 and self.extended:
            m = c.cast(data, c.POINTER(MessageExt)).contents
            self.extended_messages.append((m.message.decode(), m.duration, m.priority, m.level, m.message_type))
            return True
        if command == 9:
            c.cast(data, c.POINTER(c.c_char_p))[0] = self.system
            return True
        return super().environment(command, data)


def pcm_files():
    # Reuse the same ROM catalogue as the upstream strict loader.
    result = {}; game = None
    source = Path(__file__).resolve().parents[1] / 'crates/tgpulse-core/src/roms_db.dat'
    for line in source.read_text().splitlines():
        words = line.split()
        if words and words[0] == 'G':
            game = words[1]
        elif game in DONORS and len(words) > 2 and words[0] == 'L' and words[1] in ('m1audio:pcm1', 'm1audio:pcm2'):
            result.setdefault(game, []).append(words[2])
    assert set(result) == set(DONORS)
    return result


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ('core', 'rom-dir', 'output'):
        p.add_argument('--' + name, type=Path, required=True)
    a = p.parse_args()
    out = a.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    content = out / 'content'; content.mkdir()
    system = out / 'system'; (system / 'tgpulse-next').mkdir(parents=True)
    # Host.load canonicalizes paths: a symlink would accidentally use the
    # original ROM directory and bypass these isolated resource fixtures.
    shutil.copyfile(a.rom_dir / 'netmerc.zip', content / 'netmerc.zip')
    files = pcm_files()
    for donor in DONORS:
        with ZipFile(a.rom_dir / (donor + '.zip')) as source, ZipFile(content / (donor + '.zip'), 'w', ZIP_STORED) as dest:
            for name in files[donor]:
                dest.writestr(name, source.read(name))
    # A valid ZIP only in system/cwd must never be used as a game donor.
    for directory in (system / 'tgpulse-next', out):
        (directory / 'vf.zip').symlink_to(content / 'vf.zip')
    host = AudioHost(a.core, system)
    results = []; snapshots = {}; observations = {}
    try:
        def load(value=None):
            host.messages.clear()
            host.message_frames.clear()
            host.values = {} if value is None else {KEY: value.encode()}
            host.load(content / 'netmerc.zip')
            assert host.displays[KEY.decode()]
            assert not any(s.startswith('NetMerc Audio') for s in host.messages), 'Audio notice was sent before frontend startup'
            host.run(1)
            assert all(frames == 300 for text, frames in zip(host.messages, host.message_frames)
                       if text.startswith('NetMerc Audio'))
            return [s for s in host.messages if s.startswith('NetMerc Audio')]

        for donor in (*DONORS, 'off'):
            messages = load(None if donor == 'vf' else donor)
            expected = ['NetMerc Audio: Procedural Fallback'] if donor == 'off' else []
            assert messages == expected, messages
            observations[donor] = host.run(120)
            snapshots[donor] = host.snapshot()
            expected_continuation = host.run(5)
            expected_state = host.snapshot()
            host.restore(snapshots[donor]); assert host.run(5) == expected_continuation
            assert host.snapshot() == expected_state
            # A changed selection is only applied on content reload.
            host.restore(snapshots[donor])
            host.values[KEY] = b'vr' if donor != 'vr' else b'off'; host.changed = True
            assert host.run(5) == expected_continuation
            assert host.snapshot() == expected_state
            # Muting every source survives state restore and takes effect live.
            host.restore(snapshots[donor])
            for source in ('multipcm1', 'multipcm2', 'ym3438', 'dsb'):
                host.values[('tgpulse_next_' + source + '_gain').encode()] = b'mute'
            host.changed = True
            assert host.run(5)['audio_sha256'] == hashlib.sha256(bytes(host.samples * 4)).hexdigest()
            host.restore(snapshots[donor]); host.changed = True
            assert host.run(5)['audio_sha256'] == hashlib.sha256(bytes(host.samples * 4)).hexdigest()
            host.lib.retro_unload_game()
            results.append({'donor': donor, 'messages': expected, 'continuation': True,
                            'change_requires_reload': True, 'manual_mute': True})
        assert len(set(snapshots.values())) == 5, 'Resource identities must distinguish actual banks/fallback'
        load('vf'); host.run(120)
        before = host.snapshot()
        for donor in ('vr', 'swa', 'wingwar', 'off'):
            host.reject_state(snapshots[donor])
            assert host.snapshot() == before, 'Incompatible state mutated the machine'
        host.lib.retro_reset(); host.run(5)
        host.restore(snapshots['vf']); assert host.snapshot() == snapshots['vf']
        host.lib.retro_unload_game()
        original = content / 'vf.zip'; preserved = content / 'vf.valid.zip'
        original.rename(preserved)
        # Decoys remain valid while the adjacent ZIP is absent.
        for directory in (system / 'tgpulse-next', out):
            (directory / 'vf.zip').unlink(); (directory / 'vf.zip').symlink_to(preserved)
        previous = Path.cwd(); os.chdir(out)
        try:
            for mode in ('missing', 'malformed', 'incomplete', 'blank'):
                if mode == 'malformed': original.write_bytes(b'not a ZIP')
                elif mode in ('incomplete', 'blank'):
                    with ZipFile(original, 'w', ZIP_STORED) as dest, ZipFile(preserved) as source:
                        for name in (files['vf'][:1] if mode == 'incomplete' else files['vf']):
                            data = source.read(name)
                            dest.writestr(name, data if mode == 'incomplete' else bytes(len(data)))
                messages = load('vf')
                assert 'NetMerc Audio Donor Unavailable - Procedural Fallback' in messages
                assert host.run(120) == observations['off'], mode
                assert host.snapshot() == snapshots['off'], mode
                host.lib.retro_unload_game()
                original.unlink(missing_ok=True)
                results.append({'fallback': mode, 'equals_off': True})
        finally:
            os.chdir(previous); preserved.rename(original)
        # Modern queued warnings survive later informational notices; the
        # complete resource matrix above also exercises legacy compatibility.
        host.extended = True
        host.values = {KEY: b'off'}
        host.extended_messages.clear()
        host.load(content / 'netmerc.zip')
        assert not any(m[0].startswith('NetMerc Audio') for m in host.extended_messages)
        host.run(1)
        audio = [m for m in host.extended_messages if m[0].startswith('NetMerc Audio')]
        assert len(audio) == 1 and audio[0][0] == 'NetMerc Audio: Procedural Fallback'
        assert 5000 <= audio[0][1] <= 5300 and audio[0][2:] == (3, 2, 0)
        host.run(5)
        assert len([m for m in host.extended_messages if m[0].startswith('NetMerc Audio')]) == 1
        host.lib.retro_unload_game()
        report = {'core_sha256': hashlib.sha256(a.core.read_bytes()).hexdigest(),
                  'deferred_queued_warning': True, 'legacy_message_fallback': True,
                  'pcm_only_donors': True, 'system_and_cwd_ignored': True,
                  'incompatible_states_rejected_atomically': True, 'reset': True, 'checks': results}
        (out / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
        print(json.dumps(report))
    finally:
        host.close()


if __name__ == '__main__':
    main()
