#!/usr/bin/env python3
"""Audit photographed NetMerc values, committed samples and fresh-load menus.

Reuse the existing PNG decoder and Save RAM container validator. Native menu
readback is required; the EEPROM CRC used by other Model 1 sets is not assumed.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tomllib

from audit_model1_game_system_campaign import pixels
from libretro_nvram_capture import validate_save

ROOT = Path(__file__).resolve().parents[1]


def region(path, y):
    (width, height), rgb = pixels(path)
    assert (width,height) == (496,384)
    return b''.join(rgb[(row*width+155)*3:(row*width+420)*3] for row in range(y,y+16))


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--evidence',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args(); base=args.evidence
    doc=json.loads(subprocess.check_output(['ruby','-ryaml','-rjson','-e',
        'puts JSON.generate(YAML.safe_load(File.read(ARGV[0]), permitted_classes: [], aliases: true))',
        str(ROOT/'data/diagnostic-menus/netmerc.yaml')],text=True))
    fields=[o for page in doc['menus'].values() for o in page['options']]
    assert len(fields)==4 and sum(len(o['observed_values']) for o in fields)==20
    coords={'difficulty':128,'country':152,'advertise':176,'coin_start':136}
    offsets={'difficulty':[0x28,0x2a],'country':[0x5c],'advertise':[0x2c],'coin_start':[0x30,0x34,0x6c]}
    initial=(base/'baseline/base.srm').read_bytes();validate_save(initial,'netmerc')
    initial_hash=hashlib.sha256(initial).hexdigest()
    count=0; mappings={};reports=[]
    for option in fields:
        key=option['key'];values=[]
        # The photographed cycle must return to the same field value.
        assert region(base/f'survey/{key}/value-0.png',coords[key]) == region(base/f'survey/{key}/value-{len(option["observed_values"])}.png',coords[key])
        for step,label in enumerate(option['observed_values']):
            stem=f'{key}-{step}';recipe=tomllib.loads((base/'recipes'/f'{stem}.toml').read_text())
            saved=base/'samples'/stem;reload=base/'reloads'/stem
            before=json.loads((saved/'manifest.json').read_text());after=json.loads((reload/'manifest.json').read_text())
            assert before['status']==after['status']=='complete'
            assert before['recipe']==recipe and recipe['expected_value']==label
            assert before['import_sha256']==initial_hash
            for m in [before,after]:
                assert all(m['core_options'][k]=='disabled' for k in ['tgpulse_next_initial_nvram_setup','tgpulse_next_nvram_settings'])
            a=(saved/'saved.srm').read_bytes();b=(reload/'reloaded.srm').read_bytes()
            validate_save(a,'netmerc');validate_save(b,'netmerc')
            assert after['import_sha256']==hashlib.sha256(a).hexdigest()
            expected=region(base/f'survey/{key}/value-{step}.png',coords[key])
            assert region(saved/'selected.png',coords[key])==expected,(stem,'selection differs')
            assert region(reload/'reloaded.png',coords[key])==expected,(stem,'reload differs')
            assert a[65600:]==b[65600:]==bytes([255])*128
            # Check all four primary settings after reboot, including unrelated ones.
            for primary in [0x2a,0x5c,0x2c,0x34]:
                assert a[64+primary]==b[64+primary],(stem,'persisted field differs')
                if primary not in offsets[key]:assert a[64+primary]==initial[64+primary],(stem,'unrelated setting changed')
            native=[a[64+n] for n in offsets[key]]
            assert native==[b[64+n] for n in offsets[key]],(stem,'derived field differs')
            values.append(native);count+=1
            reports.append({'sample':stem,'value':label,'native_bytes':native,'saved_sha256':hashlib.sha256(a).hexdigest(),'fresh_menu_readback':True})
        mappings[key]={'backup_offsets':[hex(n) for n in offsets[key]],'values':values}
    sample=(base/'calibration/sample/saved.srm').read_bytes();reload=(base/'calibration/reload/reloaded.srm').read_bytes()
    validate_save(sample,'netmerc');validate_save(reload,'netmerc')
    positions=[0x3c,0x38,0x40,0x44];endpoints=[sample[64+n] for n in positions]
    assert endpoints==[0,255,0,255]==[reload[64+n] for n in positions]
    for n in positions: assert sample[64+n:64+n+4] == reload[64+n:64+n+4]
    assert region(base/'calibration/sample/endpoint-3.png',144)==region(base/'calibration/reload/reloaded.png',144)
    report={'set':'netmerc','fields':4,'values':count,'mappings':mappings,'samples':reports,
            'controller_full_range_saved_and_reloaded':True,'controller_endpoints':endpoints,
            'native_integrity':'All photographed settings and controller endpoints retained by native firmware after fresh import; no EEPROM CRC is claimed.',
            'container_crc':'verified for every saved and reloaded sample','passed':True}
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'fields':4,'values':count,'calibration':True,'passed':True}))


if __name__=='__main__':main()
