#!/usr/bin/env python3
"""Check isolated Game System captures against documentary screenshots."""
import argparse
import hashlib
import json
import pathlib
import struct
import tomllib
import zlib

from libretro_nvram_capture import validate_save


def game_system_crc(eeprom):
    # The 93C45 image stores words little-endian; the game's CRC traverses
    # each word high byte first, starting after the stored checksum word.
    crc = 0
    for index in range(0x0a, 0x80, 2):
        for byte in (eeprom[index + 1], eeprom[index]):
            crc ^= byte << 8
            for _ in range(8):
                crc = ((crc << 1) ^ (0x1021 if crc & 0x8000 else 0)) & 0xffff
    return crc


def pixels(path):
    data = path.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"Invalid PNG: {path}")
    position, compressed, size = 8, bytearray(), None
    while position < len(data):
        length = struct.unpack_from(">I", data, position)[0]
        kind = data[position + 4:position + 8]
        payload = data[position + 8:position + 8 + length]
        if zlib.crc32(kind + payload) != struct.unpack_from(">I", data, position + 8 + length)[0]:
            raise ValueError(f"PNG chunk checksum mismatch: {path}")
        if kind == b"IHDR":
            width, height, depth, color, compression, filtering, interlace = struct.unpack(">IIBBBBB", payload)
            if (depth, color, compression, filtering, interlace) != (8, 2, 0, 0, 0):
                raise ValueError(f"Expected 8-bit non-interlaced RGB PNG: {path}")
            size = (width, height)
        elif kind == b"IDAT":
            compressed.extend(payload)
        elif kind == b"IEND":
            break
        position += 12 + length
    if size is None:
        raise ValueError(f"PNG has no header: {path}")
    width, height = size
    raw = zlib.decompress(compressed)
    stride = width * 3
    if len(raw) != height * (stride + 1):
        raise ValueError(f"PNG pixel size mismatch: {path}")
    previous = bytearray(stride)
    image = bytearray()
    for row_index in range(height):
        start = row_index * (stride + 1)
        mode = raw[start]
        source = raw[start + 1:start + stride + 1]
        current = bytearray(stride)
        for index, value in enumerate(source):
            left = current[index - 3] if index >= 3 else 0
            up = previous[index]
            upper_left = previous[index - 3] if index >= 3 else 0
            if mode == 0:
                predictor = 0
            elif mode == 1:
                predictor = left
            elif mode == 2:
                predictor = up
            elif mode == 3:
                predictor = (left + up) // 2
            elif mode == 4:
                estimate = left + up - upper_left
                distances = (abs(estimate - left), abs(estimate - up), abs(estimate - upper_left))
                predictor = (left, up, upper_left)[distances.index(min(distances))]
            else:
                raise ValueError(f"Unknown PNG filter {mode}: {path}")
            current[index] = (value + predictor) & 255
        image.extend(current)
        previous = current
    return size, image


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--set", choices=("vr", "vformula"), required=True)
    parser.add_argument("--recipes", type=pathlib.Path, required=True)
    parser.add_argument("--samples", type=pathlib.Path, required=True)
    parser.add_argument("--reloads", type=pathlib.Path)
    args = parser.parse_args()
    root = pathlib.Path(__file__).resolve().parents[1]
    recipes = sorted(args.recipes.glob("*--step-*.toml"))
    expected_count = {"vr": 63, "vformula": 59}[args.set]
    errors, acquired, reloaded = [], 0, 0
    if len(recipes) != expected_count:
        errors.append(f"Expected {expected_count} recipes, found {len(recipes)}")
    for path in recipes:
        recipe = tomllib.loads(path.read_text())
        field, step = recipe["field"], recipe["value_step"]
        sample = args.samples / path.stem
        suffix = f"{step:02d}" if args.set == "vr" else str(step)
        evidence = root / "docs/diagnostic-evidence" / args.set / f"{field.replace('_', '-')}-{suffix}.png"
        try:
            manifest = json.loads((sample / "manifest.json").read_text())
            if manifest["status"] != "complete" or manifest["recipe"] != recipe:
                raise ValueError("Capture manifest differs from recipe")
            saved = (sample / "saved.srm").read_bytes()
            validate_save(saved, args.set)
            eeprom = saved[64 + 65536:]
            if int.from_bytes(eeprom[8:10], "big") != game_system_crc(eeprom):
                raise ValueError("Native Game System EEPROM CRC mismatch")
            if pixels(sample / "selected.png") != pixels(evidence):
                raise ValueError("Selected value differs from catalogue image")
            acquired += 1
            if args.reloads:
                reload = args.reloads / path.stem
                check = json.loads((reload / "manifest.json").read_text())
                if check["status"] != "complete":
                    raise ValueError("Fresh-load capture incomplete")
                verify_recipe = tomllib.loads((args.recipes / f"{args.set}--verify--{field}.toml").read_text())
                if check["recipe"] != verify_recipe:
                    raise ValueError("Fresh-load recipe mismatch")
                with (sample / "saved.srm").open("rb") as stream:
                    saved_hash = hashlib.file_digest(stream, "sha256").hexdigest()
                if check.get("import_sha256") != saved_hash:
                    raise ValueError("Fresh load imported another Save RAM")
                if pixels(reload / "reloaded.png") != pixels(evidence):
                    raise ValueError("Fresh-load menu differs from catalogue image")
                reloaded += 1
        except (FileNotFoundError, KeyError, ValueError, zlib.error) as error:
            errors.append(f"{path.name}: {error}")
    print(json.dumps({"set": args.set, "recipes": len(recipes), "acquired": acquired,
                      "reloaded": reloaded if args.reloads else None,
                      "errors": errors}, indent=2))
    if errors:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
