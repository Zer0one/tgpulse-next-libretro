#!/usr/bin/env python3
"""Verify reviewed Model 1 seeding, save precedence and live settings via the ABI.

Uses existing samples, catalogue and workbook; never writes user save files.
Hardware rendering is disabled here. Real RetroArch delivery is a separate gate.
"""
import argparse
import binascii
import ctypes as c
import hashlib
import json
from pathlib import Path
import subprocess
from generate_model1_nvram import ROOT, layout, patches, repair, workbook_rows, validate_native, reviewed_option, startup_values, workbook_records
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
    p.add_argument('--netmerc-seed-rom',type=Path,
                   help='Also compare optional-file NetMerc initialization with a ZIP containing its seed')
    p.add_argument('--netmerc-existing-save',type=Path,
                   help='Verify save precedence and explicit overrides on an isolated same-set Save RAM copy')
    a=p.parse_args();a.output.mkdir(parents=True,exist_ok=False)
    sheets=workbook_rows(ROOT/'docs/model1_core_options_review.xlsx')
    setups={r['set']:r for r in workbook_records(sheets,'Automatic setup') if r['reviewer decision']=='Approved'}
    selected={name:[r['setting'] for r in workbook_records(sheets,'Core options') if r['set']==name and r['proposed nvram settings']=='Yes'] for name in setups}
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

    def load(name,automatic=True,rom=None):
        opts.clear();opts.update({b'tgpulse_next_renderer':b'software',b'tgpulse_next_initial_nvram_setup':b'enabled' if automatic else b'disabled'})
        changed[0]=False;visible.clear();messages.clear();core.retro_init()
        path=str((rom or a.rom_dir/(name+'.zip')).resolve(strict=True)).encode()
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
        if doc['game']['set'] == 'netmerc': continue
        name=doc['game']['set'];spec=layout(doc)
        all_fields=[f for menu in doc['menus'].values() for f in menu.get('options',[])]
        fields=[reviewed_option(all_fields,label) for label in selected[name]]
        baseline=(a.samples/name/'samples'/f'{name}--{all_fields[0]["key"]}--step-00'/'saved.srm').read_bytes();validate_save(baseline,name)
        expected=bytearray(baseline[65600:])
        for label,value in startup_values(all_fields,setups[name]['explicit startup values']).items():
            field=reviewed_option(all_fields,label)
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
        # Enabling overrides without saved selector choices must retain the
        # approved automatic policy, rather than revert to native defaults.
        startup=startup_values(all_fields,setups[name]['explicit startup values'])
        expected=bytearray(baseline[65600:])
        for field in fields:
            value=startup.get(field['label'],str(field['native_default']))
            index=next(i for i,v in enumerate(field['observed_values']) if str(v)==value)
            for offset,byte in patches(field,index):expected[offset]=byte
        repair(expected,spec)
        opts[b'tgpulse_next_nvram_settings']=b'enabled';changed[0]=True
        core.retro_run();assert save(name)[65600:]==expected, 'Default overrides differ from approved startup policy'
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
        results.append({'set':name,'fields':len(fields),'automatic_image':True,'existing_save_precedence':True,'default_overrides_match_startup':True,'live_selection':True,'reset_state_preservation':True,'disabled_preserves':True,'visibility':True,'automatic_disabled':True})
    # NetMerc uses backup RAM: verify approved encodings against native samples.
    doc=next(d for d in docs if d['game']['set']=='netmerc')
    all_fields=[f for menu in doc['menus'].values() for f in menu['options']]
    fields=[reviewed_option(all_fields,label) for label in selected['netmerc']]
    campaign=ROOT/'validation/nvram-campaigns/2026-10-04/netmerc'
    native=(campaign/'initialized-baseline/saved.srm').read_bytes()
    expected=bytearray(native[64:65600]);expected[0x5c]=2
    for offset,value in [(0x3c,255),(0x38,0),(0x40,0),(0x44,255)]:
        expected[offset:offset+4]=bytes([value,255,255,255])
    load('netmerc');snapshot();automatic=save('netmerc')
    (a.output/'netmerc-automatic.srm').write_bytes(automatic)
    if automatic[64:65600]!=expected:
        raise AssertionError([(hex(i),x,y) for i,(x,y) in enumerate(zip(automatic[64:65600],expected)) if x!=y][:20])
    assert automatic[65600:]==native[65600:]
    assert automatic[64+0x18:64+0x1a]==bytes(2), 'Automatic baseline must start with zero credits'
    assert visible['tgpulse_next_nvram_settings']
    unload()
    ptr=load('netmerc');c.memmove(ptr,native,len(native));snapshot()
    assert save('netmerc')==native, 'Existing NetMerc save overwritten'
    opts[b'tgpulse_next_nvram_settings']=b'enabled'
    changed[0]=True;core.retro_run()
    defaults=save('netmerc')
    assert defaults[64+0x5c]==2, 'Default Country must retain automatic Export'
    assert defaults[64+0x38:64+0x48]==native[64+0x38:64+0x48], 'Default overrides changed calibration'
    for field in fields:
        key=f'tgpulse_next_nvram_netmerc_{field["key"]}'.encode()
        for step,value in enumerate(field['observed_values']):
            opts[key]=str(value).encode();changed[0]=True
            assert display_callback[0];display_callback[0]();core.retro_run()
            raw=save('netmerc')
            for offset,byte in patches(field,step):assert raw[64+offset]==byte
            assert raw[65600:]==native[65600:], 'NetMerc EEPROM was changed'
            assert raw[64+0x30:64+0x38]==native[64+0x30:64+0x38], 'Coin settings were changed'
            assert raw[64+0x38:64+0x48]==native[64+0x38:64+0x48], 'Calibration was changed'
        opts[key]=str(field['native_default']).encode()
    state,size=snapshot();before=save('netmerc')
    core.retro_reset();snapshot();assert save('netmerc')==before
    opts[b'tgpulse_next_nvram_settings']=b'disabled';changed[0]=True;display_callback[0]();core.retro_run()
    disabled=save('netmerc')
    assert disabled[64+0x28:64+0x70]==before[64+0x28:64+0x70]
    assert core.retro_unserialize(state,size);assert save('netmerc')==before
    assert not any(visible[f'tgpulse_next_nvram_netmerc_{f["key"]}'] for f in fields)
    unload();load('netmerc',False);snapshot()
    virgin=bytearray([255]*65536);virgin[0x1000:0x3004]=bytes(0x2004);virgin[0]=0x0f
    assert save('netmerc')[64:65600]==virgin;unload()
    seed_verified=False
    if a.netmerc_seed_rom:
        import zipfile
        with zipfile.ZipFile(a.netmerc_seed_rom) as archive:
            factory=archive.read('netmerc_nvram.bin')
        initialized_factory=bytearray(factory)
        if hashlib.sha1(factory).hexdigest()=='411134c1e6307f2e32c3b4b372597b45b14a9834':
            initialized_factory[0x1000:0x3004]=bytes(0x2004);initialized_factory[0]=0x0f
            initialized_factory[0x18:0x1a]=bytes(2)
        expected_factory=initialized_factory.copy();expected_factory[0x5c]=2
        for offset,value in [(0x3c,255),(0x38,0),(0x40,0),(0x44,255)]:
            expected_factory[offset:offset+4]=bytes([value,255,255,255])
        load('netmerc',rom=a.netmerc_seed_rom);snapshot()
        assert save('netmerc')[64:65600]==expected_factory, 'Optional factory data overwritten'
        unload();load('netmerc',False,rom=a.netmerc_seed_rom);snapshot()
        assert save('netmerc')[64:65600]==initialized_factory, 'Fresh seed initialization differs'
        unload()
        ptr=load('netmerc',rom=a.netmerc_seed_rom);c.memmove(ptr,native,len(native));snapshot()
        assert save('netmerc')==native, 'Optional seed initialization overrode a valid frontend save'
        unload();seed_verified=True
    existing_verified=False
    if a.netmerc_existing_save:
        personal=a.netmerc_existing_save.read_bytes();validate_save(personal,'netmerc')
        if a.netmerc_seed_rom:
            ptr=load('netmerc',rom=a.netmerc_seed_rom)
            c.memmove(ptr,personal,len(personal));snapshot()
            assert save('netmerc')==personal, 'Seed credit repair modified a valid saved credit counter'
            core.retro_reset();snapshot();assert save('netmerc')==personal
            unload()
        ptr=load('netmerc');c.memmove(ptr,personal,len(personal));snapshot()
        assert save('netmerc')==personal, 'Existing save received an initialization patch'
        assert 'Applied automatic initial NVRAM setup' not in messages
        opts[b'tgpulse_next_nvram_settings']=b'enabled'
        for field,value in [('difficulty','HARDEST'),('country','USA'),('advertise','ON')]:
            opts[f'tgpulse_next_nvram_netmerc_{field}'.encode()]=value.encode()
        changed[0]=True;core.retro_run()
        raw=save('netmerc')
        assert [raw[64+i] for i in (0x28,0x2a,0x5c,0x2c)]==[8,3,1,0]
        assert raw[64:68]==personal[64:68], 'Bookkeeping signature modified'
        assert raw[64+0x30:64+0x48]==personal[64+0x30:64+0x48], 'Coin/calibration modified'
        assert 'Applied NVRAM Settings; machine reset' in messages
        for _ in range(1250):core.retro_run()
        assert not any('layout not ready' in m for m in messages)
        unload();existing_verified=True
    netmerc={'fields':len(fields),'values':sum(len(f['observed_values']) for f in fields),
             'optional_seed_verified':seed_verified,'automatic_country_export':True,
             'existing_save_precedence':True,'all_values_preserve_coin_and_calibration':True,
             'reset_state_preservation':True,'disabled_preserves':True,'visibility':True}
    netmerc['existing_save_without_marker']=existing_verified
    report={'core_sha256':hashlib.sha256(a.core.read_bytes()).hexdigest(),'frames':frames[0],'sets':results,
            'netmerc_calibration':netmerc}
    (a.output/'result.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))


if __name__=='__main__':main()
