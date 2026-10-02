#!/usr/bin/env python3
"""Check Model 1 hardware frames in a real EGL frontend, adapting SM2's runner.

Frontend readback is verification only; the core's normal path stays on GPU.
The output directory must be new and stores no ROMs.
"""
import argparse
import ctypes as c
import hashlib
import json
from pathlib import Path
import runpy

api = runpy.run_path(str(Path(__file__).with_name('libretro_nvram_capture.py')))
ENV, VIDEO, BATCH, POLL, INPUT, Game = [api[k] for k in ('ENV', 'VIDEO', 'BATCH', 'POLL', 'INPUT', 'GameInfo')]

class Variable(c.Structure):
    _fields_ = [('key', c.c_char_p), ('value', c.c_char_p)]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ('core', 'host', 'rom', 'output'):
        p.add_argument('--' + name, type=Path, required=True)
    p.add_argument('--api', choices=('desktop', 'gles', 'software'), default='desktop')
    p.add_argument('--frames', type=int, default=180)
    p.add_argument('--context-cycle', type=int, default=0)
    p.add_argument('--context-loss-cycle', type=int, default=0)
    p.add_argument('--overlay', action='store_true')
    p.add_argument('--aspect', choices=('auto','4_3','16_9'), default='auto')
    p.add_argument('--widescreen', choices=('stretch','expand_3d','expand_3d_2d'), default='stretch')
    p.add_argument('--supersampling', type=int, choices=range(1,5), default=1)
    p.add_argument('--software-warmup', type=int, default=0, help='Warm deterministic machine state in Software, then reload the chosen GPU path')
    p.add_argument('--nvram', type=Path, help='Validated same-set Save RAM fixture; unchanged source')
    a = p.parse_args()
    if a.frames < 1 or (a.context_cycle and a.context_loss_cycle):
        p.error('Use positive frames and only one context cycle type')
    out = a.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    core = c.CDLL(str(a.core.resolve())); host = c.CDLL(str(a.host.resolve()))
    host.tgpulse_gl_set_es.argtypes = [c.c_bool]; host.tgpulse_gl_set_es(a.api == 'gles')
    host.tgpulse_gl_environment.argtypes = [c.c_uint, c.c_void_p]; host.tgpulse_gl_environment.restype = c.c_bool
    host.tgpulse_gl_start.restype = c.c_bool; host.tgpulse_gl_error.restype = c.c_char_p
    host.tgpulse_gl_frame.argtypes = [c.c_uint, c.c_uint, c.c_bool]; host.tgpulse_gl_frame.restype = c.c_bool
    host.tgpulse_gl_pixels.restype = c.c_void_p
    width = 683 if a.api != 'software' and a.aspect == '16_9' and a.widescreen != 'stretch' else 496
    warming = [False]
    options = {b'tgpulse_next_aspect_ratio':a.aspect.encode(), b'tgpulse_next_widescreen_mode':a.widescreen.encode(), b'tgpulse_next_supersampling':str(a.supersampling).encode(), b'tgpulse_next_renderer': b'software' if a.api == 'software' else b'opengl',
               b'tgpulse_next_timing_overlay': b'auto' if a.overlay else b'disabled'}
    shutdown = []; pixels = []; pcm = bytearray(); frames = [0]; errors = []
    @ENV
    def env(cmd, data):
        if cmd == 15:
            v = c.cast(data, c.POINTER(Variable)).contents; v.value = options.get(v.key); return v.value is not None
        if cmd == 17:
            c.cast(data, c.POINTER(c.c_bool))[0] = False; return True
        if cmd == 7: shutdown.append(True); return True
        if cmd == 52:
            c.cast(data, c.POINTER(c.c_uint))[0] = 2; return True
        if cmd == 10: return c.cast(data, c.POINTER(c.c_uint))[0] == 1
        if cmd in (11, 16, 18, 35, 37, 67, 69): return True
        return host.tgpulse_gl_environment(cmd, data) if a.api != 'software' else False
    @VIDEO
    def video(data, w, h, pitch):
        if warming[0]: return
        frames[0] += 1
        if (w, h) != (width, 384): errors.append('Incorrect frame geometry'); return
        if a.api == 'software':
            if not data or data == c.c_void_p(-1).value: errors.append('Missing software pixels'); return
            if frames[0] == a.frames:
                raw = c.string_at(data, h*pitch)
                rgb = bytearray(w*h*3)
                for y in range(h):
                    row = raw[y*pitch:y*pitch+w*4]; offset=y*w*3
                    rgb[offset:offset+w*3:3]=row[2::4]; rgb[offset+1:offset+w*3:3]=row[1::4]; rgb[offset+2:offset+w*3:3]=row[0::4]
                pixels.append(bytes(rgb))
            return
        if data != c.c_void_p(-1).value or pitch != 0: errors.append('Invalid hardware frame'); return
        capture = frames[0] == a.frames
        if not host.tgpulse_gl_frame(w, h, capture): errors.append(host.tgpulse_gl_error().decode()); return
        if capture:
            raw = c.string_at(host.tgpulse_gl_pixels(), w*h*4)
            rows = b''.join(raw[y*w*4:(y+1)*w*4] for y in range(h-1, -1, -1))
            rgb=bytearray(w*h*3);rgb[0::3]=rows[0::4];rgb[1::3]=rows[1::4];rgb[2::3]=rows[2::4];pixels.append(bytes(rgb))
    @BATCH
    def batch(data, count): pcm.extend(c.string_at(data,count*4)); return count
    @POLL
    def poll(): pass
    @INPUT
    def state(port, device, index, button): return 0
    for name, cb in [('environment',env),('video_refresh',video),('audio_sample_batch',batch),('input_poll',poll),('input_state',state)]:
        fn=getattr(core,'retro_set_'+name); fn.argtypes=[type(cb)]; fn(cb)
    core.retro_load_game.argtypes=[c.POINTER(Game)];core.retro_load_game.restype=c.c_bool
    core.retro_get_memory_data.argtypes=[c.c_uint];core.retro_get_memory_data.restype=c.c_void_p
    core.retro_get_memory_size.argtypes=[c.c_uint];core.retro_get_memory_size.restype=c.c_size_t
    game=Game(str(a.rom.resolve()).encode(),None,0,None)
    core.retro_serialize_size.restype=c.c_size_t
    for name in ('serialize','unserialize'):
        fn=getattr(core,'retro_'+name);fn.argtypes=[c.c_void_p,c.c_size_t];fn.restype=c.c_bool
    core.retro_init(); loaded=False; context=False
    try:
        snapshot=None
        if a.software_warmup:
            assert 0 < a.software_warmup <= 6000
            backend=options[b'tgpulse_next_renderer'];options[b'tgpulse_next_renderer']=b'software';warming[0]=True
            loaded=core.retro_load_game(c.byref(game));assert loaded
            for _ in range(a.software_warmup):core.retro_run()
            size=core.retro_serialize_size();snapshot=c.create_string_buffer(size);assert core.retro_serialize(snapshot,size)
            core.retro_unload_game();loaded=False;options[b'tgpulse_next_renderer']=backend;warming[0]=False;pcm.clear()
        loaded=core.retro_load_game(c.byref(game)); assert loaded,'Content rejected'
        if a.nvram:
            raw=a.nvram.read_bytes();api['validate_save'](raw,a.rom.stem)
            assert len(raw)==core.retro_get_memory_size(0)
            c.memmove(core.retro_get_memory_data(0),raw,len(raw))
        if a.api!='software':
            context=host.tgpulse_gl_start(); assert context,host.tgpulse_gl_error()
        if snapshot is not None: assert core.retro_unserialize(snapshot,len(snapshot))
        for frame in range(a.frames):
            if frame and frame in (a.context_cycle,a.context_loss_cycle):
                if a.context_loss_cycle: host.tgpulse_gl_lose_context()
                else: host.tgpulse_gl_stop()
                context=host.tgpulse_gl_start(); assert context,host.tgpulse_gl_error()
            core.retro_run(); assert not shutdown and not errors,(frame,errors,shutdown)
        assert frames[0]==a.frames and pixels,'Missing output'
        image=f'P6\n{width} 384\n255\n'.encode()+pixels[-1];(out/'frame.ppm').write_bytes(image)
        nvram=c.string_at(core.retro_get_memory_data(0),core.retro_get_memory_size(0));(out/'save.srm').write_bytes(nvram)
        sha=lambda b:hashlib.sha256(b).hexdigest()
        report={'width':width,'aspect':a.aspect,'widescreen':a.widescreen,'supersampling':a.supersampling,'software_warmup':a.software_warmup,'api':a.api,'frames':frames[0],'hardware_frames':frames[0] if context else 0,'context_cycle':a.context_cycle,'context_loss_cycle':a.context_loss_cycle,'overlay':a.overlay,'image_sha256':sha(image),'audio_sha256':sha(pcm),'audio_frames':len(pcm)//4,'nvram_sha256':sha(nvram),'core_sha256':sha(a.core.read_bytes())}
        (out/'result.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))
    finally:
        if context: host.tgpulse_gl_stop()
        if loaded: core.retro_unload_game()
        core.retro_deinit()

if __name__=='__main__':main()
