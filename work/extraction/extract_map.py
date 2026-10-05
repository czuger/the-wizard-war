"""Cuts the map out of the board photo into img/board.jpg, at the photo's own resolution.

The map's black frame is traced along its four sides, then a Coons patch built on
those four curves maps the frame to a rectangle: this straightens both the
perspective and the poster's curvature. The frame line itself is shaved off.

Usage (from the repository root): python work/extraction/extract_map.py [output.jpg]
"""

import sys

import cv2
import numpy as np

from photo import REPO, load_photo

FRAME_SHAVE = 4  # pixels removed on each side, the half-width of the frame line plus its blur
JPEG_QUALITY = 90


def fit_line(points, iterations: int = 3):
    """Robust straight-line fit value = a * arg + b over (arg, value) points."""
    pts = np.array(points, float)
    for _ in range(iterations):
        a, b = np.polyfit(pts[:, 0], pts[:, 1], 1)
        residual = np.abs(pts[:, 1] - (a * pts[:, 0] + b))
        pts = pts[residual < max(2.0, np.median(residual) * 3)]
    return a, b


def rough_corners(gray: np.ndarray) -> np.ndarray:
    """Frame corners (TL, TR, BR, BL) from straight lines through the frame's dark pixels."""
    dark = gray < 70
    left, right, top, bottom = [], [], [], []
    for y in range(80, 2320, 10):
        xs = np.where(dark[y, 580:680])[0]
        if len(xs):
            left.append((y, 580 + xs.min()))
        xs = np.where(dark[y, 3300:3420])[0]
        if len(xs):
            right.append((y, 3300 + xs.max()))
    for x in range(620, 3350, 10):
        ys = np.where(dark[35:110, x])[0]
        if len(ys):
            top.append((x, 35 + ys.min()))
        ys = np.where(dark[2280:2380, x])[0]
        if len(ys):
            bottom.append((x, 2280 + ys.max()))
    vertical = [fit_line(left), fit_line(right)]  # x = a * y + b
    horizontal = [fit_line(top), fit_line(bottom)]  # y = c * x + d

    def meet(v, h):
        a, b = v
        c, d = h
        y = (c * b + d) / (1 - c * a)
        return a * y + b, y

    return np.array([
        meet(vertical[0], horizontal[0]),
        meet(vertical[1], horizontal[0]),
        meet(vertical[1], horizontal[1]),
        meet(vertical[0], horizontal[1]),
    ])


def median_filter(values: np.ndarray, size: int = 31) -> np.ndarray:
    pad = size // 2
    padded = np.pad(values, pad, mode="edge")
    return np.array([np.median(padded[i : i + size]) for i in range(len(values))])


def line_centre(window: np.ndarray):
    """Centre of the dark run closest to the middle of a 1-D profile, or None."""
    dark = window < 90
    idx = np.where(dark)[0]
    if len(idx) == 0:
        return None
    i = idx[np.argmin(np.abs(idx - len(window) // 2))]
    a = b = i
    while a > 0 and dark[a - 1]:
        a -= 1
    while b < len(window) - 1 and dark[b + 1]:
        b += 1
    return (a + b) / 2


def trace(gray: np.ndarray, p0, p1, vertical: bool):
    """Frame line between two rough corners: (arg, position) for each pixel along it."""
    if vertical:
        args = np.arange(int(p0[1]) - 5, int(p1[1]) + 6)
    else:
        args = np.arange(int(p0[0]) - 5, int(p1[0]) + 6)
    found = []
    for t in args:
        if vertical:
            guess = p0[0] + (p1[0] - p0[0]) * (t - p0[1]) / (p1[1] - p0[1])
            start = int(guess) - 15
            window = gray[max(t - 3, 0) : t + 4, start : start + 31].mean(0)
        else:
            guess = p0[1] + (p1[1] - p0[1]) * (t - p0[0]) / (p1[0] - p0[0])
            start = int(guess) - 15
            window = gray[start : start + 31, max(t - 3, 0) : t + 4].mean(1)
        centre = line_centre(window)
        found.append(np.nan if centre is None else start + centre)
    found = np.array(found, float)
    missing = np.isnan(found)
    found[missing] = np.interp(args[missing], args[~missing], found[~missing])
    return args, median_filter(found)


def smooth_curve(args, values):
    """Interpolating function of a traced side, with outliers replaced by a cubic fit and a moving average."""
    fit = np.polyval(np.polyfit(args, values, 3), args)
    values = np.where(np.abs(values - fit) > 3, fit, values)
    size = 41
    padded = np.pad(values, size // 2, mode="edge")
    values = np.convolve(padded, np.ones(size) / size, "valid")
    return lambda q: np.interp(q, args, values)


def straighten(img: np.ndarray) -> np.ndarray:
    gray = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY).astype(np.float32)
    tl, tr, br, bl = rough_corners(cv2.cvtColor(img, cv2.COLOR_BGR2GRAY))
    left = smooth_curve(*trace(gray, tl, bl, vertical=True))
    right = smooth_curve(*trace(gray, tr, br, vertical=True))
    top = smooth_curve(*trace(gray, tl, tr, vertical=False))
    bottom = smooth_curve(*trace(gray, bl, br, vertical=False))

    def corner(side_x, side_y, x, y):
        for _ in range(20):
            x = side_x(y)
            y = side_y(x)
        return np.array([x, y])

    p00 = corner(left, top, *tl)
    p10 = corner(right, top, *tr)
    p11 = corner(right, bottom, *br)
    p01 = corner(left, bottom, *bl)
    w = int(round(max(np.linalg.norm(p10 - p00), np.linalg.norm(p11 - p01))))
    h = int(round(max(np.linalg.norm(p01 - p00), np.linalg.norm(p11 - p10))))
    s = np.linspace(0, 1, w)[None, :]
    t = np.linspace(0, 1, h)[:, None]
    top_x = p00[0] + (p10[0] - p00[0]) * s
    bottom_x = p01[0] + (p11[0] - p01[0]) * s
    left_y = p00[1] + (p01[1] - p00[1]) * t
    right_y = p10[1] + (p11[1] - p10[1]) * t

    def coons(c_top, c_bottom, c_left, c_right, c00, c10, c01, c11):
        return (
            (1 - t) * c_top + t * c_bottom + (1 - s) * c_left + s * c_right
            - ((1 - s) * (1 - t) * c00 + s * (1 - t) * c10 + (1 - s) * t * c01 + s * t * c11)
        )

    map_x = coons(top_x, bottom_x, left(left_y), right(right_y), p00[0], p10[0], p01[0], p11[0])
    map_y = coons(top(top_x), bottom(bottom_x), left_y, right_y, p00[1], p10[1], p01[1], p11[1])
    out = cv2.remap(img, map_x.astype(np.float32), map_y.astype(np.float32), cv2.INTER_CUBIC,
                    borderMode=cv2.BORDER_REPLICATE)
    return out[FRAME_SHAVE:-FRAME_SHAVE, FRAME_SHAVE:-FRAME_SHAVE]


def main() -> None:
    output = sys.argv[1] if len(sys.argv) > 1 else str(REPO / "img" / "board.jpg")
    board = straighten(load_photo())
    cv2.imwrite(output, board, [cv2.IMWRITE_JPEG_QUALITY, JPEG_QUALITY])
    print(f"{output}: {board.shape[1]}x{board.shape[0]}")


if __name__ == "__main__":
    main()
