"""Cuts the pawns out of the counter sheet on the left of the board photo into img/token-*.png.

The sheet has two grids: 7 columns x 16 rows of small pawns (laboratories,
peddlers, caravans, rebels, cult markers) and 5 columns x 16 rows of magic items,
with a column of Xéphis fanatic strips on its right and a row of Yaboul fanatic
strips under it. Grid lines are found from edge-strength profiles, tracked one
line after the other at the printed pitch, then each wanted cell is straightened
at its own size. Fanatic strips are turned to lie flat (arrow pointing right).

The face-down cult marker and fanatic have no printed picture: they are made from
the Xéphis marker and the strength-1 Yaboul strip, emptied and given a brown "?".

Usage (from the repository root): python work/extraction/extract_tokens.py [output_dir]
"""

import sys
from pathlib import Path

import cv2
import numpy as np

from photo import REPO, bilinear, load_photo, warp_quad

# Rough corners (TL, TR, BR, BL) of the two grids in the photo.
SMALL_GRID = [(123, 70), (553, 69), (567, 1052), (138, 1054)]  # 7 x 16
ITEM_GRID = [(140, 1057), (536, 1057), (563, 2300), (166, 2300)]  # 5 x 16

# Cell (column, row) of the cleanest copy of each pawn.
SMALL_TOKENS = {
    "laboratory": (3, 1),
    "peddler-pink": (0, 4), "peddler-yellow": (1, 4), "peddler-green": (2, 4),
    "peddler-blue": (3, 4), "peddler-white": (4, 5), "peddler-black": (6, 4),
    "caravan-yellow": (1, 7), "caravan-pink": (1, 8), "caravan-green": (3, 7),
    "caravan-blue": (4, 7), "caravan-white": (5, 7), "caravan-black": (6, 7),
    "rebels": (3, 9),
    "cult-xephis": (3, 11), "cult-yaboul": (3, 14),
}
ITEM_TOKENS = {"talisman-toratsa": (2, 1), "grimoire-xephis": (2, 6), "pentacle-yaboul": (2, 11)}
# Strength -> rough top and bottom (photo y) of a Xéphis strip in the right column.
XEPHIS_STRIPS = {3: (1053, 1181), 2: (1435, 1561), 1: (2057, 2183)}
# Strength -> rough left and right (photo x) of a Yaboul strip in the bottom row.
YABOUL_STRIPS = {3: (210, 240), 2: (359, 388), 1: (507, 536)}
# Extra rows cut at the bottom of a token, where the grid line survives the trim.
EXTRA_BOTTOM_CUT = {"talisman-toratsa": 4}

BROWN = np.array([40, 60, 95], float)  # BGR ink of the invented face-down pawns


class Sheet:
    """The photo with the edge maps and the ink map used to find grid lines."""

    def __init__(self, img: np.ndarray):
        self.img = img
        f = img.astype(np.float32)
        edge_y = sum(np.abs(cv2.Sobel(f[..., k], cv2.CV_32F, 0, 1, ksize=3)) for k in range(3))
        edge_x = sum(np.abs(cv2.Sobel(f[..., k], cv2.CV_32F, 1, 0, ksize=3)) for k in range(3))
        self.edge_y = cv2.GaussianBlur(edge_y, (0, 0), 1.2)
        self.edge_x = cv2.GaussianBlur(edge_x, (0, 0), 1.2)
        # Darkest channel: red and blue ink show as dark as black ink.
        self.ink = cv2.GaussianBlur(img.min(axis=2), (3, 3), 0).astype(np.float32)

    def grid(self, quad, cols: int, rows: int) -> np.ndarray:
        """Corners of every cell, as an array [row line][column line] of (x, y)."""
        q = np.asarray(quad, float)
        horizontal = [fit_line(pts) for pts in zip(*self._tracks(q, cols, rows, vertical=False))]
        vertical = [fit_line(pts) for pts in zip(*self._tracks(q, cols, rows, vertical=True))]
        return np.array([[intersect(horizontal[r], vertical[c]) for c in range(cols + 1)] for r in range(rows + 1)])

    def _tracks(self, q, cols: int, rows: int, vertical: bool):
        """For each sampling band across the grid, the position of every successive line it crosses."""
        bands, lines = (rows, cols) if vertical else (cols, rows)
        pitch = np.linalg.norm(q[1] - q[0]) / cols if vertical else np.linalg.norm(q[3] - q[0]) / rows
        tracks = []
        for band in range(bands):
            for sub in (0.3, 0.7):
                tracks.append(self._track(q, cols, rows, band, sub, lines, pitch, vertical))
        return tracks

    def _track(self, q, cols, rows, band, sub, lines, pitch, vertical):
        if vertical:
            xa, ya = bilinear(q, 0, (band + sub - 0.12) / rows)
            _, yb = bilinear(q, 0, (band + sub + 0.12) / rows)
            cross = np.arange(int(ya), int(yb) + 1)
            centre = (ya + yb) / 2
            slant = bilinear(q, 1, (band + sub) / rows)[1] - bilinear(q, 0, (band + sub) / rows)[1]
            start, span, edges = xa, pitch * cols, self.edge_x
        else:
            xa, ya = bilinear(q, (band + sub - 0.12) / cols, 0)
            xb, _ = bilinear(q, (band + sub + 0.12) / cols, 0)
            cross = np.arange(int(xa), int(xb) + 1)
            centre = (xa + xb) / 2
            slant = bilinear(q, (band + sub) / cols, 1)[0] - bilinear(q, (band + sub) / cols, 0)[0]
            start, span, edges = ya, pitch * rows, self.edge_y
        guess, found, steps = start, [], []
        for i in range(lines + 1):
            half = 14 if i == 0 else 5
            candidates = np.arange(int(round(guess)) - half, int(round(guess)) + half + 1)
            shift = int(round(slant * (guess - start) / span))
            if vertical:
                profile = edges[np.ix_(cross + shift, candidates)].mean(0)
            else:
                profile = edges[np.ix_(candidates, cross + shift)].mean(1)
            position = candidates[np.argmax(profile)]
            if i > 0:
                steps.append(position - found[-1])
            found.append(position)
            guess = position + (np.median(steps) if len(steps) >= 3 else pitch)
        return [(centre + slant * (p - start) / span, p) for p in found]

    def snap_line(self, p0, p1, search: int):
        """Moves each end of a segment by up to `search` pixels to lie on the darkest line nearby."""
        d = p1 - p0
        normal = np.array([-d[1], d[0]]) / np.linalg.norm(d)
        best = None
        for a in range(-search, search + 1):
            for b in range(-search, search + 1):
                score = np.percentile(self._sample(p0 + normal * a, p1 + normal * b), 60)
                if best is None or score < best[0]:
                    best = (score, a, b)
        _, a, b = best
        return p0 + normal * a, p1 + normal * b

    def _sample(self, p0, p1):
        n = max(int(np.linalg.norm(p1 - p0)), 10)
        pts = p0[None] + (p1 - p0)[None] * np.linspace(0.0, 1.0, n)[:, None]
        xs = pts[:, 0].astype(np.float32)[None]
        ys = pts[:, 1].astype(np.float32)[None]
        return cv2.remap(self.ink, xs, ys, cv2.INTER_LINEAR)[0]


