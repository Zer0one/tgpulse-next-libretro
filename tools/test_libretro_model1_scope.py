#!/usr/bin/env python3
"""Compare Model 1 cores through the existing ABI host.

No frontend settings or saves are read or written. Check frame/audio/state/Save
RAM equality and check the selected Save State compatibility policy. This is
an implementation regression gate, not gameplay or controller acceptance.
"""
import argparse
import ctypes as c
import hashlib
import json
import struct
from pathlib import Path

from libretro_nvram_capture import ENV, VIDEO, BATCH, POLL, INPUT, GameInfo, Variable

SETS = ('vr', 'vformula', 'vf', 'wingwar', 'wingwarj', 'wingwaru', 'wingwar360', 'swa', 'swaj')


class Host:
    def __init__(self, path):
        self.lib = c.CDLL(str(path.resolve(strict=True)))
        self.frames = 0
        self.video_hash = hashlib.sha256()
        self.audio_hash = hashlib.sha256()
        self.samples = 0
        self.callbacks = (ENV(self.environment), VIDEO(self.video), BATCH(self.audio),
                          POLL(lambda: None), INPUT(lambda *_: 0))
        for name, callback in zip(('environment', 'video_refresh', 'audio_sample_batch',
                                   'input_poll', 'input_state'), self.callbacks):
            setter = getattr(self.lib, 'retro_set_' + name)
            setter.argtypes = [type(callback)]
            setter(callback)
        self.lib.retro_load_game.argtypes = [c.POINTER(GameInfo)]
        self.lib.retro_load_game.restype = c.c_bool
        self.lib.retro_serialize_size.restype = c.c_size_t
        for name in ('serialize', 'unserialize'):
            method = getattr(self.lib, 'retro_' + name)
            method.argtypes = [c.c_void_p, c.c_size_t]
            method.restype = c.c_bool
        self.lib.retro_get_memory_size.argtypes = [c.c_uint]
        self.lib.retro_get_memory_size.restype = c.c_size_t
        self.lib.retro_get_memory_data.argtypes = [c.c_uint]
        self.lib.retro_get_memory_data.restype = c.c_void_p
        self.lib.retro_init()

    def environment(self, command, data):
        if command == 15:
            option = c.cast(data, c.POINTER(Variable)).contents
            option.value = b'software' if option.key == b'tgpulse_next_renderer' else None
            return option.value is not None
        if command == 52:
            c.cast(data, c.POINTER(c.c_uint))[0] = 2
            return True
        return command in (10, 11, 16, 18, 35, 37, 67, 69)

    def video(self, data, width, height, pitch):
        assert data and 1 <= width <= 1024 and 1 <= height <= 1024
        self.frames += 1
        self.video_hash.update(c.string_at(data, height * pitch))

    def audio(self, data, frames):
        self.samples += frames
        self.audio_hash.update(c.string_at(data, frames * 4))
        return frames

    def load(self, path):
        assert self.lib.retro_load_game(c.byref(GameInfo(str(path.resolve(strict=True)).encode(), None, 0, None)))

    def run(self, frames):
        self.frames = self.samples = 0
        self.video_hash = hashlib.sha256()
        self.audio_hash = hashlib.sha256()
        for _ in range(frames):
            self.lib.retro_run()
        assert self.frames == frames and self.samples > 0
        return {'frames': self.frames, 'audio_frames': self.samples,
                'video_sha256': self.video_hash.hexdigest(),
                'audio_sha256': self.audio_hash.hexdigest()}

    def snapshot(self):
        size = self.lib.retro_serialize_size()
        assert size > 0
        data = c.create_string_buffer(size)
        assert self.lib.retro_serialize(data, size)
        return data.raw

    def restore(self, data):
        buffer = c.create_string_buffer(data, len(data))
        assert self.lib.retro_unserialize(buffer, len(data))

    def reject_state(self, data):
        before = self.snapshot()
        buffer = c.create_string_buffer(data, len(data))
        assert not self.lib.retro_unserialize(buffer, len(data)), 'Legacy state was accepted'
        assert self.snapshot() == before, 'Rejected state changed the machine'

    def save_ram(self):
        size = self.lib.retro_get_memory_size(0)
        assert size == 65728
        return c.string_at(self.lib.retro_get_memory_data(0), size)

    def close(self):
        self.lib.retro_unload_game()
        self.lib.retro_deinit()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('baseline-core', 'core', 'rom-dir', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--frames', type=int, default=120)
    parser.add_argument('--state-policy', choices=('identical', 'format-1-to-4', 'format-1-to-5'), default='identical',
                        help='Explicitly verify an approved incompatible machine-state update')
    args = parser.parse_args()
    if not 1 <= args.frames <= 600 or args.output.exists():
        parser.error('Use a new output file and 1–600 frames')
    results = []
    for name in SETS:
        baseline = Host(args.baseline_core)
        candidate = Host(args.core)
        try:
            path = args.rom_dir / (name + '.zip')
            baseline.load(path)
            candidate.load(path)
            expected = baseline.run(args.frames)
            assert candidate.run(args.frames) == expected, (name, 'video/audio mismatch')
            snapshot = baseline.snapshot()
            current = candidate.snapshot()
            if args.state_policy == 'identical':
                assert current == snapshot, (name, 'machine snapshot mismatch')
                candidate.restore(snapshot)
            else:
                version = 4 if args.state_policy == 'format-1-to-4' else 5
                for saved, version in ((snapshot, 1), (current, version)):
                    assert saved[32:40] == b'TGP1STAT'
                    assert struct.unpack_from('<I', saved, 40)[0] == version
                candidate.reject_state(snapshot)
                candidate.restore(current)
            assert candidate.save_ram() == baseline.save_ram(), (name, 'Save RAM mismatch')
            continuation = baseline.run(5)
            assert candidate.run(5) == continuation, (name, 'pre-split state continuation mismatch')
            continued = candidate.snapshot()
            if args.state_policy == 'identical':
                assert continued == baseline.snapshot(), (name, 'continued state mismatch')
            else:
                candidate.restore(current)
                assert candidate.run(5) == continuation, (name, 'new state continuation mismatch')
                assert candidate.snapshot() == continued, (name, 'new state replay mismatch')
            result = {'set': name, **expected, 'state_sha256': hashlib.sha256(snapshot).hexdigest(),
                      'current_state_sha256': hashlib.sha256(current).hexdigest(),
                      'save_ram_sha256': hashlib.sha256(candidate.save_ram()).hexdigest(),
                      'state_policy': args.state_policy,
                      'pre_split_state_continuation': args.state_policy == 'identical',
                      'legacy_state_rejected_atomically': args.state_policy != 'identical',
                      'new_state_continuation': True}
            results.append(result)
            print('PASS:', name, 'frames/audio/Save RAM and', args.state_policy, 'state continuation', flush=True)
        finally:
            candidate.close()
            baseline.close()
    report = {'sets': results, 'frames_per_set': args.frames,
              'baseline_core_sha256': hashlib.sha256(args.baseline_core.read_bytes()).hexdigest(),
              'core_sha256': hashlib.sha256(args.core.read_bytes()).hexdigest()}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
