#!/usr/bin/env python3
"""Capture Model 1 Save RAM through the ABI; see docs/NVRAM_CAPTURE.md."""
import argparse
import binascii
import ctypes as c
import hashlib
import json
import pathlib
import struct
import time
import tomllib
import zlib

BUTTONS = dict(zip(
    ('B', 'Y', 'SELECT', 'START', 'UP', 'DOWN', 'LEFT', 'RIGHT',
     'A', 'X', 'L', 'R', 'L2', 'R2', 'L3', 'R3'), range(16)))
ANALOG_KEYS = {'left_x', 'left_y', 'l2', 'r2'}
CAPTURE_OPTIONS = {
    b'tgpulse_next_initial_nvram_setup': b'disabled',
    b'tgpulse_next_nvram_settings': b'disabled',
}
ENV = c.CFUNCTYPE(c.c_bool, c.c_uint, c.c_void_p)
VIDEO = c.CFUNCTYPE(None, c.c_void_p, c.c_uint, c.c_uint, c.c_size_t)
BATCH = c.CFUNCTYPE(c.c_size_t, c.POINTER(c.c_short), c.c_size_t)
POLL = c.CFUNCTYPE(None)
INPUT = c.CFUNCTYPE(c.c_short, c.c_uint, c.c_uint, c.c_uint, c.c_uint)


class GameInfo(c.Structure):
    _fields_ = [('path', c.c_char_p), ('data', c.c_void_p),
               ('size', c.c_size_t), ('meta', c.c_char_p)]


class Variable(c.Structure):
    _fields_ = [('key', c.c_char_p), ('value', c.c_char_p)]


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def validate_save(raw, expected_set):
    if len(raw) != 65728 or raw[:8] != b'TGP1SRAM':
        raise ValueError('Unexpected Model 1 Save RAM size or magic')
    if struct.unpack_from('<III', raw, 8) != (1, 65536, 128):
        raise ValueError('Unexpected Save RAM layout')
    if raw[24:56].split(b'\0')[0].decode('ascii') != expected_set:
        raise ValueError('Save RAM set does not match content')
    crc = binascii.crc_hqx(raw[64:], 0)
    if crc != struct.unpack_from('<H', raw, 20)[0]:
        raise ValueError('Save RAM container CRC mismatch')


