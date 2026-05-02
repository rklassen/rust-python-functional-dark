"""Generate the Rust Python Functional Dark color wheel comparison chart.

Renders two HSL color wheels (Standard vs Deuteranomaly) with Fibonacci
spiral subdivisions, quantized block fills, and hexagonal swatch callouts
for the five principal theme colors.
"""

from __future__ import annotations

import math
import colorsys
from typing import Final

import numpy as np
from numpy.typing import NDArray
from PIL import Image, ImageDraw, ImageFont

# ── Theme constants ───────────────────────────────────────────────────

EDITOR_BG: Final[tuple[int, int, int, int]] = (0x13, 0x13, 0x15, 255)

SWATCHES: Final[dict[str, tuple[int, int, int]]] = {
    "functional": (0xE7, 0xE8, 0xB9),
    "executive":  (0xC8, 0x98, 0xB5),  # note: hue-rotated from theme original
    "common":     (0x7F, 0x9F, 0xA1),
    "literal":    (0xA9, 0xCD, 0xD9),
    "paratext":   (0x49, 0x59, 0x5B),
}

# ── Wheel parameters ──────────────────────────────────────────────────

CANVAS_W: Final[int] = 1024
CANVAS_H: Final[int] = 512
SCALE: Final[int] = 3  # supersample factor

RADIUS: Final[int] = 150
GAP: Final[int] = 200
RADIAL_RING_OFFSET: Final[float] = math.radians(-8)
GLOBAL_ROTATION: Final[float] = math.radians(-175)

RING_WIDTH: Final[float] = 1.5
AXIAL_WIDTH: Final[float] = 1.5
SATURATION: Final[float] = 0.4

FIBONACCI_RINGS: Final[list[tuple[int, int, int, int]]] = [
    # (inner_fifth, outer_fifth, n_divisions, offset_multiplier)
    (1, 2, 5, 1),
    (2, 3, 8, 2),
    (3, 4, 13, 3),
    (4, 5, 21, 4),
]

# ── Swatch layout ────────────────────────────────────────────────────

TILE_STEP: Final[float] = 2 * math.pi / 21  # outer ring subdivision

# Hue angles (radians) — some rotated from original theme values
STRUCT_HUE: Final[float] = (5.651 + 2 * TILE_STEP) % (2 * math.pi)
FUNC_HUE: Final[float] = (0.785 + TILE_STEP) % (2 * math.pi)
TEXT_HUE: Final[float] = 3.203
COMMENT_HUE: Final[float] = 3.258
NUMERIC_HUE: Final[float] = 3.403

# (name, baked_rgb, hue_rad, lightness, is_principal)
BAKED_COLORS: Final[list[tuple[str, tuple[int, int, int], float, float, bool]]] = [
    ("text",     SWATCHES["common"],     TEXT_HUE,    0.565, True),
    ("struct",   SWATCHES["executive"],   STRUCT_HUE,  0.690, True),
    ("function", SWATCHES["functional"],  FUNC_HUE,    0.820, True),
    ("comment",  SWATCHES["paratext"],    COMMENT_HUE, 0.322, False),
    ("numeric",  SWATCHES["literal"],     NUMERIC_HUE, 0.757, False),
]

SWATCH_DISPLAY_OFFSETS: Final[dict[str, float]] = {
    "comment": math.radians(-25) + TILE_STEP / 2,
    "numeric": math.radians(-12),
    "text":    TILE_STEP,
}

# ── Deuteranomaly simulation (Machado et al. 2009, severity ~1.0) ────

DEUTAN_MATRIX: Final[NDArray[np.float64]] = np.array([
    [0.367322, 0.860646, -0.227968],
    [0.280085, 0.672501,  0.047413],
    [-0.011820, 0.042940, 0.968881],
])


def apply_deuteranomaly(r: int, g: int, b: int) -> tuple[int, int, int]:
    """Apply deuteranomaly color vision simulation to an RGB triplet."""
    rgb: NDArray[np.float64] = np.array([r / 255.0, g / 255.0, b / 255.0])
    result: NDArray[np.float64] = np.clip(DEUTAN_MATRIX @ rgb, 0, 1)
    return (int(result[0] * 255), int(result[1] * 255), int(result[2] * 255))


# ── Color math ────────────────────────────────────────────────────────

def inv_smoothstep(t: float) -> float:
    """Attempt inverse of smoothstep — expands mid-lightness range."""
    if t < 0.5:
        return math.sqrt(t / 2.0)
    else:
        return 1.0 - math.sqrt((1.0 - t) / 2.0)


