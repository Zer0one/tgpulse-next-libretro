#!/usr/bin/env python3
"""Trace NetMerc startup pose writes using already-built native Rust libraries.

ROMs are read only. Creates a fresh evidence directory and a temporary probe;
never installs dependencies or reads/writes personal saves or controller devices.
"""
import argparse
import json
from pathlib import Path
import subprocess


def main():
    root = Path(__file__).resolve().parents[1]
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--rom', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    p.add_argument('--deps', type=Path, default=root/'target/release/deps',
                   help='Existing release dependency directory, optionally from standalone')
    p.add_argument('--verify-bootstrap', action='store_true',
                   help='Verify neutral startup, live/zero poses and machine-state replay')
    a = p.parse_args()
    a.output.mkdir(parents=True, exist_ok=False)
    deps = a.deps.resolve(strict=True)
    def library(name):
        candidates = list(deps.glob('lib'+name+'-*.rlib'))
        if not candidates:
            p.error('Missing built library: '+name+'; build the existing project first')
        return max(candidates, key=lambda f: f.stat().st_mtime)
    core = library('tgpulse_core')
    codecs = sorted(deps.glob('libbincode-*.rlib'),key=lambda f:f.stat().st_mtime,reverse=True)
    if not codecs:
        p.error('Missing built library: bincode')
    binary = a.output.resolve()/'probe'
    source = root/'tools/trace_netmerc_camera.rs'
    # Existing standalone caches may contain multiple dependency graphs.
    # Select a codec that links to the same Serde instance as the core.
    for codec in codecs:
        result = subprocess.run(['rustc','--edition=2021','-L','dependency='+str(deps),
                    '--extern','tgpulse_core='+str(core),'--extern','bincode='+str(codec),
                    str(source),'-O','-o',str(binary)],capture_output=True,text=True)
        if result.returncode == 0:
            break
        if 'multiple different versions' not in result.stderr:
            raise RuntimeError(result.stderr)
    else:
        raise RuntimeError(result.stderr)
    with (a.output/'trace.txt').open('w') as f:
        command = [str(binary), str(a.rom.resolve(strict=True))]
        if a.verify_bootstrap:
            command.append('--verify-bootstrap')
        subprocess.run(command, stdout=f, check=True)
    result = {'core_library':str(core),'snapshot_codec':str(codec),
              'bootstrap_verified': a.verify_bootstrap,
              'physical_gameplay_acceptance':False,'trace':str(a.output/'trace.txt')}
    (a.output/'report.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))

if __name__ == '__main__':
    main()
