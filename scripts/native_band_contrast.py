#!/usr/bin/env python3
"""Pixel evidence for the native glass bands of the Linux client.

The script reads the native QA report and the captured PNG frames of one
evidence directory and measures the contrast of the really rendered pixels:

  * the band rectangle comes from the report (CSS pixels) and is multiplied by
    the device pixel ratio that the same report records;
  * the sample points are the points the driver recorded inside the band and
    outside every control, glyph and caption of that band, so a sampled pixel
    is the rendered background of the band and never a glyph;
  * the text colours are the computed colours the driver recorded for the real
    text of that band;
  * the contrast is the WCAG 2.x relative-luminance ratio between the sampled
    pixel and that text colour.

This is a pixel measurement of the rendered frame. The driver and
scripts/verify_native_client.py report a second, clearly labelled method: a
computed composite over the live intersecting backgrounds. Neither method
replaces the other and both are reported with their own name.

Pillow is used when it is importable (PIL 12.3.0 in this environment). When it
is missing the script falls back to a small pure-stdlib PNG reader (zlib plus
struct), so the evidence never depends on an optional package.

Usage:
    python3 scripts/native_band_contrast.py --evidence <dir> --report report.json
"""

from __future__ import annotations

import argparse
import json
import struct
import sys
import zlib
from pathlib import Path

TEXT_CONTRAST_MIN = 4.5
GLYPH_CONTRAST_MIN = 3.0

try:  # pragma: no cover - the fallback is exercised only without Pillow
    from PIL import Image

    PIL_AVAILABLE = True
except ImportError:  # pragma: no cover
    Image = None
    PIL_AVAILABLE = False


# --------------------------------------------------------------------------
# PNG reading
# --------------------------------------------------------------------------
def read_png(path: Path) -> tuple[int, int, list[tuple[int, int, int]]]:
    """Return (width, height, pixels) of a non-interlaced 8-bit PNG."""
    data = path.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"{path} is not a PNG")
    offset = 8
    width = height = 0
    bit_depth = color_type = 0
    idat = bytearray()
    while offset < len(data):
        (length,) = struct.unpack(">I", data[offset:offset + 4])
        kind = data[offset + 4:offset + 8]
        payload = data[offset + 8:offset + 8 + length]
        offset += 12 + length
        if kind == b"IHDR":
            width, height, bit_depth, color_type, _compression, _filter, interlace = struct.unpack(">IIBBBBB", payload)
            if bit_depth != 8 or interlace != 0:
                raise ValueError(f"{path} uses an unsupported PNG layout (depth {bit_depth}, interlace {interlace})")
        elif kind == b"IDAT":
            idat += payload
        elif kind == b"IEND":
            break
    channels = {0: 1, 2: 3, 4: 2, 6: 4}.get(color_type)
    if channels is None:
        raise ValueError(f"{path} uses an unsupported PNG colour type {color_type}")
    raw = zlib.decompress(bytes(idat))
    stride = width * channels
    pixels: list[tuple[int, int, int]] = []
    previous = bytearray(stride)
    position = 0
    for _ in range(height):
        filter_type = raw[position]
        position += 1
        line = bytearray(raw[position:position + stride])
        position += stride
        if filter_type == 1:
            for index in range(channels, stride):
                line[index] = (line[index] + line[index - channels]) & 0xFF
        elif filter_type == 2:
            for index in range(stride):
                line[index] = (line[index] + previous[index]) & 0xFF
        elif filter_type == 3:
            for index in range(stride):
                left = line[index - channels] if index >= channels else 0
                line[index] = (line[index] + ((left + previous[index]) >> 1)) & 0xFF
        elif filter_type == 4:
            for index in range(stride):
                left = line[index - channels] if index >= channels else 0
                up = previous[index]
                up_left = previous[index - channels] if index >= channels else 0
                estimate = left + up - up_left
                distances = (abs(estimate - left), abs(estimate - up), abs(estimate - up_left))
                if distances[0] <= distances[1] and distances[0] <= distances[2]:
                    predictor = left
                elif distances[1] <= distances[2]:
                    predictor = up
                else:
                    predictor = up_left
                line[index] = (line[index] + predictor) & 0xFF
        elif filter_type != 0:
            raise ValueError(f"{path} uses an unsupported PNG filter {filter_type}")
        for x in range(width):
            base = x * channels
            if channels >= 3:
                pixels.append((line[base], line[base + 1], line[base + 2]))
            else:
                pixels.append((line[base], line[base], line[base]))
        previous = line
    return width, height, pixels


