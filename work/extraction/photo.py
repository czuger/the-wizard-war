"""Shared helpers for cutting the map and the pawns out of the board photo."""

from pathlib import Path

import cv2
import numpy as np

REPO = Path(__file__).resolve().parents[2]
PHOTO = REPO / "work" / "6a013486df16fc970c0133f3bcf0b2970b.jpg"


def load_photo() -> np.ndarray:
    """Returns the board photo (BGR), or exits when it is missing."""
    img = cv2.imread(str(PHOTO))
    if img is None:
        raise SystemExit(f"cannot read {PHOTO}")
    return img


def bilinear(quad, u: float, v: float) -> np.ndarray:
    """Point at (u, v) in [0, 1]² inside a quad given as TL, TR, BR, BL."""
    q = np.asarray(quad, float)
    top = q[0] + (q[1] - q[0]) * u
    bottom = q[3] + (q[2] - q[3]) * u
    return top + (bottom - top) * v


def warp_quad(img: np.ndarray, quad, inset: int = 3, rotate=None) -> np.ndarray:
    """Straightens a quad (TL, TR, BR, BL) to a rectangle of its own size, minus `inset` pixels per side."""
    q = np.asarray(quad, float)
    w = int(round((np.linalg.norm(q[1] - q[0]) + np.linalg.norm(q[2] - q[3])) / 2))
    h = int(round((np.linalg.norm(q[3] - q[0]) + np.linalg.norm(q[2] - q[1])) / 2))
    m = cv2.getPerspectiveTransform(q.astype(np.float32), np.float32([[0, 0], [w, 0], [w, h], [0, h]]))
    out = cv2.warpPerspective(img, m, (w, h), flags=cv2.INTER_CUBIC)[inset : h - inset, inset : w - inset]
    return cv2.rotate(out, rotate) if rotate is not None else out
