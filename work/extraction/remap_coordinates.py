"""Converts the map's pixel coordinates in the Rust sources from an old board picture to the current one.

Used once, when img/board.jpg went from 1055x867 to the 2780x2289 cut made by
extract_map.py. SIFT features matched between the two pictures give a homography,
which moves every site position, `HALL_POSITION` (src/domain/board.rs) and every
district position (`District::board_position` in src/domain/district.rs).
Run it only with the picture the coordinates were measured on, e.g.:

    git show f4d7a7a:img/board.jpg > /tmp/old-board.jpg
    python work/extraction/remap_coordinates.py /tmp/old-board.jpg
"""

import re
import sys

import cv2
import numpy as np

from photo import REPO

TUPLE = re.compile(r"\((\d+), (\d+)\)")


def homography(old_path: str, new_path: str) -> np.ndarray:
    """Old picture pixels -> new picture pixels."""
    old = cv2.imread(old_path, cv2.IMREAD_GRAYSCALE)
    new = cv2.imread(new_path, cv2.IMREAD_GRAYSCALE)
    if old is None or new is None:
        raise SystemExit("cannot read the board pictures")
    scale = old.shape[1] / new.shape[1]  # match at similar sizes
    small = cv2.resize(new, None, fx=scale, fy=scale, interpolation=cv2.INTER_AREA)
    sift = cv2.SIFT_create(8000)
    k1, d1 = sift.detectAndCompute(old, None)
    k2, d2 = sift.detectAndCompute(small, None)
    good = [a for a, b in cv2.BFMatcher().knnMatch(d1, d2, k=2) if a.distance < 0.7 * b.distance]
    p1 = np.float32([k1[m.queryIdx].pt for m in good])
    p2 = np.float32([k2[m.trainIdx].pt for m in good]) / scale
    h, mask = cv2.findHomography(p1, p2, cv2.RANSAC, 3.0)
    inliers = mask.ravel() == 1
    error = np.linalg.norm(cv2.perspectiveTransform(p1[inliers][None], h)[0] - p2[inliers], axis=1)
    print(f"{inliers.sum()} matches, mean error {error.mean():.2f} px")
    return h


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    h = homography(sys.argv[1], str(REPO / "img" / "board.jpg"))

    def convert(match: re.Match) -> str:
        x, y = cv2.perspectiveTransform(np.float32([[[int(match[1]), int(match[2])]]]), h)[0, 0]
        return f"({int(round(x))}, {int(round(y))})"

    board = REPO / "src" / "domain" / "board.rs"
    head, tail = board.read_text().split("static PATHS", 1)  # the path table holds site ids, not pixels
    head = TUPLE.sub(convert, head.replace("(u16, u16)", "@TYPE@")).replace("@TYPE@", "(u16, u16)")
    board.write_text(head + "static PATHS" + tail)

    district = REPO / "src" / "domain" / "district.rs"
    text = district.read_text()
    start = text.index("pub fn board_position")
    end = text.index("}\n    }", start)
    district.write_text(text[:start] + TUPLE.sub(convert, text[start:end]) + text[end:])


if __name__ == "__main__":
    main()