def inv_inv_smoothstep(lightness: float) -> float:
    """Forward smoothstep: given L = inv_smoothstep(t), solve for t."""
    if lightness < 0.5:
        return 2.0 * lightness * lightness
    else:
        return 1.0 - 2.0 * (1.0 - lightness) ** 2


def hsl_color(
    angle: float, dist: float, radius: float, deuteran: bool = False
) -> tuple[int, int, int, int]:
    """Sample HSL color at a given angle and distance from center."""
    t: float = dist / radius
    hue: float = (angle / (2 * math.pi)) % 1.0
    lightness: float = inv_smoothstep(t)
    rgb: tuple[float, float, float] = colorsys.hls_to_rgb(hue, lightness, SATURATION)
    rv, gv, bv = int(rgb[0] * 255), int(rgb[1] * 255), int(rgb[2] * 255)
    if deuteran:
        rv, gv, bv = apply_deuteranomaly(rv, gv, bv)
    return (rv, gv, bv, 255)


# ── Geometry helpers ──────────────────────────────────────────────────

def hexagon_points(
    cx: float, cy: float, radius: float, rotation: float = 0.0
) -> list[tuple[float, float]]:
    """Return the six vertices of a regular hexagon."""
    return [
        (
            cx + radius * math.cos(math.pi / 3 * i + rotation),
            cy + radius * math.sin(math.pi / 3 * i + rotation),
        )
        for i in range(6)
    ]


def nearest_corners(
    pts_a: list[tuple[float, float]], pts_b: list[tuple[float, float]]
) -> tuple[tuple[float, float], tuple[float, float]]:
    """Find the closest pair of vertices between two polygons."""
    best_d: float = float("inf")
    best_pa: tuple[float, float] = pts_a[0]
    best_pb: tuple[float, float] = pts_b[0]
    for pa in pts_a:
        for pb in pts_b:
            d: float = math.sqrt((pa[0] - pb[0]) ** 2 + (pa[1] - pb[1]) ** 2)
            if d < best_d:
                best_d = d
                best_pa = pa
                best_pb = pb
    return best_pa, best_pb


def get_sector(
    angle: float,
    boundaries: list[float],
    n_divs: int,
) -> int:
    """Determine which angular sector an angle falls in."""
    for i in range(n_divs):
        a1: float = boundaries[i]
        a2: float = boundaries[(i + 1) % n_divs]
        if a2 > a1:
            if a1 <= angle < a2:
                return i
        else:
            if angle >= a1 or angle < a2:
                return i
    return n_divs - 1


# ── Ring info builder ─────────────────────────────────────────────────

RingInfo = tuple[
    float,                          # inner_r
    float,                          # outer_r
    list[float],                    # boundaries
    list[tuple[int, int, int, int]],  # block_colors
    int,                            # n_divs
]


def build_ring_info(
    radius: float, deuteran: bool = False
) -> list[RingInfo]:
    """Build precomputed ring data with quantized block colors."""
    info: list[RingInfo] = []
    for inner_f, outer_f, n_divs, off_mult in FIBONACCI_RINGS:
        inner_r: float = radius * inner_f / 5.0
        outer_r: float = radius * outer_f / 5.0
        offset: float = RADIAL_RING_OFFSET * off_mult
        boundaries: list[float] = sorted(
            [
                (math.pi / 3 + GLOBAL_ROTATION + offset + 2 * math.pi * i / n_divs)
                % (2 * math.pi)
                for i in range(n_divs)
            ]
        )
        block_colors: list[tuple[int, int, int, int]] = []
        for i in range(n_divs):
            a1: float = boundaries[i]
            a2: float = boundaries[(i + 1) % n_divs]
            if a2 <= a1:
                mid: float = (a1 + (a2 + 2 * math.pi)) / 2.0
                if mid >= 2 * math.pi:
                    mid -= 2 * math.pi
            else:
                mid = (a1 + a2) / 2.0
            sample_angle: float = mid - GLOBAL_ROTATION
            block_colors.append(hsl_color(sample_angle, inner_r, radius, deuteran))
        info.append((inner_r, outer_r, boundaries, block_colors, n_divs))
    return info


# ── Circle renderer ───────────────────────────────────────────────────

