#!/usr/bin/env python3
"""Generate reviewed Model 1 option maps/templates from existing campaign evidence.

Adapts SM2's generated initial_nvram_templates and reviewed field table. Raw
Save RAM stays ignored; only derived, validated RLE payloads enter the adapter.
Uses the project's existing Ruby YAML reader and Python standard library.
"""
import argparse
import hashlib
import json
import re
from pathlib import Path
import subprocess
import xml.etree.ElementTree as ET
import zipfile
from libretro_nvram_capture import validate_save

ROOT = Path(__file__).resolve().parents[1]
NS = {'m': 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}


def workbook_rows(path):
    with zipfile.ZipFile(path) as z:
        strings = []
        if 'xl/sharedStrings.xml' in z.namelist():
            strings = [''.join(n.itertext()) for n in ET.fromstring(z.read('xl/sharedStrings.xml')).findall('m:si', NS)]
        rels = {r.get('Id'): r.get('Target') for r in ET.fromstring(z.read('xl/_rels/workbook.xml.rels'))}
        result = {}
        for sheet in ET.fromstring(z.read('xl/workbook.xml')).findall('m:sheets/m:sheet', NS):
            target = rels[sheet.get('{http://schemas.openxmlformats.org/officeDocument/2006/relationships}id')]
            target = target.lstrip('/') if target.startswith('/') else 'xl/' + target
            rows = []
            for row in ET.fromstring(z.read(target)).findall('m:sheetData/m:row', NS):
                cells = {}
                for c in row.findall('m:c', NS):
                    value = c.find('m:v', NS)
                    text = '' if value is None else value.text or ''
                    if c.get('t') == 's': text = strings[int(text)]
                    elif c.get('t') == 'inlineStr': text = ''.join(c.find('m:is', NS).itertext())
                    cells[c.get('r').rstrip('0123456789')] = text
                rows.append(cells)
            result[sheet.get('name')] = rows
        return result


def workbook_records(sheets, name):
    rows = sheets[name]
    headers = {column: value.casefold() for column, value in rows[0].items() if value}
    if len(set(headers.values())) != len(headers): raise ValueError('Duplicate workbook headers')
    return [{header: row.get(column, '') for column, header in headers.items()} for row in rows[1:]]


# Adapt SM2 retroarch_option_label; preserve domain acronyms and numeric tokens.
ACRONYMS = set("BGM CRT DX EUR EXP GT ID JPN NG OK PK SD SP TT URL US USA VJCOM VJMAN VS NVRAM EEPROM CPU GPU FM DSB MPEG LR FR".split())

def display_label(value):
    return re.sub(r"[A-Za-z0-9]+", lambda m: m[0] if any(c.isdigit() for c in m[0]) or m[0].upper() in ACRONYMS else m[0].capitalize(), str(value))


def reviewed_option(options, label):
    matches = [o for o in options if o['label'].casefold() == label.casefold()]
    if len(matches) != 1: raise ValueError(f'Ambiguous or unknown reviewed setting: {label}')
    return matches[0]


def startup_values(options, text):
    result = {}
    if ' = ' not in text: return result
    for entry in text.split('; '):
        label, value = entry.split(' = ', 1)
        option = reviewed_option(options, label)
        matches = [v for v in option['observed_values'] if str(v).casefold() == value.casefold()]
        if len(matches) != 1: raise ValueError(f'Ambiguous startup value: {entry}')
        result[option['label']] = str(matches[0])
    return result


def crc(data, initial=0):
    value = initial
    for byte in data:
        value ^= byte << 8
        for _ in range(8): value = ((value << 1) ^ (0x1021 if value & 0x8000 else 0)) & 0xffff
    return value


def number(value):
    return int(value, 0) if isinstance(value, str) else value


def layout(doc):
    spec = doc['nvram_layout']['integrity']
    return (number(spec['covered_offsets'][0]), number(spec['covered_offsets'][1]) + 1,
            number(spec['initial_value']), spec['covered_byte_order'].startswith('high_byte'),
            spec['stored_byte_order'] == 'big_endian', 'mirror' in doc['nvram_layout'])


def repair(eeprom, spec):
    start, end, initial, swap, big, mirror = spec
    data = eeprom[start:end]
    if swap: data = bytes(byte for i in range(0, len(data), 2) for byte in (data[i+1], data[i]))
    eeprom[8:10] = crc(data, initial).to_bytes(2, 'big' if big else 'little')
    if mirror: eeprom[66:120] = eeprom[6:60]


