#!/usr/bin/env python3
"""Optionally batch-migrate old textual source Gain mutes to Libretro value 0.

The core accepts old values without this tool. Inspect by default. Use --apply
only with RetroArch closed; changed files get
an untouched .bak copy before an atomic replacement. Only listed source Gain
keys and exact old values are changed. Other Core Options are preserved.
"""

import argparse
import os
from pathlib import Path
import re
import shutil
import stat
import tempfile


GAIN_KEYS = (
    "tgpulse_next_multipcm1_gain",
    "tgpulse_next_multipcm2_gain",
    "tgpulse_next_ym3438_gain",
    "tgpulse_next_dsb_gain",
    "sm2_scsp_gain",
    "sm2_dsb_gain",
    "sm2_multipcm1_gain",
    "sm2_multipcm2_gain",
    "sm2_ym3438_gain",
)
KEY_PATTERN = b"|".join(re.escape(key.encode("ascii")) for key in GAIN_KEYS)
OLD_MUTE = re.compile(
    rb"(?m)^(?P<prefix>[ \t]*(?:" + KEY_PATTERN
    + rb")[ \t]*=[ \t]*)(?P<quote>[\"'])(?:mute|Mute)(?P=quote)"
)


def migrated(data: bytes) -> tuple[bytes, int]:
    return OLD_MUTE.subn(lambda match: match["prefix"] + match["quote"]
                         + b"0" + match["quote"], data)


def candidates(paths: list[Path]) -> list[Path]:
    found = set()
    for path in paths:
        if path.is_symlink():
            raise ValueError(f"refusing symbolic link: {path}")
        if path.is_file():
            found.add(path)
        elif path.is_dir():
            found.update(child for child in path.rglob("*.opt")
                         if child.is_file() and not child.is_symlink())
            found.update(child for child in path.rglob("retroarch-core-options.cfg")
                         if child.is_file() and not child.is_symlink())
        else:
            raise ValueError(f"path does not exist: {path}")
    return sorted(found)


def replace_with_backup(path: Path, data: bytes) -> None:
    backup = path.with_name(path.name + ".bak")
    if backup.exists():
        raise FileExistsError(f"backup already exists: {backup}")
    original_mode = stat.S_IMODE(path.stat().st_mode)
    shutil.copy2(path, backup)
    fd, temporary = tempfile.mkstemp(prefix=f".{path.name}.migrate-", dir=path.parent)
    try:
        with os.fdopen(fd, "wb") as output:
            os.fchmod(output.fileno(), original_mode)
            output.write(data)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="+", type=Path,
                        help="Core Options .opt files or directories to inspect")
    parser.add_argument("--apply", action="store_true",
                        help="write changes after creating .bak copies")
    args = parser.parse_args()
    try:
        pending = []
        for path in candidates(args.paths):
            updated, count = migrated(path.read_bytes())
            if count:
                backup = path.with_name(path.name + ".bak")
                if args.apply and backup.exists():
                    raise FileExistsError(f"backup already exists: {backup}")
                pending.append((path, updated, count))
        for path, updated, count in pending:
            if args.apply:
                replace_with_backup(path, updated)
            print(f"{path}: {count} value(s) {'migrated' if args.apply else 'to migrate'}")
        print(f"Total: {sum(count for _, _, count in pending)} value(s)"
              + (" migrated" if args.apply else " to migrate (dry run)"))
    except (OSError, ValueError) as error:
        parser.exit(1, f"error: {error}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