def fit_line(points):
    """Robust straight line value = a * arg + b through (arg, value) points."""
    pts = np.array(points, float)
    keep = np.ones(len(pts), bool)
    for _ in range(3):
        coef = np.polyfit(pts[keep, 0], pts[keep, 1], 1)
        residual = np.abs(pts[:, 1] - np.polyval(coef, pts[:, 0]))
        keep = residual < max(1.5, np.median(residual) * 2.5)
    return coef


def intersect(horizontal, vertical) -> np.ndarray:
    a, b = horizontal  # y = a * x + b
    c, d = vertical  # x = c * y + d
    y = (a * d + b) / (1 - a * c)
    return np.array([c * y + d, y])


def meet(l1, l2) -> np.ndarray:
    """Intersection of two lines given by two points each."""
    (a, b), (c, d) = l1, l2
    t = np.linalg.solve(np.array([b - a, c - d]).T, c - a)[0]
    return a + (b - a) * t


def cell(corners: np.ndarray, col: int, row: int) -> np.ndarray:
    return np.array([corners[row, col], corners[row, col + 1], corners[row + 1, col + 1], corners[row + 1, col]])


def trim_grid_lines(im: np.ndarray, max_cut: int = 6) -> np.ndarray:
    """Strips outer rows and columns that are mostly ink unlike the cell background (a leftover grid line)."""
    h, w = im.shape[:2]
    band = np.concatenate([
        im[6:10].reshape(-1, 3), im[-10:-6].reshape(-1, 3),
        im[:, 6:10].reshape(-1, 3), im[:, -10:-6].reshape(-1, 3),
    ]).astype(int)
    background = np.median(band, axis=0)

    def inky(pixels) -> float:
        return (np.abs(pixels.astype(int) - background).sum(1) > 90).mean()

    t = b = l = r = 0
    while t < max_cut and inky(im[t]) > 0.45:
        t += 1
    while b < max_cut and inky(im[h - 1 - b]) > 0.45:
        b += 1
    while l < max_cut and inky(im[:, l]) > 0.45:
        l += 1
    while r < max_cut and inky(im[:, w - 1 - r]) > 0.45:
        r += 1
    return im[t : h - b, l : w - r]