def load_frame(path: Path) -> tuple[int, int, list[tuple[int, int, int]]]:
    if PIL_AVAILABLE:
        with Image.open(path) as image:
            rgb = image.convert("RGB")
            raw = rgb.tobytes()
            pixels = [(raw[index], raw[index + 1], raw[index + 2]) for index in range(0, len(raw), 3)]
            return rgb.width, rgb.height, pixels
    return read_png(path)


# --------------------------------------------------------------------------
# Colour maths (WCAG 2.x)
# --------------------------------------------------------------------------
def parse_color(value: str) -> tuple[float, float, float] | None:
    text = str(value or "").strip()
    if text.startswith("#"):
        digits = text[1:]
        if len(digits) in (3, 4):
            digits = "".join(digit * 2 for digit in digits)
        if len(digits) not in (6, 8):
            return None
        return tuple(int(digits[index:index + 2], 16) for index in (0, 2, 4))  # type: ignore[return-value]
    if text.lower().startswith("rgb"):
        inner = text[text.index("(") + 1:text.rindex(")")]
        parts = [part for part in inner.replace("/", ",").replace(" ", ",").split(",") if part]
        if len(parts) < 3:
            return None
        try:
            return tuple(float(part) for part in parts[:3])  # type: ignore[return-value]
        except ValueError:
            return None
    return None


def relative_luminance(color: tuple[float, float, float]) -> float:
    def channel(value: float) -> float:
        ratio = value / 255.0
        return ratio / 12.92 if ratio <= 0.03928 else ((ratio + 0.055) / 1.055) ** 2.4

    return 0.2126 * channel(color[0]) + 0.7152 * channel(color[1]) + 0.0722 * channel(color[2])


def contrast_ratio(first: tuple[float, float, float], second: tuple[float, float, float]) -> float:
    left = relative_luminance(first)
    right = relative_luminance(second)
    return (max(left, right) + 0.05) / (min(left, right) + 0.05)


# --------------------------------------------------------------------------
# Report reading
# --------------------------------------------------------------------------
def frame_bands(screenshot: dict) -> dict:
    layout = screenshot.get("layout") or {}
    return layout.get("bands") or {}


