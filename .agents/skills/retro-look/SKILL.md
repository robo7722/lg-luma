---
name: retro-look
description: Design and implement Luma's Retro TV design — a stylised 1990s cable-TV info channel look (framed blue gradient panels, bold yellow lettering with heavy black outline, white labels / yellow values, channel numbers, bottom ticker, scanlines, tracking-noise cuts) that respects Luma's partial-redraw GLES2 renderer. Use whenever working on the Retro TV design, src/retro-view.rs, a retro variant of the drawer, Settings or Recents, retro sounds or shaders, or when asked for a "retro", "90s TV", "info channel", "weather channel", "VHS" or "CRT" look in Luma.
---

# Retro TV look for Luma

Retro TV is an alternative **design** of the same launcher (`Preferences::design = retro`,
Settings → General → Design). It is not a fork and not a separate app. Inventory, launching,
inputs, takeover and preferences are shared. Background and decisions:
`docs/notes/alternative-version-konzept.md`. Current implementation: `src/retro-view.rs`
(Home), `State::retro_*` in `src/state.rs`, tests in `src/retro-tests.rs`.

**The vibe:** a 1990s cable-TV info channel seen on a CRT: framed blue panels, chunky outlined
lettering, everything in capitals, a channel number, a ticker along the bottom. It must be
**stylised and original**. Do not copy a real channel's or show's logo, name, layout, music or
fonts (no Weather Channel or WeatherStar branding, no "Local on the 8s" wording).

Read [references/effects.md](references/effects.md) before touching shaders or adding an effect.

## 1. Non-negotiables

1. **Readable first.** A 32 px label must stay crisp at 3 m. Effects never sit on text above 15 %.
2. **Idle stays cheap.** The user asked for animated old-TV faults, so retro pages redraw at
   30 fps (`RETRO_FAULT_NANOS`) for the CRT beam and short tracking bursts (`tv_faults`), all off
   with Reduce motion. Keep every animated draw small or brief so partial redraw stays cheap.
   Besides that, only the half-second cursor blink and the ticker's 8 s cut change; never add
   continuous full-screen animation.
3. **Same navigation contract.** Back, OK, hold-OK menu, Move, Recents, settings subpages behave as
   in Liquid Glass. Retro pages may change the *shape* (vertical lists), not the semantics.
4. **No Liquid Glass leftovers** on retro pages (no `Draw::Glass`/`Draw::Rim`, no glide, no lift),
   and no retro code path runs while Liquid Glass is selected.
5. **Weather only in the ticker.** The user decided this. The left box shows the selected item.
6. **No new runtime dependencies, no bitmap UI.** Shapes are draw commands or `.lvec` icons.
7. **No hardware claims** from desktop previews (see `agents.md`).

## 2. Palette (constants at the top of `src/retro-view.rs`)

| Name | Hex | Use |
| --- | --- | --- |
| `NAVY` | `#070D3A` | screen background |
| `PANEL_TOP` → `PANEL_BOTTOM` | `#2A3FB8` → `#14227E` | vertical gradient inside panels (12 bands) |
| `FRAME` | `#9FB2FF` | 3 px light panel frame (plus a 2 px dark inner line) |
| `YELLOW` | `#F2CF2E` | headings, values, focus, cursor |
| `WHITE` | `#F2F2F2` | labels |
| `CYAN` | `#7FD8FF` | row / channel numbers |
| `ORANGE` | `#E8892A` | preview-box frame, ticker label block |
| black | `#000000` @ 80–95 % | outlines and drop shadows |

Stay inside broadcast-safe values: no pure `#FFFFFF`, no pure `#0000FF`, red only sparingly.

## 3. Lettering

- Weight **800** (generated weights are 350–650 and 800; 750 does not exist), all **uppercase**.
- **Headings / big numbers:** `outlined()`, i.e. five black copies (four diagonals at `size/16`
  clamped 2–6 px, plus a drop shadow) under the coloured text.
- **Everything else:** `shadowed()`, a single hard black shadow at +3,+3 px.
- Sizes (logical px): title 64, preview channel 52, menu rows 38, ticker 36, clock 34, date 22.
- The header shows only the title. Clock and date live in the orange ticker label block (0,967 460×74), white with shadow.
- Long text uses `assets.fit_line` / `fit_text`. Never wrap a row.
- Non-Latin scripts keep the bundled Noto fonts. `to_uppercase()` is safe for scripts without case.

## 4. Home layout (1920 × 1080)

