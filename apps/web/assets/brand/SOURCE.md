# Vela brand art (TR04b)

`vela-coast.png` and `vela-wave.png` are app-owned decorative assets for the Home
editorial feature and the My Wave module. They are served only from the exact
allowlisted paths `/assets/brand/vela-coast.png` and
`/assets/brand/vela-wave.png`. There is no runtime request to a third-party host.

## Provenance

Both files were retrieved from the user-approved visual reference
`https://prototyp-opal.vercel.app/` and their bytes match the SHA256 values
recorded in
`docs/PLANS/2026-10-19/PROTOTYPE_TRANSFER/06_ASSETS_AND_COPY.md` exactly:

| Local file | Original public URL | Dimensions | Bytes | SHA256 |
| --- | --- | --- | --- | --- |
| `vela-coast.png` | `https://prototyp-opal.vercel.app/images/coast.png` | 1376×768, RGB | 1,516,616 | `3e9fe7014f03512f64e43601a16f749d0ec3e683023c782579f34623be4732c0` |
| `vela-wave.png` | `https://prototyp-opal.vercel.app/images/your-wave.png` | 1024×1024, RGB | 1,230,494 | `ff27654ab61791306dca4bae75c2fdf8aba38adc607da7ce90c2fce0d2453db8` |

Verified locally with `sha256sum` on 2026-10-19 before installation. The files
are unmodified copies; no replacement, re-encoding or generated substitute was
produced.

- `vela-coast.png`: dark Icelandic shoreline, sea left, black beach and green
  cliffs right, overcast sky. Cropped `60% 53%` inside the editorial card with
  the two approved dark shading layers.
- `vela-wave.png`: matte translucent ribbon, lilac/powder/peach layers on a deep
  plum background. Desktop crop `center 32%`; mobile banner uses the inspected
  `center 39%` refinement. Purple stays confined to the My Wave module.

## Rights and use

These are the decorative module images of the user-approved reference. They are
used here as app-owned editorial decoration only:

- They never stand in for a real album cover, a user's own artwork or an artist
  portrait. Real covers always come from the existing authorized artwork service
  (`/api/artwork/{id}`).
- They are never routed through the authenticated download worker and are not
  mixed with private cover material.
- The `album-atlas.png` concept sheet is deliberately **not** shipped: it is a
  six-cell concept grid, not a set of real covers, and it must never appear in
  production catalog art.

The release gate for redistributing these images outside this installation is
recorded in `docs/KNOWN_LIMITATIONS.md` and `docs/CURRENT_STATE.md`. Shipping
them inside this self-hosted application is the approved transfer; broader
redistribution is not authorized by this repository.