def validate_native(eeprom, spec):
    fixed = bytearray(eeprom); repair(fixed, spec)
    if fixed != eeprom: raise ValueError('Native checksum/mirror mismatch')


def patches(option, step):
    spec = option['nvram']
    offsets = spec.get('backup_offsets', spec.get('eeprom_offsets', [spec.get('eeprom_offset')]))
    values = spec['values'][step]
    if not isinstance(values, list): values = [values]
    if len(offsets) != len(values): raise ValueError('Offset/value count differs')
    return [(number(offset), number(value)) for offset, value in zip(offsets, values)]


def encoded_rle(data):
    # Same repeat/literal command format as SM2 initial_nvram_templates.
    out = []; position = 0
    while position < len(data):
        count = 1
        while position + count < len(data) and count < 128 and data[position+count] == data[position]: count += 1
        if count >= 3:
            out.extend([0x80 | (count-1), data[position]]); position += count
        else:
            start = position; position += count
            while position < len(data) and position-start < 128:
                run = 1
                while position+run < len(data) and run < 3 and data[position+run] == data[position]: run += 1
                if run >= 3: break
                position += min(run, 128-(position-start))
            out.append(position-start-1); out.extend(data[start:position])
    return out


def cstr(value):
    return 'c' + json.dumps(str(value), ensure_ascii=False)


