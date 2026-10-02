#!/usr/bin/env python3
"""Adapt SM2's ROM-free ABI, dependency and empty-lifecycle release gate."""
import argparse
import ctypes as c
import platform
from pathlib import Path
import re
import subprocess

EXPORTS = {
    'retro_api_version', 'retro_cheat_reset', 'retro_cheat_set', 'retro_deinit',
    'retro_get_memory_data', 'retro_get_memory_size', 'retro_get_region',
    'retro_get_system_av_info', 'retro_get_system_info', 'retro_init',
    'retro_load_game', 'retro_load_game_special', 'retro_reset', 'retro_run',
    'retro_serialize', 'retro_serialize_size', 'retro_set_audio_sample',
    'retro_set_audio_sample_batch', 'retro_set_controller_port_device',
    'retro_set_environment', 'retro_set_input_poll', 'retro_set_input_state',
    'retro_set_video_refresh', 'retro_unload_game', 'retro_unserialize',
}


class Info(c.Structure):
    _fields_ = [('name', c.c_char_p), ('version', c.c_char_p),
               ('extensions', c.c_char_p), ('fullpath', c.c_bool),
               ('block_extract', c.c_bool)]


def command(*args):
    result = subprocess.check_output(args, text=True, stderr=subprocess.STDOUT)
    print(result.strip())
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('core', type=Path)
    parser.add_argument('--target', required=True)
    parser.add_argument('--info', type=Path, help='Metadata from the exact source checkout being built')
    parser.add_argument('--machine-scope', choices=('model1', 'model2', 'model1+model2'),
                        help='Require the compiled machine marker, independently of metadata')
    args = parser.parse_args()
    core = args.core.resolve(strict=True)
    if args.machine_scope:
        scopes = set(re.findall(rb'TGPulse compiled machines: (model1\+model2|model1|model2)',
                                core.read_bytes()))
        assert scopes == {args.machine_scope.encode()}, scopes
    root = Path(__file__).resolve().parents[1]
    info = (args.info or root / 'tgpulse_next_m1_libretro.info').read_text()
    version = re.search(r'^display_version = "([^"]+)"$', info, re.M).group(1)
    kind = platform.system()
    if kind == 'Darwin':
        architecture = 'arm64' if args.target.startswith('aarch64') else 'x86_64'
        assert command('lipo', '-archs', str(core)).strip() == architecture
        symbols = command('nm', '-gU', str(core))
        exports = set(re.findall(r'\b_?(retro_\w+)$', symbols, re.M))
        deps = command('otool', '-L', str(core))
        assert all(line.strip().startswith(('/usr/lib/', '/System/Library/'))
                   for line in deps.splitlines()[2:]), deps
    elif kind == 'Windows':
        headers = command('dumpbin', '/headers', str(core))
        assert re.search(r'8664 machine', headers, re.I), headers
        symbols = command('dumpbin', '/exports', str(core))
        # MSVC may append "= <folded implementation>" to exported aliases.
        # The first name after the ordinal/hint/RVA is the actual ABI export.
        exports = set(re.findall(r'^\s+\d+\s+[0-9A-F]+\s+[0-9A-F]+\s+(retro_\w+)(?:\s|$)',
                                 symbols, re.M | re.I))
        deps = command('dumpbin', '/dependents', str(core))
        assert not re.search(r'libgcc|libstdc|libwinpthread|vcruntime|msvcp|vulkan|SDL', deps, re.I), deps
    else:
        headers = command('readelf', '-h', str(core))
        machine = 'AArch64' if args.target.startswith('aarch64') else 'X86-64'
        assert machine.lower() in headers.lower(), headers
        symbols = command('nm', '-D', '--defined-only', str(core))
        exports = set(re.findall(r'\b(retro_\w+)$', symbols, re.M))
        deps = command('ldd', str(core))
        assert 'not found' not in deps, deps
        assert not re.search(r'lib(vulkan|SDL|stdc\+\+)', deps), deps
    assert exports == EXPORTS, (exports - EXPORTS, EXPORTS - exports)
    lib = c.CDLL(str(core))
    for symbol in EXPORTS:
        getattr(lib, symbol)
    env_type = c.CFUNCTYPE(c.c_bool, c.c_uint, c.c_void_p)
    environment = env_type(lambda command, data: False)
    lib.retro_set_environment.argtypes = [env_type]
    lib.retro_set_environment(environment)
    lib.retro_get_system_info.argtypes = [c.POINTER(Info)]
    lib.retro_load_game.argtypes = [c.c_void_p]
    lib.retro_load_game.restype = c.c_bool
    lib.retro_serialize_size.restype = c.c_size_t
    lib.retro_get_memory_size.argtypes = [c.c_uint]
    lib.retro_get_memory_size.restype = c.c_size_t
    lib.retro_get_memory_data.argtypes = [c.c_uint]
    lib.retro_get_memory_data.restype = c.c_void_p
    assert lib.retro_api_version() == 1
    for _ in range(3):
        lib.retro_init()
        actual = Info()
        lib.retro_get_system_info(c.byref(actual))
        assert actual.name == b'TGPulse-Next' and actual.version.decode() == version
        assert actual.extensions == b'zip' and actual.fullpath and actual.block_extract
        assert not lib.retro_load_game(None)
        assert lib.retro_serialize_size() > 0
        assert lib.retro_get_memory_size(0) == 65728 and lib.retro_get_memory_data(0)
        assert lib.retro_get_memory_size(1) == 0 and not lib.retro_get_memory_data(1)
        lib.retro_run()
        lib.retro_reset()
        lib.retro_unload_game()
        lib.retro_deinit()
    print('PASS: native format, 25 ABI exports, dependency checks and three empty lifecycle cycles')


if __name__ == '__main__':
    main()
