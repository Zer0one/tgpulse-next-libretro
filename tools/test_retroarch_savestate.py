#!/usr/bin/env python3
"""Verify Model 1 Save States in one isolated macOS RetroArch process."""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import socket
import subprocess
import time

from retroarch_savestate_helpers import (
    free_udp_port,
    post_escape_to_pid,
    take_screenshot,
    wait_while_alive,
)


def quote(value: Path | str) -> str:
    return str(value).replace("\\", "\\\\").replace('"', '\\"')


def wait_for_state(directory: Path, process: subprocess.Popen[bytes]) -> Path:
    deadline = time.monotonic() + 10.0
    while time.monotonic() < deadline:
        states = sorted(directory.rglob("*.state*"), key=lambda path: path.stat().st_mtime_ns)
        if states and states[-1].stat().st_size:
            return states[-1]
        if process.poll() is not None:
            raise RuntimeError(f"RetroArch exited before writing the state ({process.returncode})")
        time.sleep(0.10)
    raise RuntimeError("RetroArch did not write a save state")


def send_command(command_port: int, command: str) -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as sock:
        sock.sendto(f"{command}\n".encode(), ("127.0.0.1", command_port))


def crop_game_region(source: Path, destination: Path, *, top: float | None = None,
                     height_fraction: float = 0.60) -> None:
    """Exclude RetroArch notifications while retaining a large game region."""
    properties = subprocess.run(
        ["/usr/bin/sips", "-g", "pixelWidth", "-g", "pixelHeight", str(source)],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.splitlines()
    values = {
        key.strip(): int(value.strip())
        for line in properties if ":" in line
        for key, value in [line.split(":", 1)]
        if key.strip() in {"pixelWidth", "pixelHeight"}
    }
    width, height = values["pixelWidth"], values["pixelHeight"]
    crop_width = max(1, round(width * 0.40))
    crop_height = max(1, round(height * height_fraction))
    offset_x = (width - crop_width) // 2
    offset_y = (height - crop_height) // 2 if top is None else round(height * top)
    subprocess.run(
        [
            "/usr/bin/sips", "--cropToHeightWidth", str(crop_height), str(crop_width),
            "--cropOffset", str(offset_y), str(offset_x), str(source),
            "--out", str(destination),
        ],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def quit_retroarch(process: subprocess.Popen[bytes], command_port: int) -> bool:
    if process.poll() is not None:
        return False
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as sock:
        sock.sendto(b"QUIT\n", ("127.0.0.1", command_port))
    try:
        process.wait(timeout=8.0)
        return False
    except subprocess.TimeoutExpired:
        try:
            post_escape_to_pid(process.pid)
            time.sleep(1.0)
            if process.poll() is None:
                post_escape_to_pid(process.pid)
            process.wait(timeout=5.0)
            return False
        except (OSError, RuntimeError, subprocess.TimeoutExpired):
            process.terminate()
        try:
            process.wait(timeout=5.0)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5.0)
        return True


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("retroarch", "core", "system", "rom", "output"):
        parser.add_argument(f"--{name}", type=Path, required=True)
    parser.add_argument("--set-name", required=True)
    parser.add_argument("--replay", type=Path)
    parser.add_argument("--boot-wait", type=float, default=38.0)
    parser.add_argument("--advance-wait", type=float, default=3.0)
    parser.add_argument("--reset-before-advance", action="store_true",
                        help="For static title screens, reset before each resumed interval")
    parser.add_argument("--crop-top", type=float,
                        help="Normalized top of the compared region; default is centered")
    parser.add_argument("--crop-height", type=float, default=0.60)
    args = parser.parse_args()
    if not 0 < args.crop_height <= 1 or (args.crop_top is not None and
            not 0 <= args.crop_top <= 1 - args.crop_height):
        parser.error("Compared region must fit inside the game image")

    retroarch = args.retroarch.resolve(strict=True)
    core = args.core.resolve(strict=True)
    system = args.system.resolve(strict=True)
    rom = args.rom.resolve(strict=True)
    replay = args.replay.resolve(strict=True) if args.replay else None
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    for name in ("saves", "states", "screenshots", "config", "playlists"):
        (output / name).mkdir()

    (output / "playlists" / "builtin").mkdir()
    command_port = free_udp_port()
    core_options = output / "core-options.cfg"
    core_options.write_text(
        'tgpulse_next_smooth_shadows = "enabled"\n'
        'tgpulse_next_volume = "100"\n',
        encoding="utf-8",
    )
    config_values = {
        "config_save_on_exit": "false",
        "playlist_directory": output / "playlists",
        "content_favorites_path": output / "playlists/builtin/content_favorites.lpl",
        "content_history_path": output / "playlists/builtin/content_history.lpl",
        "content_image_history_path": output / "playlists/builtin/content_image_history.lpl",
        "content_music_history_path": output / "playlists/builtin/content_music_history.lpl",
        "content_video_history_path": output / "playlists/builtin/content_video_history.lpl",
        "system_directory": system,
        "savefile_directory": output / "saves",
        "savestate_directory": output / "states",
        "screenshot_directory": output / "screenshots",
        "core_options_path": core_options,
        "global_core_options": "true",
        "network_cmd_enable": "true",
        "network_cmd_port": str(command_port),
        "video_driver": "gl",
        "audio_driver": "coreaudio",
        "input_driver": "cocoa",
        "video_fullscreen": "false",
        "video_windowed_fullscreen": "false",
        "video_scale": "2",
        "video_vsync": "false",
        "audio_sync": "true",
        "audio_enable": "true",
        "pause_nonactive": "false",
        "savestate_auto_save": "false",
        "savestate_auto_load": "false",
        "savestate_auto_index": "false",
        "savestate_file_compression": "false",
        "savestate_thumbnail_enable": "false",
        "state_slot": "0",
        "input_pause_toggle": "p",
        "input_save_state": "f2",
        "input_load_state": "f4",
        "video_shader_enable": "false",
        "menu_enable_widgets": "false",
        "video_font_enable": "false",
        "video_threaded": "false",
        "auto_overrides_enable": "false",
        "auto_remaps_enable": "false",
    }
    config = output / "retroarch.cfg"
    config.write_text(
        "".join(f'{key} = "{quote(value)}"\n' for key, value in config_values.items()),
        encoding="utf-8",
    )
    command = [str(retroarch), "-v", "-c", str(config), "-L", str(core)]
    if replay:
        local_replay = output / "input.replay"
        shutil.copy2(replay, local_replay)
        command.extend(["-P", str(local_replay)])
    command.append(str(rom))
    (output / "command.json").write_text(json.dumps(command, indent=2) + "\n")

    forced = False
    with (output / "run.log").open("wb") as log:
        process = subprocess.Popen(command, cwd=output, stdout=log, stderr=subprocess.STDOUT)
        try:
            wait_while_alive(process, args.boot_wait)
            send_command(command_port, "PAUSE_TOGGLE")
            wait_while_alive(process, 1.0)
            send_command(command_port, "SAVE_STATE")
            state = wait_for_state(output / "states", process)
            wait_while_alive(process, 2.5)
            saved = take_screenshot(process, output, command_port, "saved")
            send_command(command_port, "PAUSE_TOGGLE")
            if args.reset_before_advance:
                # This RetroArch build handles RESET/LOAD only while running.
                wait_while_alive(process, 0.2)
                send_command(command_port, "RESET")
                # Confirm this build's double-press Reset policy.
                wait_while_alive(process, 0.1)
                send_command(command_port, "RESET")
            wait_while_alive(process, args.advance_wait)
            send_command(command_port, "PAUSE_TOGGLE")
            wait_while_alive(process, 1.0)
            advanced = take_screenshot(process, output, command_port, "advanced")

            send_command(command_port, "PAUSE_TOGGLE")
            wait_while_alive(process, 0.2)
            send_command(command_port, "LOAD_STATE")
            wait_while_alive(process, 0.2)
            send_command(command_port, "PAUSE_TOGGLE")
            wait_while_alive(process, 0.2)
            send_command(command_port, "FRAMEADVANCE")
            wait_while_alive(process, 1.0)
            restored = take_screenshot(process, output, command_port, "restored")
            send_command(command_port, "PAUSE_TOGGLE")
            if args.reset_before_advance:
                # This RetroArch build handles RESET/LOAD only while running.
                wait_while_alive(process, 0.2)
                send_command(command_port, "RESET")
                # Confirm this build's double-press Reset policy.
                wait_while_alive(process, 0.1)
                send_command(command_port, "RESET")
            wait_while_alive(process, args.advance_wait)
            send_command(command_port, "PAUSE_TOGGLE")
            wait_while_alive(process, 1.0)
            advanced_again = take_screenshot(process, output, command_port, "advanced-again")
            send_command(command_port, "PAUSE_TOGGLE")
            wait_while_alive(process, 0.2)
            send_command(command_port, "LOAD_STATE")
            wait_while_alive(process, 0.2)
            send_command(command_port, "PAUSE_TOGGLE")
            wait_while_alive(process, 0.2)
            send_command(command_port, "FRAMEADVANCE")
            wait_while_alive(process, 1.0)
            restored_again = take_screenshot(process, output, command_port, "restored-again")
            send_command(command_port, "PAUSE_TOGGLE")
            wait_while_alive(process, 1.0)
        finally:
            forced = quit_retroarch(process, command_port)

    digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    saved_hash = digest(saved)
    advanced_hash = digest(advanced)
    restored_hash = digest(restored)
    saved_crop = output / "screenshots" / "saved-game-region.png"
    restored_crop = output / "screenshots" / "restored-game-region.png"
    restored_again_crop = output / "screenshots" / "restored-again-game-region.png"
    advanced_crop = output / "screenshots" / "advanced-game-region.png"
    advanced_again_crop = output / "screenshots" / "advanced-again-game-region.png"
    crop = {"top": args.crop_top, "height_fraction": args.crop_height}
    crop_game_region(saved, saved_crop, **crop)
    crop_game_region(restored, restored_crop, **crop)
    crop_game_region(restored_again, restored_again_crop, **crop)
    crop_game_region(advanced, advanced_crop, **crop)
    crop_game_region(advanced_again, advanced_again_crop, **crop)
    saved_game_hash = digest(saved_crop)
    restored_game_hash = digest(restored_crop)
    restored_again_game_hash = digest(restored_again_crop)
    advanced_game_hash = digest(advanced_crop)
    advanced_again_game_hash = digest(advanced_again_crop)
    log_text = (output / "run.log").read_text(encoding="utf-8", errors="replace")
    result = {
        "set_name": args.set_name,
        "advance_policy": "reset_then_run" if args.reset_before_advance else "run",
        "compared_region": {"top": args.crop_top, "height": args.crop_height, "width": 0.40},
        "state": str(state),
        "state_size": state.stat().st_size,
        "state_sha256": digest(state),
        "core_sha256": digest(core),
        "rom_sha256": digest(rom),
        "retroarch_version": next((line.strip() for line in log_text.splitlines() if "RetroArch " in line), "unknown"),
        "saved_screenshot": str(saved),
        "advanced_screenshot": str(advanced),
        "restored_screenshot": str(restored),
        "saved_sha256": saved_hash,
        "advanced_sha256": advanced_hash,
        "restored_sha256": restored_hash,
        "saved_game_region_sha256": saved_game_hash,
        "restored_game_region_sha256": restored_game_hash,
        "restored_again_game_region_sha256": restored_again_game_hash,
        "advanced_again_sha256": digest(advanced_again),
        "advanced_game_region_sha256": advanced_game_hash,
        "advanced_again_game_region_sha256": advanced_again_game_hash,
        "advanced_changed": advanced_game_hash != saved_game_hash,
        "restored_game_region_exactly": restored_game_hash == saved_game_hash,
        "two_restores_game_region_exactly": restored_game_hash == restored_again_game_hash,
        "second_advance_changed": advanced_again_game_hash != restored_game_hash,
        "save_log_message": (
            "Salvataggio dello stato" in log_text or "Saved state" in log_text
        ),
        "load_log_message": (
            "Caricamento dello stato" in log_text or "Loaded state" in log_text
        ),
        "exit_code": process.returncode,
        "history_isolated": str(output / "playlists") in log_text and "/Documents/RetroArch/playlists/" not in log_text,
        "forced_kill": forced,
    }
    (output / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    if not result["advanced_changed"]:
        raise RuntimeError("The game image did not advance after saving")
    if args.reset_before_advance and not result["restored_game_region_exactly"]:
        raise RuntimeError("Loaded state did not restore the saved title region")
    if not result["two_restores_game_region_exactly"]:
        raise RuntimeError("Two loads of the same state produced different game regions")
    if not result["second_advance_changed"]:
        raise RuntimeError("The game did not advance after the first restore")
    if not result["save_log_message"] or not result["load_log_message"]:
        raise RuntimeError("RetroArch did not log both Save State operations")
    if not result["history_isolated"]:
        raise RuntimeError("RetroArch used playlist history outside the test directory")
    if process.returncode != 0 or forced:
        raise RuntimeError("RetroArch did not close cleanly")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