def render_circle(
    pixels: Any,  # PixelAccess — no public type stub
    cx_pos: int,
    cy: int,
    radius: int,
    ring_info: list[RingInfo],
    ring_radii: list[float],
    ring_width: float,
    axial_width: float,
    bg: tuple[int, int, int, int],
) -> None:
    """Render a quantized HSL wheel into the pixel buffer."""
    r1: float = radius / 5.0
    for py in range(int(cy - radius - 1), int(cy + radius + 2)):
        for px in range(int(cx_pos - radius - 1), int(cx_pos + radius + 2)):
            dx: int = px - cx_pos
            dy: int = py - cy
            dist: float = math.sqrt(dx * dx + dy * dy)
            if dist > radius:
                continue
            if any(abs(dist - gr) < ring_width for gr in ring_radii):
                pixels[px, py] = bg
                continue
            if dist < r1:
                pixels[px, py] = (0, 0, 0, 255)
                continue
            angle: float = math.atan2(dy, dx)
            if angle < 0:
                angle += 2 * math.pi
            for inner_r, outer_r, boundaries, block_colors, n_divs in ring_info:
                if inner_r < dist < outer_r:
                    cut: bool = False
                    for ax_a in boundaries:
                        cx2: float = math.cos(ax_a)
                        sy2: float = math.sin(ax_a)
                        if dx * cx2 + dy * sy2 > 0 and abs(dx * sy2 - dy * cx2) < axial_width:
                            cut = True
                            break
                    if cut:
                        pixels[px, py] = bg
                    else:
                        pixels[px, py] = block_colors[get_sector(angle, boundaries, n_divs)]
                    break


# ── Anchor placement ──────────────────────────────────────────────────

def find_ring_and_tile_center(
    hue_rad: float,
    lightness: float,
    radius: float,
    ring_info: list[RingInfo],
) -> tuple[float, float]:
    """Find the (mid_r, mid_angle) center of the tile matching hue + lightness."""
    t: float = inv_inv_smoothstep(lightness)
    dist: float = t * radius
    ring_bounds: list[tuple[float, float]] = [(0, radius / 5.0)] + [
        (radius * i / 5.0, radius * (i + 1) / 5.0) for i in range(1, 5)
    ]
    ring_idx: int = 0
    for i, (ir, orr) in enumerate(ring_bounds):
        if ir <= dist <= orr:
            ring_idx = i
            break
    else:
        ring_idx = 4
    if ring_idx == 0:
        return 0.0, 0.0
    ri: int = ring_idx - 1
    if ri >= len(ring_info):
        ri = len(ring_info) - 1
    inner_r, outer_r, bounds, _bc, n_divs = ring_info[ri]
    mid_r: float = (inner_r + outer_r) / 2.0
    display_angle: float = (hue_rad + GLOBAL_ROTATION) % (2 * math.pi)
    sector: int = get_sector(display_angle, bounds, n_divs)
    a1: float = bounds[sector]
    a2: float = bounds[(sector + 1) % n_divs]
    if a2 <= a1:
        mid_a: float = (a1 + (a2 + 2 * math.pi)) / 2.0
        if mid_a >= 2 * math.pi:
            mid_a -= 2 * math.pi
    else:
        mid_a = (a1 + a2) / 2.0
    return mid_r, mid_a


# ── Main ──────────────────────────────────────────────────────────────

