# workflo logo

A block split by one smooth step: two persisted states and the fluid transition between them. The
name is always lowercase and has no final w: `workflo`.

## Files

| File | What it is |
|---|---|
| `workflo-logo-light.svg` | Horizontal logo for light backgrounds (teal symbol, ink wordmark) |
| `workflo-logo-dark.svg` | Horizontal logo for dark backgrounds (teal symbol, paper wordmark) |
| `workflo-banner.png` | 2560 x 640 banner with its own dark background, readable on any theme |
| `workflo-icon.svg` | Square icon: the symbol drawn as the 180 tile of a 200 x 200 viewBox |
| `workflo-favicon.svg` | Small-size cut of the symbol for 16 to 48 px: full-bleed tile, wider slit |
| `workflo-favicon.ico` | The small-size cut rendered at 16, 32 and 48 px |
| `workflo-avatar.png` | 512 x 512 avatar: symbol on a full ink square, safe for circular crops |
| `workflo-social.png` | 1280 x 640 card: logo, tagline, `workflo::migrate(&pool).await?;` |

The slit in the symbol is transparent, so the icon and the favicon take the colour of whatever is
behind them and work on light and dark themes without a variant. The dark logo and the banner use
a slit 4 units wider on the 256 grid: a dark gap inside a light tile looks thinner than it is.

## What to use where

### README

The root `README.md` already uses the header below. GitHub picks the light or dark SVG from the
viewer's theme. GitLab strips `<picture>` and `<source>`, so it shows the `<img>` fallback, the
banner, which reads on both GitLab themes.

```html
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/logo/workflo-logo-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="assets/logo/workflo-logo-light.svg">
  <img alt="workflo" src="assets/logo/workflo-banner.png" width="560">
</picture>
```

### docs.rs

rustdoc shows a logo in the sidebar and a favicon in the tab. Both need absolute URLs, set once at
the top of `src/lib.rs`:

```rust
#![doc(
    html_logo_url = "https://gitlab.com/ThomasTartrau/workflo/-/raw/main/assets/logo/workflo-icon.svg",
    html_favicon_url = "https://gitlab.com/ThomasTartrau/workflo/-/raw/main/assets/logo/workflo-favicon.ico"
)]
```

### Portfolio (tartrau.fr project page)

- Project logo: `workflo-icon.svg`, copied to the site as `/logos/workflo.svg`. It follows the same
  template as the other project logos (200 viewBox, 180 tile, radius 40): here the tile is the
  symbol itself, in teal, and the page background shows through the slit.
- Project image: `workflo-social.png`, also the Open Graph image.

### LinkedIn

- Project section of the profile, or a post about workflo: `workflo-social.png` as the media.
- LinkedIn page logo, if a page is ever created: `workflo-avatar.png`.

### GitLab

- Project avatar: `workflo-avatar.png`, in Settings > General > Project avatar (GitLab recommends
  192 x 192 and 200 KB at most; it resizes the file).

### GitHub

- A repository has no icon. Set `workflo-social.png` in Settings > General > Social preview; it
  shows in link previews on LinkedIn, Slack, X and others.

## Colours

| Name | HEX | RGB | Use |
|---|---|---|---|
| Canal | `#0B9699` | 11 150 153 | The symbol only |
| Ink | `#0E0F12` | 14 15 18 | Wordmark on light backgrounds, dark tiles |
| Paper | `#F4F3EF` | 244 243 239 | Wordmark on dark backgrounds |

Canal keeps a contrast of at least 3:1 on every background the logo meets: 3.6:1 on white, 3.2:1
on Paper, 5.3:1 on Ink and 4.6:1 on the GitLab dark theme. Secondary text on the social card is
set in JetBrains Mono (SIL Open Font License); the wordmark itself is drawn, not typed.

## Rules

- Keep clear space around the logo at least as tall as the `o` of the wordmark.
- Minimum sizes: horizontal logo 100 px wide; symbol 16 px. Below 48 px use `workflo-favicon.*`,
  whose wider slit stays open on the pixel grid.
- On a dark background use `workflo-logo-dark.svg`, not a recoloured light file.
- The step always rises from left to right. Don't mirror or rotate the symbol, stretch it, recolour
  it, add effects, change the curve of the slit, or retype the wordmark in a font.
