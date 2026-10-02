#!/usr/bin/env python3
"""Audit VF's isolated saved values and fresh-load diagnostic screenshots."""
import argparse
import hashlib
import json
import pathlib
import tomllib

from audit_model1_game_system_campaign import pixels
from libretro_nvram_capture import validate_save


def vf_eeprom_crc(eeprom):
    # The game starts after its bookkeeping word at 0x0a. EEPROM words are
    # stored little-endian, but the checksum traverses each high byte first.
    crc = 0
    for index in range(0x0c, 0x80, 2):
        for byte in (eeprom[index + 1], eeprom[index]):
            crc ^= byte << 8
            for _ in range(8):
                crc = ((crc << 1) ^ (0x1021 if crc & 0x8000 else 0)) & 0xffff
    return crc


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--recipes', type=pathlib.Path, required=True)
    parser.add_argument('--samples', type=pathlib.Path, required=True)
    parser.add_argument('--reloads', type=pathlib.Path, required=True)
    args = parser.parse_args()
    root = pathlib.Path(__file__).resolve().parents[1]
    recipes = sorted(args.recipes.glob('vf--*--step-*.toml'))
    errors, acquired, reloaded = [], 0, 0
    if len(recipes) != 98:
        errors.append(f'Expected 98 recipes, found {len(recipes)}')
    for path in recipes:
        try:
            recipe = tomllib.loads(path.read_text())
            if recipe['set'] != 'vf' or recipe['value_step'] < 0:
                raise ValueError('Unexpected recipe metadata')
            field, step = recipe['field'], recipe['value_step']
            evidence = root / 'docs/diagnostic-evidence/vf' / f"{field.replace('_', '-')}-{step}.png"
            sample = args.samples / path.stem
            manifest = json.loads((sample / 'manifest.json').read_text())
            if manifest['status'] != 'complete' or manifest['recipe'] != recipe:
                raise ValueError('Capture manifest differs from recipe')
            saved = (sample / 'saved.srm').read_bytes()
            validate_save(saved, 'vf')
            eeprom = saved[64 + 65536:]
            if int.from_bytes(eeprom[8:10], 'big') != vf_eeprom_crc(eeprom):
                raise ValueError('Native VF EEPROM CRC mismatch')
            if pixels(sample / 'selected.png') != pixels(evidence):
                raise ValueError('Selected value differs from catalogue image')
            acquired += 1
            reload = args.reloads / path.stem
            check = json.loads((reload / 'manifest.json').read_text())
            verify_recipe = tomllib.loads((args.recipes / f'vf--verify--{field}.toml').read_text())
            if check['status'] != 'complete' or check['recipe'] != verify_recipe:
                raise ValueError('Fresh-load capture differs from verification recipe')
            with (sample / 'saved.srm').open('rb') as stream:
                saved_hash = hashlib.file_digest(stream, 'sha256').hexdigest()
            if check.get('import_sha256') != saved_hash:
                raise ValueError('Fresh load imported another Save RAM')
            if pixels(reload / 'reloaded.png') != pixels(evidence):
                raise ValueError('Fresh-load menu differs from catalogue image')
            reloaded += 1
        except (FileNotFoundError, KeyError, ValueError) as error:
            errors.append(f'{path.name}: {error}')
    print(json.dumps({'set': 'vf', 'recipes': len(recipes), 'acquired': acquired,
                      'reloaded': reloaded, 'errors': errors}, indent=2))
    if errors:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
