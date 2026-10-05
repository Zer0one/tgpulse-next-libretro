#!/usr/bin/env python3
"""Install a verified macOS build under the user's public RetroArch name.

The historical script filename is retained for existing documented commands.
Back up replaced public files and retire the former Development copy, verify
both installed files, and refresh metadata discovery on the next launch.
"""
import argparse
import hashlib
import json
from pathlib import Path
import tempfile
import os
import time
import shutil

OLD_CORE_NAME = 'TGPulse-Next: Model 1'
CORE_NAME = 'TGPulse-Next - Model 1'


def copy_identity_directory(base):
    """Preserve originals and existing destination files during a name change."""
    source = base / OLD_CORE_NAME
    destination = base / CORE_NAME
    copied, conflicts = [], []
    if not source.is_dir():
        return None
    for old in sorted(source.rglob('*')):
        if old.is_symlink() or not old.is_file():
            continue
        relative = old.relative_to(source)
        if relative.name.startswith(OLD_CORE_NAME + '.'):
            relative = relative.with_name(CORE_NAME + relative.name[len(OLD_CORE_NAME):])
        new = destination / relative
        if new.exists():
            if new.read_bytes() != old.read_bytes():
                conflicts.append(str(new))
            continue
        new.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(old, new)
        assert new.read_bytes() == old.read_bytes()
        copied.append(str(new))
    return {'source': str(source), 'destination': str(destination),
            'copied': copied, 'preserved_destination_conflicts': conflicts}


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
    p.add_argument('--core',type=Path,default=root/'tgpulse_next_m1_libretro.dylib')
    p.add_argument('--retroarch-root',type=Path,default=Path.home()/'Library/Application Support/RetroArch')
    p.add_argument('--data-root',type=Path,default=Path.home()/'Documents/RetroArch',
                   help='Default local save/state root; custom layouts need an equivalent manual copy')
    a=p.parse_args();core=a.core.resolve(strict=True)
    info=(root/'tgpulse_next_m1_libretro.info').read_bytes()
    target=a.retroarch_root/'cores/tgpulse_next_m1_libretro.dylib';metadata=a.retroarch_root/'info/tgpulse_next_m1_libretro.info'
    backup=a.retroarch_root/'backups'/('tgpulse-public-install-'+str(time.time_ns()))
    backup.mkdir(parents=True)
    for old in (target,metadata):
        if old.exists():shutil.copy2(old,backup/old.name)
    data=core.read_bytes();result={'core':str(target),'core_sha256':install(data,target),'info':str(metadata),'info_sha256':install(info,metadata),'public_names':True,'backup':str(backup)}
    for old in (a.retroarch_root/'cores/tgpulse_next_dev_m1_libretro.dylib',a.retroarch_root/'info/tgpulse_next_dev_m1_libretro.info'):
        if old.exists():old.rename(backup/old.name)
    bases = (a.retroarch_root/'config', a.retroarch_root/'config/remaps',
             a.retroarch_root/'saves', a.retroarch_root/'states',
             a.data_root/'saves', a.data_root/'states')
    result['identity_directory_copies'] = [receipt for base in bases
        if (receipt := copy_identity_directory(base)) is not None]
    cache=a.retroarch_root/'info/core_info.cache'
    if cache.exists():
        backup=cache.with_name('core_info.cache.before-tgpulse-public-'+str(time.time_ns()))
        cache.rename(backup)
        result['metadata_cache_backup']=str(backup)
    print(json.dumps(result,indent=2))

if __name__=='__main__':main()