def png(frame):
    raw, width, height, pitch = frame
    scan = bytearray()
    for y in range(height):
        scan.append(0)
        row = raw[y * pitch:y * pitch + width * 4]
        for x in range(width):
            i = x * 4
            scan.extend((row[i + 2], row[i + 1], row[i]))
    def chunk(kind, data):
        return (struct.pack('>I', len(data)) + kind + data
                + struct.pack('>I', zlib.crc32(kind + data)))
    return (b'\x89PNG\r\n\x1a\n'
            + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 2, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(scan)) + chunk(b'IEND', b''))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--core', type=pathlib.Path, required=True)
    parser.add_argument('--rom', type=pathlib.Path, required=True)
    parser.add_argument('--recipe', type=pathlib.Path, required=True)
    parser.add_argument('--output', type=pathlib.Path, required=True)
    parser.add_argument('--load', type=pathlib.Path)
    parser.add_argument('--dry-run', action='store_true')
    args = parser.parse_args()
    recipe = tomllib.loads(args.recipe.read_text())
    actions = recipe['actions']
    names = set()
    for action in actions:
        if 'frames' in action:
            if not isinstance(action['frames'], int) or not 1 <= action['frames'] <= 36000:
                parser.error('Each frame step must be between 1 and 36000')
            for button in action.get('buttons', []):
                if button not in BUTTONS:
                    parser.error('Unknown button: ' + button)
            for key, value in action.get('analog', {}).items():
                if key not in ANALOG_KEYS or not isinstance(value, int) or not -32768 <= value <= 32767:
                    parser.error('Analog input must use left_x, left_y, l2 or r2 with a signed 16-bit value')
        elif set(action) == {'capture'}:
            name = action['capture']
            if not isinstance(name, str) or not name or any(ch not in
                    'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_' for ch in name):
                parser.error('Capture names must be simple file stems')
            if name in names:
                parser.error('Duplicate capture name')
            names.add(name)
        else:
            parser.error('Expected a frame step or capture action')
    if args.dry_run:
        print(json.dumps({'set': recipe['set'], 'captures': sorted(names),
                          'frames': sum(a.get('frames', 0) for a in actions)}))
        return
    if args.rom.stem != recipe['set']:
        parser.error('ROM file stem must match recipe set')
    output = args.output.resolve()
    repo = pathlib.Path(__file__).resolve().parents[1]
    if output == repo or repo in output.parents:
        parser.error('Capture output must be outside the source checkout')
    # Check inputs before creating evidence output; failed preflight should not
    # leave an empty directory that blocks a corrected run.
    core_hash = digest(args.core)
    rom_hash = digest(args.rom)
    manifest = {'status': 'running', 'method': 'direct_libretro_abi',
                'set': recipe['set'], 'core_sha256': core_hash,
                'rom_sha256': rom_hash, 'recipe': recipe, 'captures': [],
                'core_options': {key.decode(): value.decode()
                                 for key, value in CAPTURE_OPTIONS.items()}}
    # A new directory protects earlier evidence from accidental replacement.
    output.mkdir(parents=True, exist_ok=False)
    lib = c.CDLL(str(args.core.resolve()))
    held, analog, last = set(), {}, [None]
    pixel_format = [None]
    @ENV
    def environment(command, data):
        if command == 15:  # GET_VARIABLE: acquire native operator settings
            variable = c.cast(data, c.POINTER(Variable)).contents
            variable.value = CAPTURE_OPTIONS.get(variable.key)
            return variable.value is not None
        if command == 52:
            c.cast(data, c.POINTER(c.c_uint))[0] = 2
            return True
        if command == 10:
            pixel_format[0] = c.cast(data, c.POINTER(c.c_uint))[0]
            return pixel_format[0] == 1  # XRGB8888
        if command == 17:  # GET_VARIABLE_UPDATE: no option changes
            c.cast(data, c.POINTER(c.c_bool))[0] = False
            return True
        return command in (11, 35, 67)
    @VIDEO
    def video(data, width, height, pitch):
        if data:
            last[0] = (c.string_at(data, height * pitch), width, height, pitch)
    @BATCH
    def batch(data, count):
        return count
    @POLL
    def poll():
        pass
    @INPUT
    def input_state(port, device, index, button):
        if port != 0:
            return 0
        if device == 1:  # RETRO_DEVICE_JOYPAD
            return int(button in held)
        if device == 5 and index == 0:  # Left analog stick
            return int(analog.get(('left_x', 'left_y')[button], 0)) if button in (0, 1) else 0
        if device == 5 and index == 2:  # Analog L2 / R2 buttons
            return int(analog.get('l2' if button == 12 else 'r2', 0)) if button in (12, 13) else 0
        return 0
    callbacks = [('retro_set_environment', environment), ('retro_set_video_refresh', video),
                 ('retro_set_audio_sample_batch', batch), ('retro_set_input_poll', poll),
                 ('retro_set_input_state', input_state)]
    for name, callback in callbacks:
        fn = getattr(lib, name)
        fn.argtypes = [type(callback)]
        fn(callback)
    lib.retro_load_game.argtypes = [c.POINTER(GameInfo)]
    lib.retro_load_game.restype = c.c_bool
    lib.retro_get_memory_data.argtypes = [c.c_uint]
    lib.retro_get_memory_data.restype = c.c_void_p
    lib.retro_get_memory_size.argtypes = [c.c_uint]
    lib.retro_get_memory_size.restype = c.c_size_t
    loaded, frames = False, 0
    started = time.monotonic()
    lib.retro_init()
    try:
        loaded = lib.retro_load_game(c.byref(GameInfo(str(args.rom.resolve()).encode(), None, 0, None)))
        if not loaded:
            raise RuntimeError('Core rejected content')
        if args.load:
            raw = args.load.read_bytes()
            validate_save(raw, recipe['set'])
            if len(raw) != lib.retro_get_memory_size(0):
                raise ValueError('Frontend Save RAM buffer has unexpected size')
            c.memmove(lib.retro_get_memory_data(0), raw, len(raw))
            manifest['import_sha256'] = digest(args.load)
        for action in actions:
            if 'frames' in action:
                held.clear()
                held.update(BUTTONS[b] for b in action.get('buttons', []))
                analog.clear()
                analog.update(action.get('analog', {}))
                for _ in range(action['frames']):
                    lib.retro_run()
                    frames += 1
                continue
            name = action['capture']
            raw = c.string_at(lib.retro_get_memory_data(0), lib.retro_get_memory_size(0))
            validate_save(raw, recipe['set'])
            save = output / (name + '.srm')
            save.write_bytes(raw)
            if last[0] is None or pixel_format[0] != 1:
                raise RuntimeError('No XRGB8888 frame available')
            screenshot = output / (name + '.png')
            screenshot.write_bytes(png(last[0]))
            manifest['captures'].append({'name': name, 'frame': frames,
                'save_sha256': digest(save), 'screenshot_sha256': digest(screenshot),
                'container_valid': True})
        manifest['status'] = 'complete'
    except Exception as error:
        manifest['status'] = 'failed'
        manifest['error'] = str(error)
        raise
    finally:
        if loaded:
            lib.retro_unload_game()
        lib.retro_deinit()
        manifest['elapsed_seconds'] = round(time.monotonic() - started, 3)
        (output / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps({'status': manifest['status'], 'output': str(output), 'frames': frames}))


if __name__ == '__main__':
    main()
