"""Generate transparent hexagonal swatch PNGs for Rust Python Functional Dark theme."""

from __future__ import annotations

import math
from pathlib import Path
from typing import Final

from PIL import Image, ImageDraw

# ── Swatch definitions ────────────────────────────────────────────────
# (name, hex_color, rgb, size_scale)
# size_scale: 1.0 = principal, 0.72 = secondary (Literal, Paratext)

SWATCHES: Final[dict[str, tuple[int, int, int]]] = {
    "accent":     (0x8F, 0xB6, 0xB8),
    "functional": (0xE7, 0xE8, 0xB9),
    "executive":  (0xCE, 0x91, 0x93),
    "common":     (0x7F, 0x9F, 0xA1),
    "editor_bg":  (0x13, 0x13, 0x15),
    "base_bg":    (0x0C, 0x0C, 0x10),
    "literal":    (0xA9, 0xCD, 0xD9),
    "paratext":   (0x49, 0x59, 0x5B),
    "link":       (0x9B, 0xC2, 0xC4),
    "warning":    (0xC8, 0xBF, 0xA8),
    "error":      (0xD0, 0x66, 0x7F),
    "info":       (0x87, 0xB2, 0xD1),
}

SCALE_MAP: Final[dict[str, float]] = {
    "accent":     0.618,
    "functional": 1.0,
    "executive":  1.0,
    "common":     1.0,
    "editor_bg":  1.0,
    "base_bg":    1.0,
    "literal":    0.72,
    "paratext":   0.72,
    "link":       1.0,
    "warning":    0.618,
    "error":      0.618,
    "info":       0.618,
}

# ── Constants ─────────────────────────────────────────────────────────

CANVAS_SIZE: Final[int] = 64
MSAA_FACTOR: Final[int] = 3
RENDER_SIZE: Final[int] = CANVAS_SIZE * MSAA_FACTOR

BASE_HEX_RADIUS: Final[float] = 24.0 * MSAA_FACTOR
BORDER_WIDTH: Final[float] = 1.25 * MSAA_FACTOR
RADIAL_ROTATION_OFFSET: Final[float] = math.radians(-8) * 0.6 / 2  # half of 60% from base

BG_COLOR: Final[tuple[int, int, int, int]] = (0x13, 0x13, 0x15, 255)
OUTPUT_DIR: Final[Path] = Path(__file__).resolve().parents[1] / "media"


def hexagon_points(
    cx: float, cy: float, radius: float, rot: float = 0.0
) -> list[tuple[float, float]]:
    """Return the six vertices of a regular hexagon."""
    return [
        (
            cx + radius * math.cos(math.pi / 3 * i + rot),
            cy + radius * math.sin(math.pi / 3 * i + rot),
        )
        for i in range(6)
    ]


def generate_swatch(
    name: str,
    color: tuple[int, int, int],
    scale: float,
) -> Image.Image:
    """Render a single hexagonal swatch on a transparent canvas."""
    img: Image.Image = Image.new("RGBA", (RENDER_SIZE, RENDER_SIZE), (0, 0, 0, 0))
    draw: ImageDraw.ImageDraw = ImageDraw.Draw(img)

    cx: float = RENDER_SIZE / 2.0
    cy: float = RENDER_SIZE / 2.0
    hex_r: float = BASE_HEX_RADIUS * scale

    # Border hexagon
    outer_pts: list[tuple[float, float]] = hexagon_points(cx, cy, hex_r, RADIAL_ROTATION_OFFSET)
    draw.polygon(outer_pts, fill=BG_COLOR)

    # Inner fill
    inner_r: float = hex_r - BORDER_WIDTH
    inner_pts: list[tuple[float, float]] = hexagon_points(cx, cy, inner_r, RADIAL_ROTATION_OFFSET)
    draw.polygon(inner_pts, fill=(color[0], color[1], color[2], 255))

    # Downsample with Lanczos for antialiasing
    final: Image.Image = img.resize((CANVAS_SIZE, CANVAS_SIZE), Image.LANCZOS)
    return final


def main() -> None:
    """Generate all swatch PNGs."""
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    for name, color in SWATCHES.items():
        scale: float = SCALE_MAP[name]
        swatch: Image.Image = generate_swatch(name, color, scale)
        out_path: Path = OUTPUT_DIR / f"swatch_{name}.png"
        swatch.save(out_path)
        print(f"  {out_path}  ({CANVAS_SIZE}x{CANVAS_SIZE}, scale={scale})")

    print(f"\nDone — {len(SWATCHES)} swatches in {OUTPUT_DIR}")


if __name__ == "__main__":
    main()
