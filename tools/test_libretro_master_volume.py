#!/usr/bin/env python3
"""Verify live Master Volume PCM progression by replaying the same machine snapshot.

Use a complete vr.zip and a new JSON output. The test neither imports nor writes
user saves, and its frontend explicitly selects Software. A 600-frame warmup
and identical 30-frame intervals separate gain changes from game progression.
"""
import ctypes as c,runpy,array,math,json,argparse
from pathlib import Path
api=runpy.run_path(str(Path(__file__).with_name('libretro_nvram_capture.py')))
ENV,VIDEO,BATCH,POLL,INPUT,Game=[api[k] for k in ('ENV','VIDEO','BATCH','POLL','INPUT','GameInfo')]
class Variable(c.Structure):_fields_=[('key',c.c_char_p),('value',c.c_char_p)]
parser=argparse.ArgumentParser(description=__doc__)
for name in ('core','rom','output'):parser.add_argument('--'+name,type=Path,required=True)
args=parser.parse_args()
if args.output.exists():parser.error('Output must be a new JSON file')
args.output.parent.mkdir(parents=True,exist_ok=True)
lib=c.CDLL(str(args.core.resolve(strict=True)))
level=[b'100'];pcm=bytearray()
@ENV
def env(cmd,data):
 if cmd==15:
  v=c.cast(data,c.POINTER(Variable)).contents;v.value={b'tgpulse_next_renderer':b'software',b'tgpulse_next_volume':level[0]}.get(v.key);return v.value is not None
 if cmd==17:c.cast(data,c.POINTER(c.c_bool))[0]=True;return True
 if cmd==52:c.cast(data,c.POINTER(c.c_uint))[0]=2;return True
 return cmd in (10,11,16,18,35,37,67,69)
@VIDEO
def video(*args):pass
@BATCH
def batch(data,count):pcm.extend(c.string_at(data,count*4));return count
@POLL
def poll():pass
@INPUT
def state(*args):return 0
for name,cb in [('environment',env),('video_refresh',video),('audio_sample_batch',batch),('input_poll',poll),('input_state',state)]:
 fn=getattr(lib,'retro_set_'+name);fn.argtypes=[type(cb)];fn(cb)
lib.retro_load_game.argtypes=[c.POINTER(Game)];lib.retro_load_game.restype=c.c_bool
lib.retro_serialize_size.restype=c.c_size_t
for name in ('serialize','unserialize'):
 fn=getattr(lib,'retro_'+name);fn.argtypes=[c.c_void_p,c.c_size_t];fn.restype=c.c_bool
lib.retro_init();assert lib.retro_load_game(c.byref(Game(str(args.rom.resolve(strict=True)).encode(),None,0,None)))
try:
 for i in range(600):lib.retro_run()
 size=lib.retro_serialize_size();snapshot=c.create_string_buffer(size);assert lib.retro_serialize(snapshot,size)
 results=[];captures={}
 for value in (0,10,50,90,100,110,150,200,400,800):
  assert lib.retro_unserialize(snapshot,size);level[0]=str(value).encode();pcm.clear()
  for i in range(30):lib.retro_run()
  samples=array.array('h');samples.frombytes(pcm);captures[value]=list(samples)
  results.append({'volume':value,'samples':len(samples),'peak':max(map(abs,samples)),'rms':round(math.sqrt(sum(s*s for s in samples)/len(samples)),3),'clipped':sum(s in (-32768,32767) for s in samples)})
 base=captures[100];assert max(map(abs,base))>0
 for value,samples in captures.items():
  expected=[max(-32768,min(32767,math.trunc(s*value/100))) for s in base]
  assert samples==expected,(value,'nonlinear output')
 assert all(a['rms']<=b['rms'] for a,b in zip(results,results[1:]))
 args.output.write_text(json.dumps(results,indent=2)+'\n')
 print(json.dumps({'linear_gain_verified':True,'monotonic_rms':True,'measurements':results}))
finally:lib.retro_unload_game();lib.retro_deinit()
