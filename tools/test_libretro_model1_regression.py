#!/usr/bin/env python3
"""Compare published Model 1 cores with the established isolated ABI hosts.

Extends the nine-set scope runner with NetMerc in both City Workaround modes.
No personal frontend configuration or saves are accessed. Results are bounded
emulation comparisons, not physical-controller acceptance or minimum FPS.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import shutil

from test_libretro_netmerc_audio import AudioHost


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('baseline-core', 'core', 'rom-dir', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--frames', type=int, default=600)
    parser.add_argument('--bios-dir', type=Path, help='Existing device BIOS ZIPs copied into isolated system')
    parser.add_argument('--netmerc-only', action='store_true', help='Resume NetMerc after an already completed nine-set comparison')
    args = parser.parse_args()
    if not 1 <= args.frames <= 600:
        parser.error('Use 1–600 frames')
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    command = [sys.executable, str(Path(__file__).with_name('test_libretro_model1_scope.py')),
               '--baseline-core', str(args.baseline_core.resolve(strict=True)),
               '--core', str(args.core.resolve(strict=True)),
               '--rom-dir', str(args.rom_dir.resolve(strict=True)),
               '--output', str(out / 'nine-sets.json'), '--frames', str(args.frames)]
    if not args.netmerc_only:
        subprocess.run(command, check=True)
    system = out / 'system'
    system.mkdir()
    if args.bios_dir:
        destination = system / 'tgpulse-next'
        destination.mkdir()
        for name in ('model1io.zip', 'model1io2.zip', 'hd44780.zip'):
            source = args.bios_dir / name
            if source.is_file():
                shutil.copyfile(source, destination / name)
    results = []
    for mode in ('enabled', 'disabled'):
        hosts = []
        try:
            for core in (args.baseline_core, args.core):
                host = AudioHost(core, system)
                hosts.append(host)
                host.values[b'tgpulse_next_netmerc_city_workaround'] = mode.encode()
                host.load(args.rom_dir / 'netmerc.zip')
            baseline, candidate = hosts
            expected = baseline.run(args.frames)
            assert candidate.run(args.frames) == expected, (mode, 'video/PCM')
            state = baseline.snapshot()
            assert candidate.snapshot() == state, (mode, 'full state')
            save_ram = baseline.save_ram()
            assert candidate.save_ram() == save_ram, (mode, 'Save RAM')
            candidate.restore(state)
            continuation = baseline.run(10)
            assert candidate.run(10) == continuation, (mode, 'restored continuation')
            assert candidate.snapshot() == baseline.snapshot(), (mode, 'continued state')
            results.append({'set': 'netmerc', 'city_workaround': mode, **expected,
                            'state_sha256': hashlib.sha256(state).hexdigest(),
                            'save_ram_sha256': hashlib.sha256(save_ram).hexdigest(),
                            'continuation': continuation, 'continuation_equal': True})
            print('PASS: NetMerc', mode, 'video/PCM/Save RAM/state/continuation', flush=True)
        finally:
            for host in reversed(hosts):
                host.close()
    report = (json.loads((out / 'nine-sets.json').read_text()) if not args.netmerc_only else
              {'sets': [], 'frames_per_set': args.frames,
               'baseline_core_sha256': hashlib.sha256(args.baseline_core.read_bytes()).hexdigest(),
               'core_sha256': hashlib.sha256(args.core.read_bytes()).hexdigest()})
    report['netmerc'] = results
    (out / 'comparison.json').write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