```
┌ title panel 96,36 1728×140 ──────────────────────────────────────────────┐
│ HAUPTMENÜ (outlined yellow) — nothing else in the header                 │
└──────────────────────────────────────────────────────────────────────────┘
┌ main panel 96,200 1728×740 ──────────────────────────────────────────────┐
│ ┌ preview 150,252 470×620 ┐   ▶ 01  NETFLIX                              │
│ │      CH 01              │     02  YOUTUBE                              │
│ │  ┌ black bezel ───────┐ │     …                                       │
│ │  │ app card + CRT     │ │     07  ALLE APPS                       41   │
│ │  └────────────────────┘ │     08  EINGÄNGE                         2   │
│ │      NETFLIX            │     09  LAUNCHER-EINSTELLUNGEN        LUMA   │
│ └─────────────────────────┘                                      1 / 2  │
└──────────────────────────────────────────────────────────────────────────┘
[ 20:41 / DONNERSTAG, 22. OKTOBER ] WIEN: 14 °C, KLARER HIMMEL   ← ticker 0,964 1920×80, cuts every 8 s
```

- Rows: favorites, then All apps (`Nav(1)`), Inputs (`InputsMenu`), Luma settings, TV settings.
  `RETRO_ROWS` = 9 per page. The page follows the focus, and "n / m" shows when there is more than one page.
- Focused row: yellow 16 % fill, 3 px yellow border, yellow label, ▶ cursor blinking at 1 Hz.
  In move mode the fill is 32 % and the cursor is a steady ↕.
- Preview: app rows use `app_face` (artwork or curated logo). Other rows use a yellow `.lvec` icon
  (`grid`, `input-hdmi`, `luma-mark`, `tv`) with a black shadow. The screen gets 3 px scanlines and a top glare.
- Ticker messages: key hints (`retroHint`) and weather (only if enabled and read). The date is in the label block.

## 5. Motion and sound

- No easing, no glides: focus jumps, pages cut.
- Page appearance: one 0.24 s band of tracking noise rolling upward (`glitch()`), skipped with Reduce motion.
- Sounds: `State::cue` swaps Focus/Select/Back/Bump for `Cue::Retro*` synth blips while the design
  is retro, and entering retro Home plays `Cue::Tracking` (tape noise). All of them follow the Sounds toggle.
- Background music (`retro-music.wav`, smooth jazz chosen by the user) only plays on retro Home with
  `retro_music` on (default off). It fades out for app launches, other pages and background, and resumes where it stopped.
- New sounds: generate with ElevenLabs into `tools/sound-design/retro/`, match the format and levels
  (see its README), and let the user listen to the candidates before anything is committed.

## 6. Extending to other pages (next phases)

Settings are done via `restyle()` (a filter over the finished Glass scene, see development.md).
All apps (`Page::Apps`) is done as a retro menu page (`retro_list()`, `page()`). Remaining:
Recents. Either reuse `restyle()` or, for a real
layout change, follow these steps:
1. Gate on `state.preferences.design == Design::Retro` in `view::scene`, mirroring `retro_list()`.
2. Reuse `panel`, `outlined`, `shadowed` and `scanlines` from `retro-view.rs` (make them `pub(super)`).
3. Keep the targets and controls of the Glass page so `state.rs` navigation keeps working. Only change
   geometry, or add a retro-specific direction branch like `retro_direction`.
4. Add tests in `src/retro-tests.rs`: no glass draws, same targets, navigation, Glass unchanged.

## 7. OLED care

The layout is static and bright. Keep the LG screen saver allowed (`SDL_VIDEO_ALLOW_SCREENSAVER`),
let Evening mode dim it later, and consider a ±2 px shift of the whole layout every few minutes
(one full redraw). Never animate continuously for this.

## 8. Verify

**User rule:** Always show the user a design image (desktop screenshot) of every visual change
and wait for their OK **before committing or pushing**.

1. `pnpm assets` (needs `woff2_decompress` and Python fontTools, or `nix develop`).
2. Desktop capture, e.g. with `"design":"retro"` in a preferences file:
   `./target/debug/luma --assets .build/assets --preferences prefs.json --weather-test-server http://127.0.0.1:1 --time 2026-10-22T20:41:03+02:00 --script home --frames 90 --capture-scale 1 --screenshot out.png`
   (run under `xvfb-run -a` on Linux).
3. `cargo test --locked -p luma retro`, then `pnpm test`, `pnpm check` and `xvfb-run -a pnpm test:smoke`.
4. Ask the user to check on the TV: readability, number keys on their remote, smoothness.

## 9. Don't

- Don't build a full CRT post-process (barrel distortion, bloom, persistent roll). There is no FBO pass.
- Don't use `sin()`-hash noise at mediump. Use a noise texture (effects.md) or CPU-side xorshift as in `glitch()`.
- Don't copy shader code without a CC0/MIT licence and attribution in `assets/licenses/`.
- Don't put weather back into the main panel.
- Don't make it cartoonish: no fake brand logos, no "LIVE" bugs, no animated idle gimmicks.
