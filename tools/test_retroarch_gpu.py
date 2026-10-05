#!/usr/bin/env python3
"""Isolated, bounded RetroArch hardware-rendering check, following SM2's launcher.

An existing MoltenVK library may be supplied on macOS without altering the app.
Only this runner's process is terminated on timeout. Every output root is new.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time


def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ('retroarch','core','rom','output'):
        p.add_argument('--'+name,type=Path,required=True)
    p.add_argument('--moltenvk',type=Path)
    p.add_argument('--renderer',choices=('auto','vulkan','opengl','software'),default='vulkan')
    p.add_argument('--driver',choices=('vulkan','glcore','gl','metal'),default='vulkan')
    p.add_argument('--frames',type=int,default=180)
    p.add_argument('--vsync',action='store_true',help='Pace distributed cabinet runs to the display refresh rate')
    p.add_argument('--timeout',type=int,default=60)
    p.add_argument('--av-timing',choices=('native','60hz'),default='native')
    p.add_argument('--overlay',action='store_true')
    p.add_argument('--notifications',action='store_true',help='Enable frontend OSD for notification verification')
    p.add_argument('--diagnostic-display',choices=('off','overlay','overlay_half'),default='off')
    p.add_argument('--diagnostic-position',choices=('top_left','top_right','bottom_right','bottom_left'),default='top_right')
    p.add_argument('--diagnostic-opacity',type=int,choices=range(0,101,10),default=80)
    p.add_argument('--bios-dir',type=Path,help='Copy only Model 1 device BIOS ZIPs into isolated system/tgpulse-next/')
    p.add_argument('--aspect',choices=('auto','4_3','16_9'),default='auto')
    p.add_argument('--widescreen',choices=('stretch','expand_3d','expand_3d_2d'),default='stretch')
    p.add_argument('--supersampling',type=int,choices=range(1,5),default=1)
    for axis in ('steering','accelerator','brake'):
        p.add_argument('--'+axis+'-range',type=int,choices=range(50,151,10),default=100)
    p.add_argument('--steering-response',choices=('linear','progressive','fbneo'),default='linear')
    p.add_argument('--linked-cabinets',type=int,choices=range(1,10),default=1)
    p.add_argument('--netplay-role',choices=('host','client'))
    p.add_argument('--netplay-host',default='127.0.0.1')
    p.add_argument('--netplay-port',type=int,default=55435)
    p.add_argument('--nvram-sample',type=Path,help='Same-set Save RAM fixture; unchanged source, copied into isolated saves')
    p.add_argument('--initial-nvram',choices=('enabled','disabled'),default='enabled')
    p.add_argument('--nvram-settings',choices=('enabled','disabled'),default='disabled')
    p.add_argument('--nvram-setting',action='append',default=[],metavar='KEY=VALUE')
    p.add_argument('--replay',type=Path,help='Existing RetroArch replay-v1 input fixture')
    p.add_argument('--mvd-input',choices=('auto','off','right_stick','sensors'),default='auto')
    p.add_argument('--audio-donor',choices=('vf','vr','swa','wingwar','off'),default='vf')
    p.add_argument('--mvd-horizontal-range',type=int,choices=range(0,91,10),default=30)
    p.add_argument('--mvd-vertical-range',type=int,choices=range(0,91,10),default=20)
    a=p.parse_args()
    if not 1<=a.frames<=36000 or not 1<=a.timeout<=600:p.error('Invalid bound')
    for file in (a.retroarch,a.core,a.rom):file.resolve(strict=True)
    if a.replay:a.replay.resolve(strict=True)
    out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
    for name in ('saves','states','screenshots','config','playlists','system','runtime'):(out/name).mkdir()
    if a.bios_dir:
        import shutil
        destination=out/'system'/'tgpulse-next';destination.mkdir()
        for name in ('hd44780.zip','model1io.zip','model1io2.zip'):
            source=a.bios_dir/name
            if source.is_file():shutil.copyfile(source,destination/name)
    env=os.environ.copy()
    if a.moltenvk:
        (out/'runtime'/'MoltenVK').symlink_to(a.moltenvk.resolve(strict=True))
        env['DYLD_LIBRARY_PATH']=str(out/'runtime')
    opts=out/'core-options.cfg';opts.write_text(f'tgpulse_next_renderer = "{a.renderer}"\ntgpulse_next_timing_overlay = "'+('auto' if a.overlay else 'disabled')+'"\n')
    with opts.open('a') as stream:
        stream.write(f'tgpulse_next_av_timing = "{a.av_timing}"\n')
        stream.write(f'tgpulse_next_netmerc_diagnostic_display = "{a.diagnostic_display}"\ntgpulse_next_netmerc_diagnostic_position = "{a.diagnostic_position}"\ntgpulse_next_netmerc_diagnostic_opacity = "{a.diagnostic_opacity}"\n')
        stream.write(f'tgpulse_next_netmerc_audio_donor = "{a.audio_donor}"\n')
        stream.write(f'tgpulse_next_aspect_ratio = "{a.aspect}"\ntgpulse_next_widescreen_mode = "{a.widescreen}"\ntgpulse_next_supersampling = "{a.supersampling}"\n')
    ranges={axis:getattr(a,axis+'_range') for axis in ('steering','accelerator','brake')}
    with opts.open('a') as stream:
        for axis,value in ranges.items():stream.write(f'tgpulse_next_{axis}_output_range = "{value}"\n')
    with opts.open('a') as stream:stream.write(f'tgpulse_next_steering_response = "{a.steering_response}"\n')
    with opts.open('a') as stream:
        stream.write(f'tgpulse_next_netmerc_mvd_input = "{a.mvd_input}"\ntgpulse_next_netmerc_mvd_horizontal_range = "{a.mvd_horizontal_range}"\ntgpulse_next_netmerc_mvd_vertical_range = "{a.mvd_vertical_range}"\n')
    if a.linked_cabinets>1:
        if a.rom.stem not in ('vr','vformula','wingwar','wingwaru','wingwarj','wingwar360'):p.error('Set has no native M1COMM board')
        if a.rom.stem.startswith('wingwar') and a.linked_cabinets!=2:p.error('Wing War supports two cabinets')
    if not 1<=a.netplay_port<=65535:p.error('Invalid Netplay port')
    with opts.open('a') as stream:stream.write(f'tgpulse_next_linked_cabinets_{a.rom.stem} = "'+(str(a.linked_cabinets) if a.linked_cabinets>1 else 'disabled')+'"\n')
    selected_nvram={}

    for selection in a.nvram_setting:
        key,value=selection.split('=',1)
        if not key.startswith('tgpulse_next_nvram_') or any(char in key+value for char in ('"','\n','\r')):p.error('Invalid NVRAM setting')
        selected_nvram[key]=value
    with opts.open('a') as stream:
        stream.write(f'tgpulse_next_initial_nvram_setup = "{a.initial_nvram}"\ntgpulse_next_nvram_settings = "{a.nvram_settings}"\n')
        for key,value in selected_nvram.items():stream.write(f'{key} = "{value}"\n')
    values={'video_driver':a.driver,'video_threaded':'false','video_fullscreen':'false','video_windowed_fullscreen':'false','video_scale':'1','video_gpu_screenshot':'true','video_font_enable':'false','video_vsync':'false','audio_enable':'false','pause_nonactive':'false','savestate_auto_save':'false','savestate_auto_load':'false','config_save_on_exit':'false','content_history_enable':'false','auto_overrides_enable':'false','auto_remaps_enable':'false','video_shader_enable':'false','core_options_path':opts,'system_directory':out/'system','savefile_directory':out/'saves','savestate_directory':out/'states','screenshot_directory':out/'screenshots','playlist_directory':out/'playlists','rgui_config_directory':out/'config'}
    if a.netplay_role:
        values.update(netplay_ip_port=str(a.netplay_port),netplay_max_connections=str(a.linked_cabinets),netplay_public_announce='false',netplay_use_mitm_server='false',netplay_nat_traversal='false',netplay_check_frames='0',netplay_nickname=out.name)
    if a.vsync:
        values['video_vsync']='true'
    values.update(sort_savefiles_enable='false',sort_savefiles_by_content_enable='false',sort_savestates_enable='false')
    for name in ('content_history_path','content_favorites_path','content_image_history_path','content_music_history_path','content_video_history_path'):values[name]=out/(name+'.lpl')
    # RGUI needs no external Ozone textures in this isolated launch.
    values.update(menu_driver='rgui',menu_show_start_screen='false',menu_enable_widgets='false',
                  bundle_assets_extract_enable='false',audio_driver='null')
    if a.notifications:
        values.update(video_font_enable='true',video_font_size='22')
    if any(any(c in str(v) for c in ('"','\n','\r')) for v in values.values()):raise ValueError('Unsupported configuration value')
    config=out/'retroarch.cfg';config.write_text(''.join(f'{k} = "{v}"\n' for k,v in values.items()))
    if a.nvram_sample:
        from libretro_nvram_capture import validate_save
        sample=a.nvram_sample.resolve(strict=True).read_bytes()
        validate_save(sample,a.rom.stem)
        (out/'saves'/(a.rom.stem+'.srm')).write_bytes(sample)
    shot=out/'frame.png'
    cmd=[str(a.retroarch.resolve()),'-v','-c',str(config),'-L',str(a.core.resolve()),str(a.rom.resolve()),'--max-frames',str(a.frames),'--max-frames-ss','--max-frames-ss-path',str(shot)]
    if a.replay:cmd.extend(['-P',str(a.replay.resolve())])
    if a.netplay_role:
        cmd[1:1]=['-H'] if a.netplay_role=='host' else ['-C',a.netplay_host]
    (out/'command.json').write_text(json.dumps(cmd,indent=2)+'\n');start=time.monotonic()
    with (out/'run.log').open('wb') as log:
        process=subprocess.Popen(cmd,cwd=out,stdout=log,stderr=subprocess.STDOUT,env=env)
        try:code=process.wait(timeout=a.timeout)
        except subprocess.TimeoutExpired:
            process.terminate()
            try:process.wait(timeout=5)
            except subprocess.TimeoutExpired:process.kill();process.wait()
            raise RuntimeError('Frontend timed out; inspect run.log')
    logs=(out/'run.log').read_text(errors='replace')
    report={'existing_save_fixture':bool(a.nvram_sample),'initial_nvram':a.initial_nvram,'nvram_settings':a.nvram_settings,'selected_nvram':selected_nvram,'linked_cabinets':a.linked_cabinets,'netplay_role':a.netplay_role,'netboard_messages':[line for line in logs.splitlines() if '[NetBoard]' in line],'steering_response':a.steering_response,'driving_ranges':ranges,'aspect':a.aspect,'widescreen':a.widescreen,'supersampling':a.supersampling,'overlay':a.overlay,'renderer':a.renderer,'driver':a.driver,'frames_requested':a.frames,'exit_code':code,'elapsed_seconds':round(time.monotonic()-start,3),'core_sha256':hashlib.sha256(a.core.read_bytes()).hexdigest(),'png':shot.exists() and shot.read_bytes().startswith(b'\x89PNG\r\n\x1a\n'),'renderer_messages':[line for line in logs.splitlines() if 'Renderer:' in line],'geometry_updates':logs.count('SET_GEOMETRY')}
    report.update(mvd_input=a.mvd_input,mvd_ranges=[a.mvd_horizontal_range,a.mvd_vertical_range],
                  av_timing=a.av_timing,
                  diagnostic_display=a.diagnostic_display,diagnostic_position=a.diagnostic_position,
                  diagnostic_opacity=a.diagnostic_opacity,
                  audio_donor=a.audio_donor,
                  audio_messages=[line for line in logs.splitlines() if '[TGPulse-Next Libretro] NetMerc Audio' in line],
                  notifications=a.notifications,
                  replay_sha256=hashlib.sha256(a.replay.read_bytes()).hexdigest() if a.replay else None,
                  replay_error='[Replay] Invalid' in logs or 'ran out of' in logs,
                  mvd_command_messages=[line for line in logs.splitlines() if '[TGPulse-Next Libretro] MVD' in line])
    (out/'result.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))
    if code or not report['png'] or report['replay_error']:raise RuntimeError('Frontend image handoff failed; inspect run.log')

if __name__=='__main__':main()
