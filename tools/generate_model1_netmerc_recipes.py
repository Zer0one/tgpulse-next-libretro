#!/usr/bin/env python3
"""Generate NetMerc recipes from the photographed YAML inventory.

Reuse libretro_nvram_capture.py for acquisition. All variations start from the
same imported native baseline; native menu exits precede the saved capture.
"""
import argparse
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def frame(actions, frames, button=None, analog=None):
    action = {'frames': frames}
    if button: action['buttons'] = [button]
    if analog: action['analog'] = analog
    actions.append(action)


def pulse(actions, button='R3'):
    frame(actions, 10, button); frame(actions, 20)


def enter(menu):
    actions = []; frame(actions, 1200); pulse(actions, 'L3')
    for _ in range({'game_assignments': 6, 'coin_assignments': 7, 'controller': 5}[menu]):
        pulse(actions)
    pulse(actions, 'L3')
    return actions


def write(path, actions, **metadata):
    lines = ['set = "netmerc"', *[f'{k} = {json.dumps(v)}' for k, v in metadata.items()]]
    for action in actions:
        lines += ['', '[[actions]]']
        for key, value in action.items():
            encoded = ('{ ' + ', '.join(f'{k} = {v}' for k, v in value.items()) + ' }'
                       if isinstance(value, dict) else json.dumps(value))
            lines.append(f'{key} = {encoded}')
    path.write_text('\n'.join(lines) + '\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    args = parser.parse_args(); args.output.mkdir(parents=True, exist_ok=True)
    doc = json.loads(subprocess.check_output(['ruby', '-ryaml', '-rjson', '-e',
        'puts JSON.generate(YAML.safe_load(File.read(ARGV[0]), permitted_classes: [], aliases: true))',
        str(ROOT/'data/diagnostic-menus/netmerc.yaml')], text=True))
    assert doc['game'] == {'set': 'netmerc', 'parent': 'netmerc'}
    count = 0
    for menu, page in doc['menus'].items():
        for item in page['options']:
            assert item['values_status'] == 'cycle_closed'
            for step, value in enumerate(item['observed_values']):
                actions = enter(menu)
                for _ in range(item['position']): pulse(actions)
                for _ in range(step): pulse(actions, 'L3')
                actions.append({'capture': 'selected'})
                for _ in range((4 if menu == 'game_assignments' else 2) - item['position']): pulse(actions)
                pulse(actions, 'L3'); actions.append({'capture': 'after-page-exit'})
                for _ in range(5 if menu == 'game_assignments' else 4): pulse(actions)
                pulse(actions, 'L3'); frame(actions, 120)
                actions.append({'capture': 'saved'})
                write(args.output/f'{item["key"]}-{step}.toml', actions,
                      field=item['key'], expected_value=value, value_step=step)
                count += 1
            actions = enter(menu)
            for _ in range(item['position']): pulse(actions)
            actions.append({'capture': 'reloaded'})
            write(args.output/f'{item["key"]}-verify.toml', actions, field=item['key'])
    actions=[]; frame(actions,1200); actions.append({'capture':'base'})
    write(args.output/'baseline.toml',actions)
    actions = enter('controller')
    for i, axis in enumerate([{'left_x':-32768}, {'left_x':32767},
                               {'left_y':-32768}, {'left_y':32767}]):
        pulse(actions); frame(actions,60,analog=axis)
        frame(actions,10,'A',axis); frame(actions,30,analog=axis)
        actions.append({'capture':f'endpoint-{i}'})
    for _ in range(2): pulse(actions)
    pulse(actions,'L3'); actions.append({'capture':'after-page-exit'})
    for _ in range(6): pulse(actions)
    pulse(actions,'L3'); frame(actions,120); actions.append({'capture':'saved'})
    write(args.output/'controller-full-range.toml',actions)
    actions=enter('controller'); actions.append({'capture':'reloaded'})
    write(args.output/'controller-verify.toml',actions)
    print(f'Generated {count} setting samples, 4 reload recipes, baseline and full-range calibration/reload')


if __name__ == '__main__': main()
