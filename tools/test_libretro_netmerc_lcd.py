#!/usr/bin/env python3
"""Focused U7 ABI/resource checks using existing isolated frontend hosts.

Requires user-owned content/BIOSes. Fixtures stay under a new output directory;
no personal saves/configuration or source ZIPs are modified.
"""
import argparse
import ctypes as c
import hashlib
import json
from pathlib import Path
from zipfile import ZipFile, ZIP_STORED
from libretro_nvram_capture import GameInfo
from test_libretro_netmerc_audio import AudioHost

KEY = b'tgpulse_next_netmerc_diagnostic_display'

class Value(c.Structure):
    _fields_ = [('value', c.c_char_p), ('label', c.c_char_p)]
class Definition(c.Structure):
    _fields_ = [(name, c.c_char_p) for name in ('key','desc','desc_categorized','info','info_categorized','category_key')] + [('values',Value*128),('default_value',c.c_char_p)]
class Options(c.Structure):
    _fields_ = [('categories',c.c_void_p),('definitions',c.POINTER(Definition))]
class LCDHost(AudioHost):
    def __init__(self, core, system):
        self.definitions = {}
        self.option_values = {}
        super().__init__(core, system)
        self.extended = True
    def environment(self, command, data):
        if command == 67:
            definitions = c.cast(data,c.POINTER(Options)).contents.definitions
            i=0
            while definitions[i].key:
                d=definitions[i]
                self.definitions[d.key.decode()] = (d.desc.decode(),d.default_value.decode(),d.category_key.decode() if d.category_key else None)
                self.option_values[d.key.decode()] = [(v.value.decode(), v.label.decode()) for v in d.values if v.value]
                i+=1
        return super().environment(command,data)
    def lcd(self, mode):
        self.values[KEY]=mode.encode(); self.changed=True
    def notices(self):
        return [m[0] for m in self.extended_messages if 'HD44780 BIOS' in m[0]]

def write_zip(path, files):
    with ZipFile(path,'w',ZIP_STORED) as z:
        for name,data in files.items(): z.writestr(name,data)

def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ('core','rom','bios-dir','output'):p.add_argument('--'+name,type=Path,required=True)
    a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
    with ZipFile(a.rom) as z:original={n:z.read(n) for n in z.namelist()}
    original.pop('hd44780_a00.bin',None)
    with ZipFile(a.bios_dir/'hd44780.zip') as z:font=z.read('hd44780_a00.bin')
    assert len(font)==4096 and hashlib.sha1(font).hexdigest()=='65cf075a988cdcbb316b9afdd0529b374a1a65ec'
    checks=[]
    def fixture(name, no_io=False):
        root=out/name;root.mkdir();system=root/'system';(system/'tgpulse-next').mkdir(parents=True)
        chips=dict(original)
        if no_io:chips.pop('epr-18021.6')
        game=root/'netmerc.zip';write_zip(game,chips)
        return root,system,game,chips
    root,system,game,_=fixture('missing-font')
    host=LCDHost(a.core,system)
    assert host.definitions[KEY.decode()]==('Sega NetMerc Diagnostic Display','off','video')
    assert host.option_values[KEY.decode()]==[('off','OFF'),('overlay','ON (100%)'),('overlay_half','ON (50%)')]
    assert host.definitions['tgpulse_next_netmerc_diagnostic_position'][1]=='top_right'
    assert host.definitions['tgpulse_next_netmerc_diagnostic_opacity'][1]=='80'
    assert not any('diagnostic_rendering' in k for k in host.definitions)
    host.load(game);host.run(120);assert not host.notices()
    host.lcd('overlay');host.run(2);assert len(host.notices())==1
    host.lcd('overlay_half');host.run(30);assert len(host.notices())==1
    host.lcd('off');host.run(1);assert len(host.notices())==1
    host.lcd('overlay');host.run(1);assert len(host.notices())==2
    host.lib.retro_reset();host.run(1);assert len(host.notices())==2
    assert all(host.displays[k] for k in host.definitions if 'diagnostic_' in k)
    host.close();checks.append('Off silent; missing font queues one notification per activation; reset does not repeat it; options always visible')
    for location in ('system','adjacent','game','invalid-system'):
        root,system,game,chips=fixture(location,no_io=True)
        # Proves strict identification permits external I/O without relaxing game chips.
        (system/'tgpulse-next'/'model1io2.zip').write_bytes((a.bios_dir/'model1io2.zip').read_bytes())
        if location=='system':write_zip(system/'tgpulse-next'/'hd44780.zip',{'hd44780_a00.bin':font})
        elif location in ('adjacent','invalid-system'):
            write_zip(root/'hd44780.zip',{'hd44780_a00.bin':font})
            if location=='invalid-system':write_zip(system/'tgpulse-next'/'hd44780.zip',{'hd44780_a00.bin':bytes(4096)})
        else:
            chips['hd44780_a00.bin']=font;write_zip(game,chips)
        host=LCDHost(a.core,system);host.load(game);host.run(120);state=host.snapshot()
        off=host.run(1);native=host.snapshot();host.restore(state)
        host.lcd('overlay');on=host.run(1)
        assert not host.notices()
        assert off['video_sha256']!=on['video_sha256'],location
        assert off['audio_sha256']==on['audio_sha256'] and native==host.snapshot(),location
        host.restore(state);host.lcd('overlay_half');half=host.run(1)
        assert half['video_sha256'] not in (off['video_sha256'],on['video_sha256']),location
        assert half['audio_sha256']==off['audio_sha256'] and host.snapshot()==native,location
        replay=host.snapshot();first=host.run(2);host.restore(replay);assert host.run(2)==first
        host.close();checks.append(location+': validated full/half HD44780 composition and state/audio preservation with System I/O BIOS')
    root,system,game,_=fixture('missing-io',no_io=True)
    host=LCDHost(a.core,system)
    info=GameInfo(str(game).encode(),None,0,None)
    assert not host.lib.retro_load_game(c.byref(info))
    assert any('Required I/O BIOS model1io2.zip' in m[0] for m in host.extended_messages)
    host.close();checks.append('Required I/O absence rejected without synthesizing firmware')
    report={'core_sha256':hashlib.sha256(a.core.read_bytes()).hexdigest(),'checks':checks,'physical_gameplay_acceptance':False}
    (out/'report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
