#!/usr/bin/env python3
"""Verify reviewed Model 1 seeding, save precedence and live settings via the ABI.

Uses existing samples, catalogue and workbook; never writes user save files.
Hardware rendering is disabled here. Real RetroArch delivery is a separate gate.
"""
import argparse
import ctypes as c
import hashlib
import json
from pathlib import Path
import subprocess
from generate_model1_nvram import ROOT, layout, patches, repair, workbook_rows, validate_native
from libretro_nvram_capture import ENV, VIDEO, BATCH, POLL, INPUT, GameInfo, validate_save

class Variable(c.Structure):
    _fields_=[('key',c.c_char_p),('value',c.c_char_p)]
class Display(c.Structure):
    _fields_=[('key',c.c_char_p),('visible',c.c_bool)]


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--core',type=Path,required=True)
    p.add_argument('--rom-dir',type=Path,required=True)
    p.add_argument('--samples',type=Path,default=ROOT/'validation/nvram-campaigns/2026-10-01')
    p.add_argument('--output',type=Path,required=True)
    a=p.parse_args();a.output.mkdir(parents=True,exist_ok=False)
    sheets=workbook_rows(ROOT/'docs/model1_core_options_review.xlsx')
    setups={r['A']:r for r in sheets['Automatic setup'][1:] if r.get('G')=='Approved'}
    selected={name:[r['C'] for r in sheets['Core options'][1:] if r.get('B')==name and r.get('F')=='Yes'] for name in setups}
    docs=json.loads(subprocess.check_output(['ruby','-ryaml','-rjson','-e','puts JSON.generate(ARGV.map{|p| YAML.safe_load(File.read(p), permitted_classes: [], aliases: true)})',*[str(ROOT/f'data/diagnostic-menus/{s}.yaml') for s in setups]],text=True))
    core=c.CDLL(str(a.core.resolve())); opts={}; changed=[False]; visible={}; frames=[0]; messages=[];display_callback=[None]
    @ENV
    def env(cmd,data):
        if cmd==15:
            v=c.cast(data,c.POINTER(Variable)).contents;v.value=opts.get(v.key);return v.value is not None
        if cmd==17:
            c.cast(data,c.POINTER(c.c_bool))[0]=changed[0];changed[0]=False;return True
        if cmd==52:c.cast(data,c.POINTER(c.c_uint))[0]=2;return True
        if cmd==69:
            address=c.cast(data,c.POINTER(c.c_void_p))[0]
            display_callback[0]=c.CFUNCTYPE(c.c_bool)(address);return True
        if cmd==55:
            v=c.cast(data,c.POINTER(Display)).contents;visible[v.key.decode()]=v.visible;return True
        if cmd==6:
            messages.append(c.cast(data,c.POINTER(c.c_char_p))[0].decode());return True
        if cmd==10:return c.cast(data,c.POINTER(c.c_uint))[0]==1
        return cmd in (11,16,18,35,37,67)
    @VIDEO
    def video(data,w,h,pitch):
        assert data and data!=c.c_void_p(-1).value and (w,h,pitch)==(496,384,1984)
        frames[0]+=1
    @BATCH
    def batch(data,count):return count
    @POLL
    def poll():pass
    @INPUT
    def input_state(port,device,index,button):return 0
    for name,cb in [('environment',env),('video_refresh',video),('audio_sample_batch',batch),('input_poll',poll),('input_state',input_state)]:
        fn=getattr(core,'retro_set_'+name);fn.argtypes=[type(cb)];fn(cb)
    core.retro_load_game.argtypes=[c.POINTER(GameInfo)];core.retro_load_game.restype=c.c_bool
    core.retro_get_memory_data.argtypes=[c.c_uint];core.retro_get_memory_data.restype=c.c_void_p
    core.retro_get_memory_size.argtypes=[c.c_uint];core.retro_get_memory_size.restype=c.c_size_t
    core.retro_serialize_size.restype=c.c_size_t
    for name in ('serialize','unserialize'):
        fn=getattr(core,'retro_'+name);fn.argtypes=[c.c_void_p,c.c_size_t];fn.restype=c.c_bool

    def load(name,automatic=True):
        opts.clear();opts.update({b'tgpulse_next_renderer':b'software',b'tgpulse_next_initial_nvram_setup':b'enabled' if automatic else b'disabled'})
        changed[0]=False;visible.clear();messages.clear();core.retro_init()
        path=str((a.rom_dir/(name+'.zip')).resolve(strict=True)).encode()
        assert core.retro_load_game(c.byref(GameInfo(path,None,0,None))),name
        ptr=core.retro_get_memory_data(0);assert ptr and core.retro_get_memory_size(0)==65728
        assert c.string_at(ptr,65728)==bytes(65728),'Save pointer must stay blank before frontend import'
        return ptr

    def snapshot():
        size=core.retro_serialize_size();state=c.create_string_buffer(size)
        assert core.retro_serialize(state,size)
        return state,size

    def save(name):
        raw=c.string_at(core.retro_get_memory_data(0),65728);validate_save(raw,name);return raw

    def unload():core.retro_unload_game();core.retro_deinit()

    results=[]
    for doc in docs:
        name=doc['game']['set'];spec=layout(doc)
        all_fields=[f for menu in doc['menus'].values() for f in menu.get('options',[])]
        fields=[next(f for f in all_fields if f['label']==label) for label in selected[name]]
        baseline=(a.samples/name/'samples'/f'{name}--{all_fields[0]["key"]}--step-00'/'saved.srm').read_bytes();validate_save(baseline,name)
        expected=bytearray(baseline[65600:])
        if ' = ' in setups[name]['E']:
            for text in setups[name]['E'].split('; '):
                label,value=text.split(' = ');field=next(f for f in all_fields if f['label']==label)
                step=next(i for i,v in enumerate(field['observed_values']) if str(v)==value)
                for offset,byte in patches(field,step):expected[offset]=byte
        repair(expected,spec)
        load(name);snapshot();raw=save(name)
        assert raw[64:65600]==baseline[64:65600] and raw[65600:]==expected,'Set-specific automatic image differs'
        assert visible['tgpulse_next_nvram_settings']
        assert not any(v for k,v in visible.items() if k.startswith('tgpulse_next_nvram_') and k!='tgpulse_next_nvram_settings')
        unload()
        # Existing frontend save wins over automatic setup, including its country/cabinet.
        ptr=load(name);c.memmove(ptr,baseline,len(baseline));snapshot();assert save(name)==baseline,'Existing save overwritten'
        # Live enabling patches only reviewed values, then resets to load firmware caches.
        opts[b'tgpulse_next_nvram_settings']=b'enabled';expected=bytearray(baseline[65600:])
        for field in fields:
            index=(field['observed_values'].index(field['native_default'])+1)%len(field['observed_values'])
            opts[f'tgpulse_next_nvram_{name}_{field["key"]}'.encode()]=str(field['observed_values'][index]).encode()
            for offset,byte in patches(field,index):expected[offset]=byte
        repair(expected,spec);changed[0]=True
        assert display_callback[0] and display_callback[0]()
        assert all(visible[f'tgpulse_next_nvram_{name}_{f["key"]}'] for f in fields),'Fields must appear while the menu is paused'
        core.retro_run();patched=save(name)
        assert patched[65600:]==expected,'Live operator values differ'
        validate_native(patched[65600:],spec)
        assert all(visible[f'tgpulse_next_nvram_{name}_{f["key"]}'] for f in fields)
        assert not any(v for k,v in visible.items() if k.startswith('tgpulse_next_nvram_') and k!='tgpulse_next_nvram_settings' and not k.startswith('tgpulse_next_nvram_'+name+'_'))
        # Explicit reset keeps saved fields. State load restores exact saved EEPROM.
        state,size=snapshot();core.retro_reset();core.retro_run();assert save(name)[65600:]==expected
        opts[b'tgpulse_next_nvram_settings']=b'disabled';changed[0]=True;assert display_callback[0]();core.retro_run();assert save(name)[65600:]==expected
        assert core.retro_unserialize(state,size);assert save(name)[65600:]==expected
        assert all(not visible[f'tgpulse_next_nvram_{name}_{f["key"]}'] for f in fields)
        unload()
        # Disabling automatic setup retains virgin machine defaults, never a template.
        load(name,False);snapshot();virgin=save(name);assert virgin[65600:]!=raw[65600:];unload()
        results.append({'set':name,'fields':len(fields),'automatic_image':True,'existing_save_precedence':True,'live_selection':True,'reset_state_preservation':True,'disabled_preserves':True,'visibility':True,'automatic_disabled':True})
    report={'core_sha256':hashlib.sha256(a.core.read_bytes()).hexdigest(),'frames':frames[0],'sets':results}
    (a.output/'result.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))


if __name__=='__main__':main()
