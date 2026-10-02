#!/usr/bin/env python3
"""Install a verified macOS build under the agreed RetroArch Development name.

Adapts the reference projects' core/info placement. Writes only the two named
Development artifacts and verifies both copies. Run after a verified build.
"""
import argparse
import hashlib
import json
from pathlib import Path
import tempfile
import os


def install(data, destination):
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(dir=destination.parent, prefix='.'+destination.name+'.', delete=False) as f:
        temporary=Path(f.name)
        try:
            f.write(data);f.flush();os.fsync(f.fileno())
            os.chmod(temporary,0o644);os.replace(temporary,destination)
        finally:
            temporary.unlink(missing_ok=True)
    sha=lambda b:hashlib.sha256(b).hexdigest()
    assert sha(destination.read_bytes())==sha(data), 'Installed artifact mismatch'
    return sha(data)


def main():
    p=argparse.ArgumentParser(description=__doc__)
    root=Path(__file__).resolve().parents[1]
    p.add_argument('--core',type=Path,default=root/'target/release/libtgpulse_next_m1_libretro.dylib')
    p.add_argument('--retroarch-root',type=Path,default=Path.home()/'Library/Application Support/RetroArch')
    a=p.parse_args();core=a.core.resolve(strict=True)
    info=(root/'tgpulse_next_m1_libretro.info').read_bytes()
    target=a.retroarch_root/'cores/tgpulse_next_dev_m1_libretro.dylib';metadata=a.retroarch_root/'info/tgpulse_next_dev_m1_libretro.info'
    data=core.read_bytes();result={'core':str(target),'core_sha256':install(data,target),'info':str(metadata),'info_sha256':install(info,metadata),'development_names':True}
    print(json.dumps(result,indent=2))

if __name__=='__main__':main()
