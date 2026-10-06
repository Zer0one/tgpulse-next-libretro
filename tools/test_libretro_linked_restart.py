#!/usr/bin/env python3
"""Exercise Linked Cabinets changes through Libretro Restart and Resume.

The ROM is read only. The frontend callbacks, options and Save RAM stay in this
process; no personal RetroArch configuration or saves are touched.
"""
import argparse
import ctypes as c
import json
import struct
from pathlib import Path

from libretro_nvram_capture import ENV, VIDEO, BATCH, POLL, INPUT, GameInfo, validate_save


class Variable(c.Structure):
    _fields_ = [('key', c.c_char_p), ('value', c.c_char_p)]


class NetCallbacks(c.Structure):
    _fields_ = [(name, c.c_void_p) for name in
                ('start', 'receive', 'stop', 'poll', 'connected', 'disconnected',
                 'protocol_version')]


NET_SEND = c.CFUNCTYPE(None, c.c_int, c.c_void_p, c.c_size_t, c.c_uint16)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--core', type=Path, required=True)
    parser.add_argument('--rom', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    assert args.rom.stem == 'vr', 'VR is the selected cabinet-count fixture'
    args.output.mkdir(parents=True, exist_ok=False)
    options = {b'tgpulse_next_renderer': b'software',
               b'tgpulse_next_initial_nvram_setup': b'enabled',
               b'tgpulse_next_nvram_settings': b'enabled',
               b'tgpulse_next_automatic_network_vr': b'red',
               b'tgpulse_next_linked_cabinets_vr': b'disabled'}
    changed = [False]
    packets = []
    callbacks = []

    @NET_SEND
    def send(_flags, data, size, _recipient):
        packets.append(c.string_at(data, size))

    @POLL
    def net_poll():
        pass

    @ENV
    def environment(command, data):
        if command == 15:
            variable = c.cast(data, c.POINTER(Variable)).contents
            variable.value = options.get(variable.key)
            return variable.value is not None
        if command == 70:
            variable = c.cast(data, c.POINTER(Variable)).contents
            options[variable.key] = variable.value
            changed[0] = True
            return True
        if command == 17:
            c.cast(data, c.POINTER(c.c_bool))[0] = changed[0]
            changed[0] = False
            return True
        if command == 52:
            c.cast(data, c.POINTER(c.c_uint))[0] = 2
            return True
        if command == 78:
            callbacks.append(NetCallbacks.from_buffer_copy(
                c.string_at(data, c.sizeof(NetCallbacks))))
            return True
        if command == 10:
            return c.cast(data, c.POINTER(c.c_uint))[0] == 1
        return command in (11, 16, 18, 35, 37, 55, 67, 69)

    @VIDEO
    def video(_data, _width, _height, _pitch):
        pass

    @BATCH
    def audio(_data, count):
        return count

    @POLL
    def input_poll():
        pass

    @INPUT
    def input_state(_port, _device, _index, _button):
        return 0

    core = c.CDLL(str(args.core.resolve(strict=True)))
    for name, callback in [('environment', environment), ('video_refresh', video),
                           ('audio_sample_batch', audio), ('input_poll', input_poll),
                           ('input_state', input_state)]:
        method = getattr(core, 'retro_set_' + name)
        method.argtypes = [type(callback)]
        method(callback)
    core.retro_load_game.argtypes = [c.POINTER(GameInfo)]
    core.retro_load_game.restype = c.c_bool
    core.retro_serialize_size.restype = c.c_size_t
    core.retro_get_memory_data.argtypes = [c.c_uint]
    core.retro_get_memory_data.restype = c.c_void_p

    core.retro_init()
    path = str(args.rom.resolve(strict=True)).encode()
    assert core.retro_load_game(c.byref(GameInfo(path, None, 0, None)))
    assert len(callbacks) == 1
    start = c.CFUNCTYPE(None, c.c_uint16, c.c_void_p, c.c_void_p)(callbacks[0].start)
    start(0, c.cast(send, c.c_void_p), c.cast(net_poll, c.c_void_p))
    connected = c.CFUNCTYPE(c.c_bool, c.c_uint16)(callbacks[0].connected)
    receive = c.CFUNCTYPE(None, c.c_void_p, c.c_size_t, c.c_uint16)(callbacks[0].receive)
    assert connected(1), 'An offline host rejected the client before cabinet setup'
    set_hash = 2166136261
    for byte in b'vr':
        set_hash = ((set_hash ^ byte) * 16777619) & 0xffffffff
    early_hello = b'TGMN' + bytes([1, 1]) + struct.pack('<HIH', 2, set_hash, 0)
    packet = c.create_string_buffer(early_hello)
    receive(packet, len(early_hello), 1)  # Peer resumes before the host.

    def saved():
        raw = c.string_at(core.retro_get_memory_data(0), 65728)
        validate_save(raw, 'vr')
        return raw

    for _ in range(3):
        core.retro_run()
    assert core.retro_serialize_size() > 0
    saved()
    options[b'tgpulse_next_linked_cabinets_vr'] = b'2'
    options[b'tgpulse_next_automatic_network_vr'] = b'pink'
    changed[0] = True
    core.retro_reset()
    assert core.retro_serialize_size() == 0, 'Restart did not fit COMM board'
    core.retro_run()
    first_linked = saved()
    assert (first_linked[65613], first_linked[65635]) == (2, 5), \
        'First emulated frame after Restart used stale network NVRAM'
    for _ in range(2): core.retro_run()
    assert any(packet[:4] == b'TGMN' and struct.unpack_from('<H', packet, 6)[0] == 2
               for packet in packets), 'Open lobby callback stopped after Restart'
    linked = saved()
    assert (linked[65613], linked[65635]) == (2, 5), \
        'Active automation did not select Slave and Pink after Restart'

    packets.clear()
    core.retro_reset()
    core.retro_run()
    assert any(packet[:4] == b'TGMN' and struct.unpack_from('<H', packet, 6)[0] == 2
               for packet in packets), 'Restart lost the existing lobby callbacks'

    packets.clear()
    options[b'tgpulse_next_linked_cabinets_vr'] = b'3'
    changed[0] = True
    core.retro_reset()
    for _ in range(3):
        core.retro_run()
    assert any(packet[:4] == b'TGMN' and struct.unpack_from('<H', packet, 6)[0] == 3
               for packet in packets), 'Cabinet total did not update in open lobby'

    options[b'tgpulse_next_linked_cabinets_vr'] = b'disabled'
    changed[0] = True
    core.retro_reset()
    assert core.retro_serialize_size() > 0, 'Restart did not remove COMM board'
    for _ in range(3):
        core.retro_run()
    offline = saved()
    assert offline[65613] == 0 and offline[65635] == 0, \
        'Active automation did not restore the reviewed offline role/color'

    # The user's actual sequence resumes content after changing both options;
    # it does not invoke RetroArch Restart between the menu and the next frame.
    packets.clear()
    options[b'tgpulse_next_linked_cabinets_vr'] = b'2'
    options[b'tgpulse_next_automatic_network_vr'] = b'pink'
    changed[0] = True
    core.retro_run()
    assert core.retro_serialize_size() == 0, 'Resume did not fit COMM before frame one'
    resumed = saved()
    assert (resumed[65613], resumed[65635]) == (2, 5), \
        'Resume ran with the previous offline operator role'
    assert any(packet[:4] == b'TGMN' and struct.unpack_from('<H', packet, 6)[0] == 2
               for packet in packets), 'Resume did not update the open lobby total'
    receive(packet, len(early_hello), 1)
    core.retro_run()

    options[b'tgpulse_next_linked_cabinets_vr'] = b'disabled'
    changed[0] = True
    core.retro_run()
    assert core.retro_serialize_size() > 0, 'Resume did not remove COMM'
    assert saved()[65613] == 0, 'Resume did not restore No Link'
    core.retro_unload_game()
    core.retro_deinit()
    result = {'open_lobby_callbacks_preserved': True, 'restart_totals': [1, 2, 3, 1],
              'comm_board_fitted_and_removed': True,
              'linked_slave_role_applied_before_first_frame': True,
              'resume_reconfigures_comm_before_first_frame': True,
              'staggered_lobby_join_recovers': True,
              'offline_role_restored_by_automation': True}
    (args.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result))


if __name__ == '__main__':
    main()
