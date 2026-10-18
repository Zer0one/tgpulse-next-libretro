#!/usr/bin/env python3
"""Bounded PCM recovery check using the existing isolated Model 1 ABI host.

Reads owned ROMs only. Nonzero PCM and deterministic repeated-reset output do
not establish frontend playback or listening acceptance.
"""
import argparse
import ctypes as c
import hashlib
import json
from pathlib import Path

from test_libretro_lcf_options import LCFHost


class AudioHost(LCFHost):
    def __init__(self, core):
        self.pcm = bytearray()
        super().__init__(core)

    def audio(self, data, frames):
        self.pcm.extend(c.string_at(data, frames * 4))
        return super().audio(data, frames)

    def capture(self, frames):
        self.pcm.clear()
        result = self.run(frames)
        assert any(self.pcm), 'No nonzero PCM after reset'
        result['nonzero_pcm'] = True
        return result


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for key in ('core', 'rom-dir', 'output'):
        p.add_argument('--' + key, type=Path, required=True)
    p.add_argument('--frames', type=int, default=600)
    args = p.parse_args()
    if args.output.exists() or not 360 <= args.frames <= 1200:
        p.error('Use a new output and 360–1200 frames')
    records = []
    for timing in (b'native', b'60hz'):
        for name in ('vr', 'vf', 'swa', 'netmerc'):
            host = AudioHost(args.core)
            try:
                host.values.update({b'tgpulse_next_av_timing': timing,
                                    b'tgpulse_next_renderer': b'software',
                                    b'tgpulse_next_nvram_settings': b'enabled'})
                if name == 'swa':
                    host.values[b'tgpulse_next_nvram_swa_advertise_sound'] = b'ON'
                if name == 'netmerc':
                    host.values[b'tgpulse_next_nvram_netmerc_advertise'] = b'ON'
                host.load(args.rom_dir / (name + '.zip'))
                window = max(args.frames, 1800) if name in ('swa', 'netmerc') else args.frames
                startup = host.capture(window)
                option_reset = None
                if name == 'vf':
                    host.values[b'tgpulse_next_nvram_vf_difficulty'] = b'HARD'
                    host.changed = True
                    before = len(host.messages)
                    option_reset = host.capture(window)
                    assert any('machine reset' in s for s in host.messages[before:])
                resets = []
                for _ in range(2):
                    host.lib.retro_reset()
                    resets.append(host.capture(window))
                assert resets[0]['audio_sha256'] == resets[1]['audio_sha256'], (name, timing)
                assert resets[0]['audio_frames'] == resets[1]['audio_frames']
                records.append(dict(set=name, timing=timing.decode(), startup=startup,
                                    option_reset=option_reset, explicit_resets=resets,
                                    repeated_reset_pcm_equal=True))
                print('PASS:', name, timing.decode(), 'audio recovery', flush=True)
            finally:
                host.close()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(dict(core_sha256=hashlib.sha256(args.core.read_bytes()).hexdigest(),
                                          frames_per_window=args.frames, cases=records), indent=2) + '\n')


if __name__ == '__main__':
    main()
