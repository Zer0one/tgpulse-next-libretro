#!/usr/bin/env python3
"""Run an isolated Model 1 gameplay replay through RetroArch.

Adapts SM2-Emu's scripts/smoke-retroarch.py replay-v1 fixture. Synthetic
RetroPad input verifies the frontend path, not a physical controller.
"""

import argparse
import array
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import time
import wave


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inspect_recording(path: Path) -> dict:
    data = path.read_bytes()
    if not (data[:8] == b"RIFF\0\0\0\0" and data[12:24] == b"WAVE\0fmt \0\0\0"):
        raise ValueError("Unexpected RetroArch recording header")
    tag, channels, rate, byte_rate, block, bits = struct.unpack_from("<HHIIHH", data, 28)
    size = struct.unpack_from("<I", data, 52)[0]
    if not (tag == 1 and channels == 2 and bits == 16 and block == 4
            and byte_rate == rate * block and size == len(data) - 56):
        raise ValueError("Unexpected RetroArch PCM layout")
    pcm = data[56:]
    samples = array.array("h")
    samples.frombytes(pcm)
    normalized = path.with_name(path.stem + "-normalized.wav")
    with wave.open(str(normalized), "wb") as output:
        output.setparams((channels, bits // 8, rate, 0, "NONE", "not compressed"))
        output.writeframes(pcm)
    return {
        "audio_rate": rate,
        "audio_frames": len(pcm) // block,
        "audio_peak": max(map(abs, samples)),
        "audio_nonzero_samples": sum(sample != 0 for sample in samples),
        "audio_pcm_sha256": hashlib.sha256(pcm).hexdigest(),
        "normalized_recording": str(normalized),
    }


def replay(path: Path, frames: int) -> None:
    movie = bytearray(struct.pack("<6I", 0x42535632, 1, 0, 0, 1, 0))
    movie += bytes(16)  # RetroArch 1.22 reads 40 header bytes for v1.
    for frame in range(frames + 100):
        pressed = set()
        if any(start <= frame < start + 12 for start in (2200, 2400, 2600, 2800)):
            pressed.add(2)  # Select / Coin 1.
        if any(start <= frame < start + 12 for start in (2450, 2650, 2850, 3050)):
            pressed.add(3)  # Start.
        if frame >= 3150:
            if frame % 480 < 20:
                pressed.add(5)  # VR1 (Red): D-pad Down, matching SM2.
            if frame % 600 < 16:
                pressed.add(11)  # Gear Up.
        axes = {(0, 0): 0, (0, 1): 0, (1, 0): 0, (1, 1): 0}
        if frame >= 3150:
            axes[(0, 0)] = -12000 if (frame // 150) % 2 == 0 else 12000
        movie += struct.pack("<BH", 0, 96)
        for port in range(2):
            for button in range(16):
                movie += struct.pack(
                    "<4BHh", port, 1, 0, 0, button,
                    int(port == 0 and button in pressed),
                )
            for index in range(2):
                for axis in range(2):
                    movie += struct.pack("<4BHh", port, 5, index, 0, axis, axes[(index, axis)] if port == 0 else 0)
            for button in range(16):
                value = 32767 if port == 0 and button == 13 and frame >= 3150 else 0
                movie += struct.pack("<4BHh", port, 5, 2, 0, button, value)
            for mouse_id in range(4):
                movie += struct.pack("<4BHh", port, 2, 0, 0, mouse_id, 0)
            for lightgun_id in (2, 3, 6, 7, 13, 14, 15, 16):
                movie += struct.pack("<4BHh", port, 4, 0, 0, lightgun_id, 0)
        movie += b"f"
    path.write_bytes(movie)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("retroarch", "core", "system", "rom", "output"):
        parser.add_argument(f"--{name}", type=Path, required=True)
    parser.add_argument("--frames", type=int, default=3800)
    parser.add_argument("--timeout", type=int, default=300)
    parser.add_argument("--driver", choices=("gl", "glcore", "vulkan"), default="gl")
    parser.add_argument("--renderer", choices=("auto", "software", "opengl", "vulkan"), default="auto")
    parser.add_argument("--audio-driver", default="coreaudio")
    parser.add_argument("--input-driver", default="cocoa")
    parser.add_argument("--av-timing", choices=("native", "60hz"), default="native")
    args = parser.parse_args()
    if args.frames < 3200:
        parser.error("--frames must be at least 3200 for this VR replay")
    retroarch = args.retroarch.resolve(strict=True)
    core = args.core.resolve(strict=True)
    system = args.system.resolve(strict=True)
    rom = args.rom.resolve(strict=True)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    for name in ("saves", "states", "screenshots", "playlists", "config"):
        (output / name).mkdir()
    input_movie = output / "vr-inputs.replay"
    replay(input_movie, args.frames)
    options = output / "core-options.cfg"
    options.write_text('tgpulse_next_smooth_shadows = "enabled"\n'
                       'tgpulse_next_volume = "100"\n'
                       f'tgpulse_next_renderer = "{args.renderer}"\n'
                       f'tgpulse_next_av_timing = "{args.av_timing}"\n')
    values = {
        "system_directory": system, "savefile_directory": output / "saves",
        "savestate_directory": output / "states", "screenshot_directory": output / "screenshots",
        "playlist_directory": output / "playlists", "rgui_config_directory": output / "config",
        "core_options_path": options, "video_driver": args.driver, "audio_driver": args.audio_driver,
        "input_driver": args.input_driver, "video_fullscreen": "false",
        "video_windowed_fullscreen": "false", "video_scale": "2",
        "video_vsync": "false", "audio_sync": "true", "audio_enable": "true",
        "config_save_on_exit": "false", "content_history_enable": "false",
        "pause_nonactive": "false", "savestate_auto_save": "false",
        "savestate_auto_load": "false", "video_shader_enable": "false",
        "video_threaded": "false", "auto_overrides_enable": "false",
        "auto_remaps_enable": "false", "record_driver": "wav",
    }
    for key in ("content_history_path", "content_favorites_path", "content_image_history_path",
                "content_music_history_path", "content_video_history_path"):
        values[key] = output / f"{key}.lpl"
    config = output / "retroarch.cfg"
    for value in values.values():
        if any(char in str(value) for char in ('"', '\n', '\r')):
            raise ValueError("Unsupported RetroArch configuration value")
    config.write_text("".join(f'{key} = "{value}"\n' for key, value in values.items()))
    screenshot = output / "vr-gameplay.png"
    recording = output / "vr.wav"
    command = [str(retroarch), "-v", "-c", str(config), "-L", str(core),
               "-P", str(input_movie), "--max-frames", str(args.frames),
               "--max-frames-ss", "--max-frames-ss-path", str(screenshot),
               "-r", str(recording), str(rom)]
    (output / "command.json").write_text(json.dumps(command, indent=2) + "\n")
    started = time.monotonic()
    with (output / "run.log").open("wb") as log:
        completed = subprocess.run(command, cwd=output, stdout=log,
                                   stderr=subprocess.STDOUT, timeout=args.timeout)
    log_text = (output / "run.log").read_text(errors="replace")
    result = {
        "set_name": "vr", "frames_requested": args.frames,
        "renderer": args.renderer, "driver": args.driver,
        "audio_driver": args.audio_driver, "input_driver": args.input_driver,
        "av_timing": args.av_timing,
        "elapsed_seconds": round(time.monotonic() - started, 3),
        "exit_code": completed.returncode,
        "core_sha256": digest(core), "rom_sha256": digest(rom),
        "replay_sha256": digest(input_movie),
        "screenshot_sha256": digest(screenshot) if screenshot.exists() else None,
        "screenshot_png": screenshot.exists() and screenshot.read_bytes().startswith(b"\x89PNG\r\n\x1a\n"),
        "recording_size": recording.stat().st_size if recording.exists() else 0,
        "replay_error": "[Replay] Invalid" in log_text or "ran out of" in log_text,
        "audio_driver_error": "Failed to initialize audio driver" in log_text,
    }
    if recording.exists():
        result.update(inspect_recording(recording))
    (output / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))
    if (completed.returncode or not result["screenshot_png"] or result["replay_error"]
            or result["audio_driver_error"] or result.get("audio_peak", 0) == 0
            or result.get("audio_frames", 0) < result.get("audio_rate", 1) * 20):
        raise RuntimeError("RetroArch gameplay replay failed; inspect run.log")


if __name__ == "__main__":
    main()
