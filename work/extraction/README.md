# Cutting the map and the pawns out of the board photo

These scripts made `img/board.jpg` and `img/token-*.png` from
`work/6a013486df16fc970c0133f3bcf0b2970b.jpg`, at the photo's own resolution.

```sh
python3 -m venv /tmp/extraction-venv
/tmp/extraction-venv/bin/pip install -r work/extraction/requirements.txt
/tmp/extraction-venv/bin/python work/extraction/extract_map.py      # img/board.jpg
/tmp/extraction-venv/bin/python work/extraction/extract_tokens.py   # img/token-*.png
```

Both take an optional output path (a file for the map, a folder for the pawns), to try
changes without touching `img/`.

- `extract_map.py` traces the map's black frame and straightens it with a Coons patch
  (perspective and the poster's curvature), then shaves the frame line off: 2780×2289.
- `extract_tokens.py` finds the grid lines of the counter sheet, cuts the cleanest copy
  of each pawn at its own size and turns fanatic strips flat. The face-down cult marker
  and fanatic are invented (the printed pawns have no back): they are made from the
  Xéphis marker and the strength-1 Yaboul strip, emptied and given a brown "?".
  Which cell gives which pawn is listed at the top of the script.
- `remap_coordinates.py` was run once to move the site, district and hall coordinates
  of the Rust sources from the previous 1055×867 map to this one. It is kept for the
  record; rerunning it needs the picture the coordinates were measured on.

The positions in the photo (frame search windows, grid corners, strip bounds) are
measured on this particular photo: another photo needs them measured again.
