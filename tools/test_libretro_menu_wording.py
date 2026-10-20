#!/usr/bin/env python3
"""Audit final ABI registrations against the reviewed workbook and F04/F14.

Read-only workbook use; reuses LCFHost. No ROMs/frontend or user saves needed.
The baseline checks preservation, never supplies the expected reviewed text.
"""
import argparse
import ctypes as c
import hashlib
import json
import re
from pathlib import Path

from test_libretro_lcf_options import LCFHost
from test_libretro_netmerc_lcd import Options

LINK_HELP = ('Select the total number of linked cabinets, including a Relay where supported. '
             '1 Cabinet / Disabled disconnects optional communication boards or keeps required boards in local loopback; '
             'it does not change the selected native role. Supported totals and role combinations depend on the game. '
             'Effective changes reset automatically when returning to the game.')
ROLE_HELP = ('Apply the native role and game identity independently of Linked Cabinets, even with NVRAM Settings (Service Menu) disabled. '
             'Disabled restores manual management. Managed native fields are hidden; unrelated settings are preserved. '
             'Effective changes reset automatically when returning to the game.')
SETS = ('vr', 'vformula', 'wingwar', 'wingwaru', 'wingwarj', 'wingwar360')
FUNCTIONAL_GROUPS = (
    ('netmerc_mvd_input', 'netmerc_mvd_horizontal_range', 'netmerc_mvd_vertical_range',
     'netmerc_mvd_gravity_stabilization', 'netmerc_mvd_drift_compensation', 'netmerc_mvd_sensor_diagnostics'),
    ('netmerc_diagnostic_display', 'netmerc_diagnostic_position', 'netmerc_diagnostic_opacity'),
)
# Stable reviewed-row identity, not source-code text extraction.
KEYS = {
    7: 'av_timing', 8: 'aspect_ratio', 40: 'initial_nvram_setup',
    34: 'automatic_network_*', 1: 'controller_profile_type',
    3: 'accelerator_output_range', 4: 'brake_output_range', 5: 'steering_output_range',
    6: 'steering_response', 13: 'dsb_gain', 14: 'ym3438_gain', 71: 'gamepad_rumble',
    33: 'rom_repairs', 35: 'linked_cabinets_*', 9: 'volume', 15: 'multipcm1_gain',
    16: 'multipcm2_gain', 2: 'nvram_settings', 10: 'renderer', 22: 'netmerc_audio_donor',
    23: 'netmerc_city_workaround', 24: 'netmerc_diagnostic_opacity',
    25: 'netmerc_diagnostic_display', 26: 'netmerc_diagnostic_position',
    27: 'netmerc_mvd_drift_compensation', 28: 'netmerc_mvd_gravity_stabilization',
    29: 'netmerc_mvd_horizontal_range', 30: 'netmerc_mvd_input',
    31: 'netmerc_mvd_sensor_diagnostics', 32: 'netmerc_mvd_vertical_range',
    11: 'smooth_shadows', 56: 'supersampling', 12: 'timing_overlay', 207: 'widescreen_mode',
}


class AuditHost(LCFHost):
    def __init__(self, *args, **kwargs):
        self.registered = {}
        super().__init__(*args, **kwargs)

    def environment(self, command, data):
        if command == 67:
            definitions = c.cast(data, c.POINTER(Options)).contents.definitions
            registered = {}
            i = 0
            while definitions[i].key:
                d = definitions[i]
                text = lambda x: x.decode() if x else None
                registered[d.key.decode()] = {
                    'name': text(d.desc), 'help': text(d.info),
                    'categorized_name': text(d.desc_categorized),
                    'categorized_help': text(d.info_categorized),
                    'category': text(d.category_key), 'default': text(d.default_value),
                    'values': [[text(v.value), text(v.label)] for v in d.values if v.value],
                }
                i += 1
            self.registered = registered
        return super().environment(command, data)


