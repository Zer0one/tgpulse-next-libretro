#!/usr/bin/env python3
"""Assemble/check ROM-free core packages, adapting SM2's release package gate."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]
INFO = 'tgpulse_next_m1_libretro.info'


def run(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def version():
    text = (ROOT / INFO).read_text()
    return re.search(r'^display_version = "([^"]+)"$', text, re.M).group(1)


def collect_licenses(output, target, offline):
    command = ['cargo', 'metadata', '--locked', '--format-version', '1',
               '--filter-platform', target]
    if offline:
        command.append('--offline')
    metadata = json.loads(run(*command))
    packages = {p['id']: p for p in metadata['packages']}
    # metadata resolves the whole workspace and unifies standalone features.
    # tree selects the actual M1 package graph, including its build dependencies.
    tree = ['cargo', 'tree', '--locked', '-p', 'tgpulse-libretro', '--target', target,
            '-e', 'normal,build', '--prefix', 'none', '--format', '{p}']
    if offline:
        tree.append('--offline')
    selected = {(line.split()[0], line.split()[1].removeprefix('v'))
                for line in run(*tree).splitlines()}
    seen = {key for key, p in packages.items() if (p['name'], p['version']) in selected}
    assert len(seen) == len(selected), 'Ambiguous dependency package identity'
    assert not any(packages[key]['name'] in {'i960', 'mb86235', 'sharc'} for key in seen), \
        'M1 package includes Model 2 CPU dependencies'
    inventory = []
    for package in sorted((packages[key] for key in seen), key=lambda p: (p['name'], p['version'])):
        entry = {k: package[k] for k in ('name', 'version', 'license', 'authors', 'repository')}
        entry['license_files'] = []
        if package['source']:
            source = Path(package['manifest_path']).parent
            legal = sorted(p for p in source.rglob('*') if p.is_file() and
                           p.name.upper().startswith(('LICENSE', 'LICENCE', 'COPYING', 'NOTICE', 'COPYRIGHT')))
            fallback = ROOT / 'LICENSES/crates' / (package['name'] + '-' + package['version'])
            if not legal and fallback.is_dir():
                source = fallback
                legal = sorted(p for p in fallback.rglob('*') if p.is_file())
            if not legal:
                raise ValueError(f"Missing license texts: {package['name']} {package['version']}")
            for file in legal:
                relative = Path('licenses/crates') / (package['name'] + '-' + package['version']) / file.relative_to(source)
                destination = output / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(file, destination)
                entry['license_files'].append(relative.as_posix())
        else:
            entry['license_files'] = ['LICENSE']
        inventory.append(entry)
    for name in ('libretro-api.txt', 'MAME-BSD-3-Clause.txt', 'YMFM-BSD-3-Clause.txt'):
        shutil.copyfile(ROOT / 'LICENSES' / name, output / 'licenses' / name)
    (output / 'licenses/THIRD_PARTY.json').write_text(json.dumps(inventory, indent=2) + '\n', encoding='utf-8')


def check(output, expected_version):
    info = (output / INFO).read_text()
    assert f'display_version = "{expected_version}"' in info
    assert 'systemname = "Model 1"' in info and 'license = "MIT"' in info
    assert 'savestate = "true"' in info and 'libretro_saves = "true"' in info
    assert 'corename = "TGPulse-Next: Model 1"' in info
    assert 'display_name = "Sega - Model 1 (TGPulse-Next)"' in info
    commit = (output / 'SOURCE_COMMIT.txt').read_text().strip()
    assert re.fullmatch(r'[0-9a-f]{40}', commit), commit
    build = json.loads((output / 'BUILD_INFO.json').read_text())
    assert build['source_commit'] == commit and build['version'] == expected_version
    binary = build['binary']
    assert binary in {f'tgpulse_next_m1_libretro{suffix}' for suffix in ('.so', '.dylib', '.dll')}
    assert (output / binary).stat().st_size > 0
    assert digest(output / binary) == build['binary_sha256']
    if 'machine_scope' in build:
        assert build['machine_scope'] == 'model1'
        scopes = set(re.findall(rb'TGPulse compiled machines: (model1\+model2|model1|model2)',
                                (output / binary).read_bytes()))
        assert scopes == {b'model1'}, scopes
    inventory = json.loads((output / 'licenses/THIRD_PARTY.json').read_text())
    if 'machine_scope' in build:
        assert not any(p['name'] in {'i960', 'mb86235', 'sharc'} for p in inventory)
    required = {binary, INFO, 'LICENSE', 'NOTICE', 'README.md', 'SOURCE_COMMIT.txt',
                'BUILD_INFO.json', 'SHA256SUMS', 'licenses/THIRD_PARTY.json', 'licenses/libretro-api.txt',
                'licenses/MAME-BSD-3-Clause.txt', 'licenses/YMFM-BSD-3-Clause.txt'}
    for entry in inventory:
        assert entry['license'] and entry['license_files'], entry
        required.update(entry['license_files'])
    files = {f.relative_to(output).as_posix() for f in output.rglob('*') if f.is_file()}
    assert files == required, (required - files, files - required)
    checksums = {}
    for line in (output / 'SHA256SUMS').read_text().splitlines():
        expected, separator, name = line.partition('  ')
        assert separator and re.fullmatch(r'[0-9a-f]{64}', expected) and name not in checksums, line
        checksums[name] = expected
    assert set(checksums) == files - {'SHA256SUMS'}
    for name, expected in checksums.items():
        assert digest(output / name) == expected, name
    print(f'PASS: package {build["target"]}, {len(files)} files, version, source revision, licenses and checksums')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--core', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--target')
    parser.add_argument('--archive', type=Path)
    parser.add_argument('--version', default=version())
    parser.add_argument('--offline', action='store_true')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    if args.check:
        check(args.output, args.version)
        return
    if args.core is None or args.target is None:
        parser.error('--core and --target are required when assembling')
    if args.version != version():
        parser.error('Release version must match core metadata')
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    binary = 'tgpulse_next_m1_libretro' + args.core.suffix
    shutil.copyfile(args.core, output / binary)
    for name in (INFO, 'LICENSE', 'NOTICE'):
        shutil.copyfile(ROOT / name, output / name)
    shutil.copyfile(ROOT / 'docs/LIBRETRO_RELEASE.md', output / 'README.md')
    commit = run('git', 'rev-parse', 'HEAD')
    (output / 'SOURCE_COMMIT.txt').write_text(commit + '\n', encoding='ascii')
    build = {'version': args.version, 'source_commit': commit, 'target': args.target,
             'binary': binary, 'binary_sha256': digest(output / binary),
             'rustc': run('rustc', '-Vv'), 'cargo': run('cargo', '--version'),
             'profile': 'release', 'lto': 'thin', 'codegen_units': 1,
             'machine_scope': 'model1'}
    (output / 'BUILD_INFO.json').write_text(json.dumps(build, indent=2) + '\n', encoding='utf-8')
    collect_licenses(output, args.target, args.offline)
    lines = [digest(f) + '  ' + f.relative_to(output).as_posix() + '\n'
             for f in sorted(output.rglob('*')) if f.is_file()]
    (output / 'SHA256SUMS').write_text(''.join(lines), encoding='ascii')
    check(output, args.version)
    if args.archive:
        args.archive.parent.mkdir(parents=True, exist_ok=True)
        stamp = datetime.fromtimestamp(int(run('git', 'show', '-s', '--format=%ct', 'HEAD')), timezone.utc)
        with zipfile.ZipFile(args.archive, 'x', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
            for file in sorted(output.rglob('*')):
                if file.is_file():
                    entry = zipfile.ZipInfo(file.relative_to(output).as_posix(), stamp.timetuple()[:6])
                    entry.compress_type = zipfile.ZIP_DEFLATED
                    entry.external_attr = 0o100644 << 16
                    archive.writestr(entry, file.read_bytes())
        print(f'Archive: {args.archive.name} SHA256 {digest(args.archive)}')


if __name__ == '__main__':
    main()