def array(values):
    return '&[' + ', '.join(str(v) for v in values) + ']'


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--samples', type=Path, default=ROOT/'validation/nvram-campaigns/2026-10-01')
    p.add_argument('--workbook', type=Path, default=ROOT/'docs/model1_core_options_review.xlsx')
    p.add_argument('--output', type=Path, default=ROOT/'crates/tgpulse-libretro/src/nvram_data.rs')
    p.add_argument('--check', action='store_true')
    args = p.parse_args()
    sheets = workbook_rows(args.workbook)
    selected = {}
    for row in workbook_records(sheets, 'Core options'):
        if row['proposed nvram settings'] == 'Yes':
            if row['reviewer decision'] != 'Approved': raise ValueError('Unreviewed option')
            selected.setdefault(row['set'], []).append(row['setting'])
    setups = {row['set']: row for row in workbook_records(sheets, 'Automatic setup') if row['reviewer decision'] == 'Approved'}
    if set(selected) != set(setups) or len(setups) != 10: raise ValueError('Reviewed set coverage differs')
    yaml_paths = [str(ROOT/f'data/diagnostic-menus/{name}.yaml') for name in setups]
    ruby = 'require "yaml"; require "json"; puts JSON.generate(ARGV.map { |p| YAML.safe_load(File.read(p), permitted_classes: [], aliases: true) })'
    docs = json.loads(subprocess.check_output(['ruby', '-e', ruby, *yaml_paths], text=True))
    output = ['// Generated by tools/generate_model1_nvram.py; do not edit.',
              '// Reviewed workbook + independently committed/reloaded samples; no ROM data.',
              'use super::{BackupTemplate, Field, Layout, Template, Value};', '#[rustfmt::skip]', 'pub static FIELDS: &[Field] = &[']
    templates = []; backup_templates = []; count = 0
    for doc in docs:
        name = doc['game']['set']; backup_storage = doc['nvram_layout']['storage'] == 'backup_ram'
        spec = None if backup_storage else layout(doc)
        options = [o for menu in doc['menus'].values() for o in menu.get('options', [])]
        review = doc['core_options_review']
        selected_options = [reviewed_option(options, label) for label in selected[name]]
        if selected_options != [o for o in options if o in selected_options]: raise ValueError(f'{name}: workbook menu order differs')
        approved_keys = [o['key'] for o in selected_options]
        startup = startup_values(options, setups[name]['explicit startup values'])
        if review['nvram_settings'] != approved_keys or review['automatic_startup_values'] != startup:
            raise ValueError(f'{name}: YAML and approved workbook differ')
        baseline = None; baseline_path = None
        # Reconcile the complete campaign, including fields excluded from options.
        for option in options:
            for step, label in enumerate(option['observed_values']):
                path = (ROOT/'validation/nvram-campaigns/2026-10-04/netmerc/samples'/f'{option["key"]}-{step}'/'saved.srm'
                        if backup_storage else args.samples/name/'samples'/f'{name}--{option["key"]}--step-{step:02d}'/'saved.srm')
                raw = path.read_bytes(); validate_save(raw, name)
                image = raw[64:65600] if backup_storage else raw[65600:]
                if not backup_storage: validate_native(image, spec)
                for offset, value in patches(option, step):
                    if image[offset] != value: raise ValueError(f'{path}: YAML encoding differs')
                if step == 0:
                    if baseline is None: baseline, baseline_path = raw, path
                    elif not backup_storage and image != baseline[65600:]: raise ValueError('Set default EEPROM differs across fields')
                count += 1
        for option in selected_options:
            label = display_label(option['label'])
            # Match the approved automatic policy, as in SM2/Supermodel. The
            # catalogue/workbook native default remains documentary evidence.
            default_value = startup.get(option['label'], str(option['native_default']))
            default = next(i for i, value in enumerate(option['observed_values'])
                           if str(value) == default_value)
            output.append(f'Field {{ set: "{name}", key: {cstr("tgpulse_next_nvram_"+name+"_"+option["key"])}, label: {cstr(label)}, default: {default}, values: &[')
            for step, value in enumerate(option['observed_values']):
                label_text = display_label(value) + (' (Default)' if step == default else '')
                patch = '&[' + ', '.join(f'({offset}, {byte})' for offset, byte in patches(option, step)) + ']'
                output.append(f'Value {{ key: {cstr(str(value))}, label: {cstr(label_text)}, patch: {patch} }},')
            output.append('] },')
        startup_patches = []
        for label, value in startup.items():
            option = reviewed_option(options, label)
            step = next(i for i, v in enumerate(option['observed_values']) if str(v) == value)
            startup_patches.extend(patches(option, step))
        if backup_storage:
            # Full native Clear, saved after menu exit and verified on cold
            # reload. Variation samples remain historical encoding evidence.
            baseline_path = ROOT/'validation/nvram-campaigns/2026-10-04/netmerc/initialized-baseline/saved.srm'
            baseline = baseline_path.read_bytes()
            validate_save(baseline, name)
            if baseline[64+0x18:64+0x1a] != bytes(2):
                raise ValueError('NetMerc initialized baseline has nonzero credits')
            for option in options:
                step = option['observed_values'].index(option['native_default'])
                for offset, value in patches(option, step):
                    if baseline[64+offset] != value:
                        raise ValueError('NetMerc initialized baseline differs from native defaults')
        payload = baseline[64:]; rle = encoded_rle(payload)
        if backup_storage:
            relative = baseline_path.relative_to(ROOT) if baseline_path.is_relative_to(ROOT) else baseline_path
            backup_templates.append(f'// {name}: {relative}; payload SHA256 {hashlib.sha256(payload).hexdigest()}\nBackupTemplate {{ set: "{name}", encoded: {array(rle)}, payload_crc: {crc(payload)}, startup: &[' +
                                    ', '.join(f'({o}, {v})' for o, v in startup_patches) + '] },')
            continue
        start, end, initial, swap, big, mirror = spec
        relative = baseline_path.relative_to(ROOT) if baseline_path.is_relative_to(ROOT) else baseline_path
        templates.append(f'// {name}: {relative}; payload SHA256 {hashlib.sha256(payload).hexdigest()}\nTemplate {{ set: "{name}", encoded: {array(rle)}, payload_crc: {crc(payload)}, startup: &['+
                         ', '.join(f'({o}, {v})' for o, v in startup_patches)+
                         f'], layout: Layout {{ start: {start}, end: {end}, initial: {initial}, swap: {str(swap).lower()}, big: {str(big).lower()}, mirror: {str(mirror).lower()} }} }},')
    output.extend(['];', '#[rustfmt::skip]', 'pub static TEMPLATES: &[Template] = &[', *templates, '];', '#[rustfmt::skip]', 'pub static BACKUP_TEMPLATES: &[BackupTemplate] = &[', *backup_templates, '];', ''])
    text = '\n'.join(output)
    if args.check:
        if args.output.read_text() != text: raise ValueError('Generated adapter data differs; regenerate')
    else: args.output.write_text(text)
    print(json.dumps({'sets': len(setups), 'fields': sum(map(len, selected.values())), 'validated_samples': count, 'check': args.check}))


if __name__ == '__main__': main()