def measure_band(frame: dict, band_name: str, band: dict, layout: dict,
                 image: tuple[int, int, list[tuple[int, int, int]]]) -> dict:
    width, height, pixels = image
    viewport = layout.get("viewport") or {}
    ratio = float(viewport.get("devicePixelRatio") or 1.0)
    samples = band.get("samples") or []
    text_colors = [parse_color(value) for value in (band.get("textColors") or [])]
    text_colors = [color for color in text_colors if color]
    if not text_colors:
        fallback = parse_color(band.get("color") or "")
        if fallback:
            text_colors = [fallback]
    worst = None
    sampled = 0
    for point in samples:
        x = int(round(float(point.get("x", 0)) * ratio))
        y = int(round(float(point.get("y", 0)) * ratio))
        if x < 0 or y < 0 or x >= width or y >= height:
            continue
        pixel = pixels[y * width + x]
        sampled += 1
        for text in text_colors:
            ratio_value = contrast_ratio(text, pixel)
            if worst is None or ratio_value < worst["contrast"]:
                worst = {"contrast": round(ratio_value, 3), "point": [x, y], "point_css": [point.get("x"), point.get("y")],
                         "pixel": list(pixel), "text": band.get("textColors") or [band.get("color")]}
    blur = band.get("backdropFilter") or "none"
    glass = bool(blur) and blur != "none"
    verdict = "NOT_MEASURED"
    if worst is not None:
        if not glass:
            verdict = "FALLBACK_OPAQUE"
        else:
            verdict = "PASS" if worst["contrast"] >= TEXT_CONTRAST_MIN else "FAIL"
    return {
        "band": band_name,
        "glass": glass,
        "backdrop_filter": blur,
        "rect_css": {key: band.get(key) for key in ("x", "y", "width", "height")},
        "rect_px": {"x": int(round(float(band.get("x", 0)) * ratio)), "y": int(round(float(band.get("y", 0)) * ratio)),
                    "width": int(round(float(band.get("width", 0)) * ratio)), "height": int(round(float(band.get("height", 0)) * ratio))},
        "device_pixel_ratio": ratio,
        "image": {"width": width, "height": height},
        "points": len(samples),
        "sampled": sampled,
        "text_colors": band.get("textColors") or [],
        "worst": worst,
        "threshold": TEXT_CONTRAST_MIN if glass else GLYPH_CONTRAST_MIN,
        "verdict": verdict,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Measure the rendered contrast of the native glass bands.")
    parser.add_argument("--evidence", type=Path, required=True, help="The evidence directory that holds report.json and the PNG frames.")
    parser.add_argument("--report", default="report.json", help="The report file name inside the evidence directory.")
    arguments = parser.parse_args()

    evidence = arguments.evidence.resolve()
    report_path = evidence / arguments.report
    if not report_path.is_file():
        raise SystemExit(f"No native QA report at {report_path}")
    report = json.loads(report_path.read_text())
    frames = []
    failures = []
    glass_frames = 0
    skipped = []
    for screenshot in report.get("screenshots") or []:
        name = screenshot.get("name")
        if not name or not screenshot.get("passed", True):
            continue
        bands = frame_bands(screenshot)
        if not bands:
            # A frame that carries no band layout cannot be measured. It is
            # recorded, never silently dropped: a run whose frames all lost
            # their layout must not read as a passing contrast gate.
            skipped.append(f"{name}: no band layout")
            continue
        layout = screenshot.get("layout") or {}
        image_path = evidence / f"native-{name}.png"
        if not image_path.is_file():
            failures.append(f"{name}: {image_path.name} is missing")
            continue
        try:
            image = load_frame(image_path)
        except (OSError, ValueError) as error:
            failures.append(f"{name}: {error}")
            continue
        for band_name in ("player", "topbar"):
            band = bands.get(band_name)
            if not band:
                continue
            measurement = measure_band(name, band_name, band, layout, image)
            measurement["frame"] = name
            measurement["theme"] = bands.get("theme")
            measurement["app_scale"] = bands.get("appScale")
            measurement["viewport"] = layout.get("viewport")
            frames.append(measurement)
            if measurement["glass"]:
                glass_frames += 1
            if measurement["verdict"] == "FAIL":
                failures.append(f"{name}/{band_name}: {measurement['worst']['contrast']}:1 below {TEXT_CONTRAST_MIN}:1")
    result = {
        "method": "rendered pixel sample inside the recorded band rectangle, outside every control and glyph",
        "reader": "Pillow" if PIL_AVAILABLE else "stdlib zlib+struct PNG reader",
        "text_threshold": TEXT_CONTRAST_MIN,
        "frames": frames,
        "glass_frames": glass_frames,
        "skipped": skipped,
        "verdict": "PASS" if frames and glass_frames and not failures else ("FAIL" if failures else "NOT_MEASURED"),
        "failures": failures,
    }
    print(json.dumps(result, indent=2))
    return 0 if result["verdict"] == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
