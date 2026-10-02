#!/usr/bin/env python3
"""Audit isolated Wing War full-axis calibration and its fresh import."""
import hashlib
import json
import pathlib
import tomllib

from audit_model1_assignment_campaign import native_crc
from libretro_nvram_capture import validate_save

ROOT = pathlib.Path(__file__).resolve().parents[1]
BASE = ROOT / 'validation/nvram-campaigns/2026-10-01'
SETS = ('wingwar', 'wingwaru', 'wingwarj', 'wingwar360')


def main():
    errors = []
    for set_name in SETS:
        try:
            folder = BASE / set_name / 'calibration'
            sample, reload = folder / 'sample', folder / 'reload'
            recipe = tomllib.loads((ROOT / f'tools/nvram-recipes/{set_name}-volume-calibration.toml').read_text())
            check_recipe = tomllib.loads((ROOT / f'tools/nvram-recipes/{set_name}-volume-calibration-reload.toml').read_text())
            manifest = json.loads((sample / 'manifest.json').read_text())
            check = json.loads((reload / 'manifest.json').read_text())
            if manifest['status'] != 'complete' or manifest['recipe'] != recipe:
                raise ValueError('Calibration recipe or manifest mismatch')
            if check['status'] != 'complete' or check['recipe'] != check_recipe:
                raise ValueError('Reload recipe or manifest mismatch')
            before = (sample / 'before.srm').read_bytes()
            committed = (sample / 'committed.srm').read_bytes()
            fresh = (reload / 'volume-default.srm').read_bytes()
            for raw in (before, committed, fresh):
                validate_save(raw, set_name)
                eeprom = raw[64 + 65536:]
                if not native_crc(eeprom, set_name) or eeprom[6:60] != eeprom[66:120]:
                    raise ValueError('Native EEPROM integrity mismatch')
            if check.get('import_sha256') != hashlib.sha256(committed).hexdigest():
                raise ValueError('Fresh load imported another sample')
            if committed[64 + 65536:] != fresh[64 + 65536:]:
                raise ValueError('Calibration EEPROM changed on fresh load')
            if committed[64 + 65536 + 0x1e:64 + 65536 + 0x24] != bytes((0, 255, 0, 255, 1, 255)):
                raise ValueError('Full-range calibration endpoints differ')
            print(f'{set_name}: full-range calibration persisted, native integrity valid')
        except (FileNotFoundError, KeyError, ValueError) as error:
            errors.append(f'{set_name}: {error}')
    if errors:
        raise SystemExit('\n'.join(errors))


if __name__ == '__main__':
    main()
