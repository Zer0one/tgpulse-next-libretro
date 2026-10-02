#!/usr/bin/env python3
"""Audit isolated SWA / Wing War values against screenshots and fresh imports."""
import argparse
import hashlib
import json
import pathlib
import tomllib

from audit_model1_game_system_campaign import pixels
from libretro_nvram_capture import validate_save

COUNTS = {'swa': 92, 'swaj': 90, 'wingwar': 134, 'wingwaru': 134,
          'wingwarj': 134, 'wingwar360': 149}


def native_crc(eeprom, set_name):
    # Empirically verified against independently committed values in each
    # family. The game checks only the first 66 EEPROM bytes in these sets.
    if set_name.startswith('swa'):
        crc = 0x5a81
        data = bytes(byte for index in range(0x0a, 0x42, 2)
                     for byte in (eeprom[index + 1], eeprom[index]))
        stored = int.from_bytes(eeprom[8:10], 'big')
    else:
        crc = 0x3a19
        data = eeprom[0x0a:0x42]
        stored = int.from_bytes(eeprom[8:10], 'little')
    for byte in data:
        crc ^= byte << 8
        for _ in range(8):
            crc = ((crc << 1) ^ (0x1021 if crc & 0x8000 else 0)) & 0xffff
    return crc == stored


def evidence(root, set_name, field, step):
    if field == 'network':
        filename = ('network-0', 'network-master', 'network-slave')[step] + '.png'
    else:
        filename = f"{field.replace('_', '-')}-{step}.png"
    return root / 'docs/diagnostic-evidence' / set_name / filename


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--set', choices=tuple(COUNTS), required=True)
    parser.add_argument('--recipes', type=pathlib.Path, required=True)
    parser.add_argument('--samples', type=pathlib.Path, required=True)
    parser.add_argument('--reloads', type=pathlib.Path, required=True)
    args = parser.parse_args()
    root = pathlib.Path(__file__).resolve().parents[1]
    recipes = sorted(args.recipes.glob(f'{args.set}--*--step-*.toml'))
    errors, acquired, reloaded = [], 0, 0
    if len(recipes) != COUNTS[args.set]:
        errors.append(f'Expected {COUNTS[args.set]} recipes, found {len(recipes)}')
    for path in recipes:
        try:
            recipe = tomllib.loads(path.read_text())
            if recipe['set'] != args.set or recipe['value_step'] < 0:
                raise ValueError('Unexpected recipe metadata')
            field, step = recipe['field'], recipe['value_step']
            target = evidence(root, args.set, field, step)
            sample = args.samples / path.stem
            manifest = json.loads((sample / 'manifest.json').read_text())
            if manifest['status'] != 'complete' or manifest['recipe'] != recipe:
                raise ValueError('Capture manifest differs from recipe')
            saved = (sample / 'saved.srm').read_bytes()
            validate_save(saved, args.set)
            eeprom = saved[64 + 65536:]
            if not native_crc(eeprom, args.set):
                raise ValueError('Native EEPROM CRC mismatch or uninitialized default')
            if eeprom[6:60] != eeprom[66:120]:
                raise ValueError('Native EEPROM mirror differs from primary settings block')
            if pixels(sample / 'selected.png') != pixels(target):
                raise ValueError('Selected value differs from catalogue image')
            acquired += 1
            reload = args.reloads / path.stem
            check = json.loads((reload / 'manifest.json').read_text())
            verify_recipe = tomllib.loads((args.recipes / f'{args.set}--verify--{field}.toml').read_text())
            if check['status'] != 'complete' or check['recipe'] != verify_recipe:
                raise ValueError('Fresh-load capture differs from verification recipe')
            with (sample / 'saved.srm').open('rb') as stream:
                saved_hash = hashlib.file_digest(stream, 'sha256').hexdigest()
            if check.get('import_sha256') != saved_hash:
                raise ValueError('Fresh load imported another Save RAM')
            if pixels(reload / 'reloaded.png') != pixels(target):
                raise ValueError('Fresh-load menu differs from catalogue image')
            reloaded += 1
        except (FileNotFoundError, KeyError, ValueError, IndexError) as error:
            errors.append(f'{path.name}: {error}')
    print(json.dumps({'set': args.set, 'recipes': len(recipes), 'acquired': acquired,
                      'reloaded': reloaded, 'errors': errors}, indent=2))
    if errors:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
