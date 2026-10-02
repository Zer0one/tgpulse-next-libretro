#!/usr/bin/env python3
"""Isolated Model 1 linked-cabinet runs using the SM2 multi-instance procedure.

Reuse test_retroarch_gpu for config, bounded execution and evidence. Each
cabinet gets its own frontend paths and reviewed operator choices. This runner
never writes a user's ROM, save or global configuration. No synthetic COMM.
"""
import argparse
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import sys
import time


def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ('retroarch','core','rom','output'):p.add_argument('--'+name,type=Path,required=True)
    p.add_argument('--moltenvk',type=Path)
    p.add_argument('--cabinets',type=int,default=2,choices=range(2,10))
    p.add_argument('--live',action='store_true',help='Last VR/VFormula instance is a LIVE relay')
    p.add_argument('--frames',type=int,default=2400)
    p.add_argument('--timeout',type=int,default=60)
    a=p.parse_args();name=a.rom.stem
    if name not in ('vr','vformula','wingwar','wingwaru','wingwarj','wingwar360'):p.error('Set has no M1COMM')
    if name.startswith('wingwar') and (a.cabinets!=2 or a.live):p.error('Wing War requires Master/Slave only')
    if a.cabinets==9 and not a.live:p.error('Nine total cabinets require one LIVE relay')
    out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
    # Probe an available loopback port; no frontend or core socket is opened here.
    with socket.socket() as probe:probe.bind(('127.0.0.1',0));port=probe.getsockname()[1]
    processes=[];results=[]
    base=[sys.executable,str(Path(__file__).with_name('test_retroarch_gpu.py')),'--retroarch',str(a.retroarch.resolve()),'--core',str(a.core.resolve()),'--rom',str(a.rom.resolve()),'--renderer','software','--driver','vulkan','--frames',str(a.frames),'--timeout',str(a.timeout),'--linked-cabinets',str(a.cabinets),'--netplay-port',str(port),'--nvram-settings','enabled']
    if a.moltenvk:base+=['--moltenvk',str(a.moltenvk.resolve())]
    colors=['RED','BLUE','YELLOW','GREEN','BLACK','PINK','SKYBLUE','ORANGE']
    try:
        for index in range(a.cabinets):
            role='MASTER' if index==0 else 'LIVE' if a.live and index==a.cabinets-1 else 'SLAVE'
            dest=out/f'{index}-{role.lower()}'
            cmd=base+['--output',str(dest),'--netplay-role','host' if index==0 else 'client']
            if name in ('vr','vformula'):
                cmd+=['--nvram-setting',f'tgpulse_next_nvram_{name}_link_id={role}']
                field='car_color' if name=='vr' else 'car_number'
                color=colors[index] if role!='LIVE' else colors[0]
                value=color if name=='vr' else f'NO.{index+1} ({color})' if role!='LIVE' else 'NO.1 (RED)'
                cmd+=['--nvram-setting',f'tgpulse_next_nvram_{name}_{field}={value}']
                if name=='vr':cmd+=['--nvram-setting','tgpulse_next_nvram_vr_cabinet=SPECIAL']
            else:cmd+=['--nvram-setting',f'tgpulse_next_nvram_{name}_network={role}']
            log=(out/f'{index}-launcher.log').open('wb')
            process=subprocess.Popen(cmd,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
            processes.append((process,log,dest,role))
            (out/'processes.json').write_text(json.dumps([{'pid':proc.pid,'role':r,'output':str(d)} for proc,_,d,r in processes],indent=2))
            time.sleep(0.5)
        deadline=time.monotonic()+a.timeout+10
        while any(proc.poll() is None for proc,_,_,_ in processes):
            if time.monotonic()>deadline:raise TimeoutError('Linked run exceeded bound')
            time.sleep(0.2)
        for proc,_,dest,role in processes:
            log=(dest/'run.log').read_text(errors='replace')
            result={'role':role,'launcher_exit':proc.returncode,'roster_ready':'Roster ready:' in log,'game_online_600':'Game link online for 600 consecutive frames' in log,'comm_lines':[line for line in log.splitlines() if 'COMM status/' in line],'report':str(dest/'result.json')}
            results.append(result)
        (out/'linked-report.json').write_text(json.dumps({'set':name,'port':port,'cabinets':a.cabinets,'live':a.live,'instances':results},indent=2)+'\n')
        print(json.dumps(results))
        if not all(r['launcher_exit']==0 and r['roster_ready'] and r['game_online_600'] for r in results):raise RuntimeError('Not all cabinets reached sustained game-created COMM online; inspect isolated logs')
    finally:
        for proc,log,_,_ in processes:
            if proc.poll() is None:
                os.killpg(proc.pid,signal.SIGTERM)
                try:proc.wait(timeout=3)
                except subprocess.TimeoutExpired:os.killpg(proc.pid,signal.SIGKILL);proc.wait()
            log.close()

if __name__=='__main__':main()