def main():
    import openpyxl  # Read-only extraction; no workbook authoring.
    p = argparse.ArgumentParser(description=__doc__)
    for key in ('core', 'workbook', 'output'):
        p.add_argument('--' + key, type=Path, required=True)
    p.add_argument('--baseline', type=Path)
    p.add_argument('--order-only', action='store_true', help='Permit only F04 presentation order changes against baseline')
    a = p.parse_args()
    a.output.mkdir(parents=True, exist_ok=False)
    h = AuditHost(a.core)
    modern = h.registered
    h.lib.retro_deinit()
    h = AuditHost(a.core, legacy=True)
    legacy = {k.decode(): v.decode() for k, v in h.legacy_definitions.items()}
    h.lib.retro_deinit()
    errors = []
    def check(ok, message):
        if not ok: errors.append(message)
    w = openpyxl.load_workbook(a.workbook, read_only=True, data_only=True)
    rows = list(w['Menu Review'].values)
    headers = rows[0]
    selected = [dict(zip(headers, r)) for r in rows[1:] if r[6] and not r[2]]
    check({r['Order'] for r in selected} == set(KEYS), 'Workbook applicable row coverage changed')
    checked = []
    for row in selected:
        order = row['Order']
        if order not in KEYS: continue
        suffix = KEYS[order]
        keys = ['tgpulse_next_' + suffix.replace('*', s) for s in SETS] if '*' in suffix else ['tgpulse_next_' + suffix]
        name, help_text = row['Name (New)'] or row['Name'], row['Comments']
        # Existing documented target-specific exceptions. Network wording must
        # match both the reconciled editable workbook and F14, independently.
        if order == 22: name = 'Sega NetMerc: Audio Donor (Restart Required)'
        if order == 207:
            help_text = 'Expand the 3D field of view and report 16:9 automatically, optionally stretching both 2D layers. Applies only to games without native widescreen support, with hardware rendering and Aspect Ratio set to Auto or 16:9. Explicit 4:3 suppresses expansion. Disabled preserves Aspect Ratio and native monitor settings. Native widescreen is unchanged. Changes apply when returning to the game without a reset.'
        for key in keys:
            d = modern[key]
            check(d['name'] == name, key + ': reviewed name')
            check(d['help'] == help_text, key + ': reviewed full help')
            check(d['category'] == row['Submenu'].lower(), key + ': reviewed category')
            check(legacy[key].split('; ', 1)[0] == name, key + ': legacy name')
            checked.append(key)
    check(set(modern) == set(legacy), 'Modern/legacy key coverage')
    check(list(modern) == list(legacy), 'Modern/legacy registration order')
    categories = [d['category'] for d in modern.values()]
    check(list(dict.fromkeys(categories)) == ['system', 'video', 'audio', 'input'], 'Reviewed category order')
    check(categories == sorted(categories, key=['system', 'video', 'audio', 'input'].index), 'Category grouping')
    for category in ('video', 'audio', 'input'):
        entries = [(k, d) for k, d in modern.items() if d['category'] == category]
        globals_ = [k for k, d in entries if ': ' not in d['name']]
        games = [(k, d) for k, d in entries if ': ' in d['name']]
        def group_order(entry):
            key, definition = entry
            for group in FUNCTIONAL_GROUPS:
                keys = ['tgpulse_next_' + member for member in group]
                if key in keys:
                    return (modern[keys[0]]['name'], keys.index(key))
            return (definition['name'], 0)
        expected = globals_ + [k for k, d in sorted(games, key=group_order)]
        check([k for k, _ in entries] == expected, category + ': globals first, game groups lexicographic last')
    check(list(modern).index('tgpulse_next_netmerc_mvd_vertical_range') == list(modern).index('tgpulse_next_netmerc_mvd_horizontal_range') + 1, 'MVD Horizontal/Vertical Range functional group')
    for group in FUNCTIONAL_GROUPS:
        positions = [list(modern).index('tgpulse_next_' + key) for key in group]
        check(positions == list(range(positions[0], positions[0] + len(group))), group[0] + ': parent-first functional group')
    for key, d in modern.items():
        raw = [v[0] for v in d['values']]
        check(d['default'] in raw, key + ': registered default missing')
        actual = legacy[key].split('; ', 1)[1].split('|')
        expected = [d['default']] + [v for v in raw if v != d['default']]
        if key.endswith('_gain'):
            expected += ['mute', 'Mute']  # Published legacy aliases.
        check(actual == expected, key + ': legacy default/ordered raw values')
        check(legacy[key].split('; ', 1)[0] == d['name'], key + ': legacy/modern name')
    for name in SETS:
        lk, rk = 'tgpulse_next_linked_cabinets_' + name, 'tgpulse_next_automatic_network_' + name
        check(modern[lk]['help'] == LINK_HELP and modern[rk]['help'] == ROLE_HELP, name + ': common help')
        check(modern[lk]['default'] == '-1' and modern[rk]['default'] == '-1', name + ': F14 defaults')
        check(list(modern).index(rk) == list(modern).index(lk) + 1, name + ': adjacency')
        link_values = [['-1', '1 Cabinet / Disabled']] + [[str(i), f'{i} Cabinets'] for i in range(2, 3 if name.startswith('wingwar') else 10)]
        check(modern[lk]['values'] == link_values, name + ': topology values/labels/order')
        roles = [['-1', 'Disabled'], ['0', 'Stand Alone' if name.startswith('wingwar') else 'No Link']]
        if name.startswith('wingwar'):
            roles += [['1', 'Master'], ['2', 'Slave']]
        else:
            colors = ['Red', 'Blue', 'Yellow', 'Green', 'Black', 'Pink', 'Skyblue', 'Orange'] if name == 'vformula' else ['Red', 'Orange', 'Skyblue', 'Pink', 'Black', 'Green', 'Yellow', 'Blue']
            roles += [[str(i), (f'No.{i} ({color})' if name == 'vformula' else color) + (' (Master)' if i == 1 else ' (Slave)')] for i, color in enumerate(colors, 1)]
            roles += [['r', 'Live']]
        check(modern[rk]['values'] == roles, name + ': native role values/labels/order')
    for suffix, name in [('linked_cabinets', 'Linked Cabinets'), ('automatic_network', 'Auto Network Role Settings')]:
        d = modern['tgpulse_next_' + suffix + '_unsupported']
        check(d['name'] == name and d['help'] == 'Networking is not supported by this game.'
              and d['default'] == 'Not Supported' and d['values'] == [['Not Supported', 'Not Supported']], 'Unsupported placeholder contract')
    # ROM IDs are implementation keys, not display/help prose. Scan exact tokens;
    # separately permit the established uppercase VR game acronym where present.
    identifiers = ('vr', 'vformula', 'wingwar', 'wingwaru', 'wingwarj', 'wingwar360', 'vf', 'swa', 'swaj', 'netmerc')
    hits = []
    for key, d in modern.items():
        for field in ('name', 'help', 'categorized_name', 'categorized_help'):
            for token in identifiers:
                if re.search(r'(?<![\w])' + token + r'(?![\w])', d[field] or ''):
                    hits.append([key, field, token])
    check(not hits, 'Raw ROM identifiers in user-facing strings: ' + str(hits))
    stale = [(k, f) for k, d in modern.items() for f in ('name', 'help') if re.search(r'lightun\b|from the select cabinet|Network Board|preset is ignored', d[f] or '')]
    check(not stale, 'Stale reviewed wording: ' + str(stale))
    snapshot = {'modern': modern, 'legacy': legacy}
    (a.output / 'registration.json').write_text(json.dumps(snapshot, indent=2) + '\n')
    changes = []
    order_changes = []
    if a.baseline:
        old = json.loads(a.baseline.read_text())
        # Separate placeholder rank from the first supported pair. Previously
        # their tied ranks yielded different V2 and legacy registration orders.
        placeholder = 'tgpulse_next_automatic_network_unsupported'
        if a.order_only:
            check(set(old['modern']) == set(modern), 'Registered keys changed')
            for category in ('system', 'video', 'audio', 'input'):
                retained = lambda options: [k for k, d in options.items() if d['category'] == category and (category == 'system' or ': ' not in d['name'])]
                check(retained(old['modern']) == retained(modern), category + ': functional/global order changed')
        else:
            check([k for k in old['modern'] if k != placeholder] == [k for k in modern if k != placeholder], 'Unrelated baseline order changed')
        check(old['legacy'] == legacy, 'Legacy registration text/values changed')
        for api in ('modern', 'legacy'):
            before, after = list(old[api]), list(snapshot[api])
            order_changes += [[api, key, before.index(key), after.index(key)] for key in after if before.index(key) != after.index(key)]
        for key, d in modern.items():
            for field, value in d.items():
                if old['modern'][key][field] != value:
                    changes.append([key, field])
                    check(not a.order_only and key.startswith('tgpulse_next_linked_cabinets_') and key.endswith(SETS) and field == 'help', key + ': unrelated baseline change')
    result = {'core_sha256': hashlib.sha256(a.core.read_bytes()).hexdigest(),
              'workbook_sha256': hashlib.sha256(a.workbook.read_bytes()).hexdigest(),
              'reviewed_rows': len(selected), 'reviewed_keys': len(checked),
              'modern_options': len(modern), 'legacy_options': len(legacy),
              'network_pairs': len(SETS), 'raw_id_hits': hits, 'stale_wording': stale,
              'changes': changes, 'order_changes': order_changes, 'errors': errors}
    (a.output / 'audit.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2))
    if errors: raise SystemExit(1)


if __name__ == '__main__': main()
