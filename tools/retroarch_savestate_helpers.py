"""Small macOS helpers for the RetroArch Save State skill runner."""

import ctypes
from pathlib import Path
import socket
import subprocess
import time


def free_udp_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def wait_while_alive(process: subprocess.Popen[bytes], seconds: float) -> None:
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError(f"RetroArch exited early with code {process.returncode}")
        time.sleep(min(0.10, deadline - time.monotonic()))


def take_screenshot(
    process: subprocess.Popen[bytes], work: Path, command_port: int, label: str,
) -> Path:
    directory = work / "screenshots"
    before = set(directory.glob("*.png"))
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as sock:
        sock.sendto(b"SCREENSHOT\n", ("127.0.0.1", command_port))
    deadline = time.monotonic() + 5.0
    while time.monotonic() < deadline:
        created = sorted(
            set(directory.glob("*.png")) - before,
            key=lambda path: path.stat().st_mtime_ns,
        )
        if created:
            source = created[-1]
            destination = directory / f"{label}.png"
            if source != destination:
                source.replace(destination)
            return destination
        if process.poll() is not None:
            raise RuntimeError(f"RetroArch exited before screenshot {label}")
        time.sleep(0.10)
    raise RuntimeError(f"RetroArch did not create screenshot {label}")


def post_escape_to_pid(pid: int) -> None:
    app_services = ctypes.CDLL(
        "/System/Library/Frameworks/ApplicationServices.framework/ApplicationServices"
    )
    core_foundation = ctypes.CDLL(
        "/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation"
    )
    app_services.CGEventCreateKeyboardEvent.restype = ctypes.c_void_p
    app_services.CGEventCreateKeyboardEvent.argtypes = [
        ctypes.c_void_p, ctypes.c_ushort, ctypes.c_bool,
    ]
    app_services.CGEventPostToPid.argtypes = [ctypes.c_int, ctypes.c_void_p]
    core_foundation.CFRelease.argtypes = [ctypes.c_void_p]
    for pressed in (True, False):
        event = app_services.CGEventCreateKeyboardEvent(None, 53, pressed)
        if not event:
            raise RuntimeError("CoreGraphics could not create an Escape key event")
        app_services.CGEventPostToPid(pid, event)
        core_foundation.CFRelease(event)
        time.sleep(0.03)
