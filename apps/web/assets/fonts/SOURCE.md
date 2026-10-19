# Manrope web subsets (TR01)

`manrope-latin.woff2`, `manrope-cyrillic.woff2` and `OFL.txt` are self-hosted
web assets. They are served only from the exact allowlisted paths
`/assets/fonts/manrope-latin.woff2`, `/assets/fonts/manrope-cyrillic.woff2` and
`/assets/fonts/OFL.txt`; there is no runtime font CDN request.

## Provenance

- Upstream font: `Manrope[wght].ttf` from the Google Fonts repository,
  `https://raw.githubusercontent.com/google/fonts/main/ofl/manrope/Manrope%5Bwght%5D.ttf`
  (retrieved 2026-10-19). SHA256
  `3ae11c49db0455a3cc33e37d380f20fdb8c7f8b41dc07625c177e3d87a9d6ae6`.
  Variable axis `wght` 200-800; source version "Version 2.000".
- License: SIL Open Font License 1.1, Copyright 2018 The Manrope Project
  Authors. `OFL.txt` is the upstream license file, SHA256
  `58172e0c0fac2cda8a37b348164bb55e44b0e69051e557e92b1d3f6910141f7b`.
  It matches the license hash recorded in
  `docs/PLANS/2026-10-19/PROTOTYPE_TRANSFER/06_ASSETS_AND_COPY.md`.
- These are not the prototype's Next.js files. The upstream OFL font was
  subset locally because redistribution under the OFL is permitted; the
  published prototype URLs were not hotlinked.

## Local subsetting

Built with `pyftsubset` (fontTools 4.67.0) and Brotli WOFF2 output, keeping
the `wght` variable axis, kerning/ligatures and tabular figures:

```sh
pyftsubset 'Manrope[wght].ttf' \
  --unicodes='U+0000-00FF,U+0131,U+0152-0153,U+02BB-02BC,U+02C6,U+02DA,U+02DC,U+0304,U+0308,U+0329,U+2000-206F,U+20AC,U+2122,U+2190-2199,U+2212,U+2215,U+FEFF,U+FFFD' \
  --flavor=woff2 --layout-features='kern,liga,clig,calt,tnum,case' \
  --name-IDs='*' --name-languages='*' --output-file=manrope-latin.woff2

pyftsubset 'Manrope[wght].ttf' \
  --unicodes='U+0301,U+0400-045F,U+0490-0491,U+04B0-04B1,U+2116' \
  --flavor=woff2 --layout-features='kern,liga,clig,calt,tnum,case' \
  --name-IDs='*' --name-languages='*' --output-file=manrope-cyrillic.woff2
```

Resulting SHA256:

- `manrope-latin.woff2` `2436b9c2b6fe71529530bb1e5b0139d9a9af81b9507bc1b4e09113d3119b5315` (23,056 bytes; 273 glyphs)
- `manrope-cyrillic.woff2` `d00bd0cdc67a28c770972d97c6a5f85bbed7f66c99541135d0ec967d11e77e87` (11,820 bytes; 131 glyphs)

Coverage tested: basic Latin, Russian А-я/Ё/ё, `№`, `«»`, `—`, `…`, `→`.
Greek, Vietnamese, Latin-ext and Cyrillic-ext are intentionally not shipped;
records that need them fall back to `Arial, sans-serif`. `↗` (U+2197) is not
present in upstream Manrope and is drawn by the fallback font.