def main() -> None:
    """Generate the full color wheel comparison chart."""
    from typing import Any  # noqa: F811 — re-import for PixelAccess

    W: int = CANVAS_W * SCALE
    H: int = CANVAS_H * SCALE
    img: Image.Image = Image.new("RGBA", (W, H), EDITOR_BG)
    pixels: Any = img.load()

    STROKE: tuple[int, int, int, int] = (180, 180, 180, 255)
    cy: int = H // 2 + 20 * SCALE
    r: int = RADIUS * SCALE
    gap: int = GAP * SCALE
    lx: int = W // 2 - gap // 2 - r
    rx: int = W // 2 + gap // 2 + r

    ring_radii: list[float] = [r * i / 5.0 for i in range(1, 5)]
    rw: float = RING_WIDTH * SCALE
    aw: float = AXIAL_WIDTH * SCALE

    # Build ring data
    left_info: list[RingInfo] = build_ring_info(r, deuteran=False)
    right_info: list[RingInfo] = build_ring_info(r, deuteran=True)

    # Render circles
    render_circle(pixels, lx, cy, r, left_info, ring_radii, rw, aw, EDITOR_BG)
    render_circle(pixels, rx, cy, r, right_info, ring_radii, rw, aw, EDITOR_BG)

    draw: ImageDraw.ImageDraw = ImageDraw.Draw(img)

    # ── Swatch hexagons ───────────────────────────────────────────────
    base_hex_r: int = int(14 * 1.2 * 1.25 * 1.5) * SCALE
    big_hex_r: int = int(base_hex_r * 1.33)
    small_hex_r: int = int(base_hex_r * 0.67)
    anchor_hex_r: int = 8 * SCALE
    line_width: int = max(2, round(1 * 1.62)) * SCALE
    border_width: float = line_width * 1.618
    big_hex_dist: int = r + big_hex_r + 4 * SCALE
    small_hex_dist: int = r + small_hex_r + 4 * SCALE

    line_color: tuple[int, int, int, int] = (0x13, 0x13, 0x15, 200)
    border_color: tuple[int, int, int, int] = (0x13, 0x13, 0x15, 255)

    circle_configs: list[tuple[int, list[RingInfo], bool]] = [
        (lx, left_info, False),
        (rx, right_info, True),
    ]

    for cx_pos, ring_info, deuteran in circle_configs:
        for name, true_rgb, hue_rad, lightness, is_principal in BAKED_COLORS:
            hex_r: int = big_hex_r if is_principal else small_hex_r
            dist_out: int = big_hex_dist if is_principal else small_hex_dist
            display_hue: float = (hue_rad + GLOBAL_ROTATION) % (2 * math.pi)
            display_hue += SWATCH_DISPLAY_OFFSETS.get(name, 0.0)

            hx: float = cx_pos + dist_out * math.cos(display_hue)
            hy: float = cy + dist_out * math.sin(display_hue)
            rot: float = display_hue + math.pi + math.pi / 6

            outer_pts: list[tuple[float, float]] = hexagon_points(hx, hy, hex_r, rot)
            draw.polygon(outer_pts, fill=border_color)
            inner_r: float = hex_r - border_width
            if inner_r > 0:
                inner_pts: list[tuple[float, float]] = hexagon_points(hx, hy, inner_r, rot)
                tc: tuple[int, int, int] = true_rgb
                if deuteran:
                    tc = apply_deuteranomaly(*tc)
                draw.polygon(inner_pts, fill=(tc[0], tc[1], tc[2], 255))

            # Anchor at tile center
            mid_r, mid_a = find_ring_and_tile_center(hue_rad, lightness, r, ring_info)
            anch_x: float = cx_pos + mid_r * math.cos(mid_a)
            anch_y: float = cy + mid_r * math.sin(mid_a)
            anch_pts: list[tuple[float, float]] = hexagon_points(anch_x, anch_y, anchor_hex_r, rot)
            draw.polygon(anch_pts, fill=border_color)

            # Leader line
            pa, pb = nearest_corners(outer_pts, anch_pts)
            draw.line([pa, pb], fill=line_color, width=line_width)

    # ── Title ─────────────────────────────────────────────────────────
    font_bold: ImageFont.FreeTypeFont = ImageFont.truetype(
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf", 26 * SCALE
    )
    title: str = "Rust Python Functional Dark"
    bb: tuple[int, int, int, int] = draw.textbbox((0, 0), title, font=font_bold)
    tw: int = bb[2] - bb[0]
    func_color: tuple[int, int, int, int] = (
        SWATCHES["functional"][0],
        SWATCHES["functional"][1],
        SWATCHES["functional"][2],
        255,
    )
    draw.text(((W - tw) // 2, 10 * SCALE), title, fill=func_color, font=font_bold)

    # ── Labels ────────────────────────────────────────────────────────
    font_sm: ImageFont.FreeTypeFont = ImageFont.truetype(
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf", 18 * SCALE
    )
    for cx_pos_label, label in [(lx, "Standard"), (rx, "Deuteranomaly")]:
        bb = draw.textbbox((0, 0), label, font=font_sm)
        draw.text(
            (cx_pos_label - (bb[2] - bb[0]) // 2, cy + r + 12 * SCALE),
            label,
            fill=STROKE,
            font=font_sm,
        )

    # ── Downsample + vertical center ──────────────────────────────────
    final: Image.Image = img.resize((CANVAS_W, CANVAS_H), Image.LANCZOS)

    # Shift down 20px to center vertically
    arr: NDArray[np.uint8] = np.array(final)
    new_arr: NDArray[np.uint8] = np.full_like(arr, [0x13, 0x13, 0x15, 0xFF])
    shift: int = 20
    new_arr[shift:, :] = arr[:-shift, :]
    centered: Image.Image = Image.fromarray(new_arr)

    centered.save("rust_python_functional_dark_colorwheel.png")
    print(f"Saved rust_python_functional_dark_colorwheel.png ({CANVAS_W}x{CANVAS_H})")


if __name__ == "__main__":
    # Need Any for PixelAccess which has no public type stub
    from typing import Any  # noqa: F811
    main()