def fanatic_strips(sheet: Sheet, items: np.ndarray) -> dict:
    """The six fanatic strips, keyed by token name, lying flat."""
    strips = {}
    left = (items[0, 5], items[-1, 5])
    right = sheet.snap_line(items[0, 5] + np.array([31, 0]), items[-1, 5] + np.array([31, 0]), search=5)

    def across_column(y):
        a = meet(left, (np.array([0, y]), np.array([1000, y])))
        b = meet(right, (np.array([0, y]), np.array([1000, y])))
        return sheet.snap_line(a, b, search=4)

    for strength, (y0, y1) in XEPHIS_STRIPS.items():
        top = (items[0, 0], items[0, 5]) if y0 < 1060 else across_column(y0)
        bottom = across_column(y1)
        quad = [meet(left, top), meet(right, top), meet(right, bottom), meet(left, bottom)]
        strips[f"fanatic-xephis-{strength}"] = warp_quad(sheet.img, quad, inset=2, rotate=cv2.ROTATE_90_CLOCKWISE)

    top = (items[-1, 0], items[-1, 5] + (items[-1, 5] - items[-1, 4]) * 0.4)
    bottom = sheet.snap_line(top[0] + np.array([0, 121]), top[1] + np.array([0, 121]), search=5)

    def down_row(x):
        a = meet(top, (np.array([x, 0]), np.array([x, 3000])))
        b = meet(bottom, (np.array([x, 0]), np.array([x, 3000])))
        return sheet.snap_line(a, b, search=4)

    for strength, (x0, x1) in YABOUL_STRIPS.items():
        l, r = down_row(x0), down_row(x1)
        quad = [meet(l, top), meet(r, top), meet(r, bottom), meet(l, bottom)]
        strips[f"fanatic-yaboul-{strength}"] = warp_quad(sheet.img, quad, inset=2, rotate=cv2.ROTATE_90_CLOCKWISE)
    return strips


def ink_strength(im: np.ndarray):
    """Per pixel 0 (paper) to 1 (full ink), and the paper colour."""
    f = im.astype(float)
    paper = np.median(f.reshape(-1, 3), axis=0)
    return np.clip(np.abs(f - paper).max(2) / 120, 0, 1), paper


def erase_and_brown(im: np.ndarray, mask: np.ndarray) -> np.ndarray:
    """Inpaints the masked ink away and redraws the remaining ink in brown."""
    clean = cv2.inpaint(im, cv2.dilate(mask, np.ones((3, 3), np.uint8)), 3, cv2.INPAINT_TELEA)
    k, paper = ink_strength(clean)
    return (paper[None, None] * (1 - k[..., None]) + BROWN[None, None] * k[..., None]).astype(np.uint8)


def draw_question_mark(im: np.ndarray, cx: float, cy: float, scale: float, thickness: int) -> None:
    (w, h), _ = cv2.getTextSize("?", cv2.FONT_HERSHEY_DUPLEX, scale, thickness)
    cv2.putText(im, "?", (int(cx - w / 2), int(cy + h / 2)), cv2.FONT_HERSHEY_DUPLEX, scale,
                BROWN.tolist(), thickness, cv2.LINE_AA)


def hidden_cult_marker(xephis: np.ndarray) -> np.ndarray:
    """The Xéphis sign with its lettering removed and a "?" on it."""
    h, w = xephis.shape[:2]
    k, _ = ink_strength(xephis)
    inside = np.zeros((h, w), np.uint8)
    cv2.rectangle(inside, (int(w * 0.15), int(h * 0.23)), (int(w * 0.86), int(h * 0.76)), 255, -1)
    out = erase_and_brown(xephis, ((k > 0.2) & (inside > 0)).astype(np.uint8) * 255)
    draw_question_mark(out, w * 0.52, h * 0.47, 1.0, 2)
    return out


def hidden_fanatic(yaboul_1: np.ndarray) -> np.ndarray:
    """The strength-1 Yaboul strip with its glyph replaced by a "?"."""
    h, w = yaboul_1.shape[:2]
    k, _ = ink_strength(yaboul_1)
    glyph = np.where((k > 0.4).sum(0) > h * 0.3)[0]  # columns where the glyph stands above the arrow line
    x0, x1 = glyph.min() - 3, glyph.max() + 3
    mask = np.zeros((h, w), np.uint8)
    mask[:, x0:x1] = ((k[:, x0:x1] > 0.2) * 255).astype(np.uint8)
    out = erase_and_brown(yaboul_1, mask)
    draw_question_mark(out, (x0 + x1) / 2, h * 0.5, 0.62, 1)
    return out


def main() -> None:
    output = Path(sys.argv[1] if len(sys.argv) > 1 else REPO / "img")
    sheet = Sheet(load_photo())
    small = sheet.grid(SMALL_GRID, 7, 16)
    items = sheet.grid(ITEM_GRID, 5, 16)
    tokens = {name: warp_quad(sheet.img, cell(small, c, r)) for name, (c, r) in SMALL_TOKENS.items()}
    for name, (c, r) in ITEM_TOKENS.items():
        token = trim_grid_lines(warp_quad(sheet.img, cell(items, c, r)))
        tokens[name] = token[: token.shape[0] - EXTRA_BOTTOM_CUT.get(name, 0)]
    tokens.update(fanatic_strips(sheet, items))
    tokens["cult-hidden"] = hidden_cult_marker(tokens["cult-xephis"])
    tokens["fanatic-hidden"] = hidden_fanatic(tokens["fanatic-yaboul-1"])
    for name, token in sorted(tokens.items()):
        path = output / f"token-{name}.png"
        cv2.imwrite(str(path), token)
        print(f"{path}: {token.shape[1]}x{token.shape[0]}")


if __name__ == "__main__":
    main()
