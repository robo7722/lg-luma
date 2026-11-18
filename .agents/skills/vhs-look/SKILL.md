---
name: vhs-look
description: Design and implement Luma's 90s VHS / VCR on-screen-display theme — palette, OSD typography, layout grid, stepped motion, beeps, and cheap GLES2 effects (scanlines, tracking glitch, chroma fringe, noise) that respect Luma's partial-redraw renderer. Use whenever working on the VHS design mode, its Home layout, a VHS variant of any page, VHS shaders in src/shaders/, VHS sounds, or when asked for a "retro", "VCR", "VHS", "CRT" or "90s" look in Luma.
---

# VHS look for Luma

The VHS mode is an alternative **design** of the same launcher, switched in Settings. It is not a
fork and not a separate app. Inventory, launching, inputs, takeover and preferences are shared.
Only presentation (`src/view.rs`, `src/renderer.rs`, `src/shaders/`) and, for the VHS Home,
its navigation shape (`src/state.rs`) differ. Background: `docs/notes/alternative-version-konzept.md`.

The target is the **on-screen display of a 1990s VCR menu**, read on a CRT from the sofa: flat
colour, blocky monospaced capitals, a blinking cursor, hard cuts, and a short tracking wobble when
the "tape" changes. It should feel like real hardware, not like a parody of glitch art.

Read [references/effects.md](references/effects.md) before touching shaders or adding an effect.

## 1. Non-negotiables

1. **Readable first.** Every effect is subtle enough that a 32 px label stays crisp at 3 m.
   Effects never sit on top of text at more than 15 % strength.
2. **Idle means idle.** Luma redraws only damaged rectangles (`src/damage.rs`) and draws nothing
   while nothing changes. A VHS screen at rest may only animate the **cursor blink** (a small
   damage rect). No continuously animated full-screen noise, roll or flicker. That would keep the
   TV GPU busy forever.
3. **Same navigation contract.** Back, OK, hold-OK menus, move mode, Recents and settings
   subpages behave exactly like the Liquid Glass design. Only the Home *shape* changes (vertical list).
4. **One design switch.** Theme choice is a preference enum (pattern: `WeatherStyle` in
   `src/model.rs`) with a preview in the picker. No VHS code path may run while Liquid Glass is selected.
5. **No new runtime dependencies, no bitmaps for UI.** Shapes are draw commands or `.lvec`
   vectors. Textures are allowed only for tiny generated helpers (e.g. a 64×64 noise tile).
6. **No hardware claims** from desktop previews (see `agents.md`).

## 2. Palette (sRGB, kept inside broadcast-safe 16–235)

| Token | Hex | Use |
| --- | --- | --- |
| `vhs.blue` | `#1418A0` | full-screen OSD background ("blue screen") |
| `vhs.blue-deep` | `#0C0F6E` | bands, footer bar, inactive panels |
| `vhs.white` | `#EBEBEB` | normal text, icons |
| `vhs.yellow` | `#E8D02E` | focus / selected row, cursor ▶ |
| `vhs.cyan` | `#5CD6E8` | secondary info (CH, SP/LP, weather) |
| `vhs.green` | `#5CC85C` | "OK" states, connected input dot |
| `vhs.red` | `#D8352C` | REC dot, destructive actions only (red bleeds on CRT — use small) |
| `vhs.shadow` | `#000000` @ 60 % | 3 px hard drop shadow under all OSD text |

- Never pure `#FFFFFF` or `#0000FF`.
- Text always has the hard black drop shadow (offset +3,+3 logical px, no blur). That is the OSD signature.
- **Overlay variant:** instead of the blue screen, the user's wallpaper is dimmed to ~35 %
  with scanlines on top and the same OSD text. This is better for OLED owners who dislike a large blue field.
  Offer both as a sub-option once the base works.

## 3. Typography

- **Font:** a monospaced VCR/OSD face. Candidate: *Better VCR* (SIL OFL 1.1). Before adding it to
  `assets/fonts/`, verify the upstream source, licence text and glyph coverage, and add the licence
  next to `ofl.txt`. *VCR OSD Mono* has no verified licence — do not ship it.
  Until a font is verified, prototype with the existing font in a monospace-like layout.
- **Uppercase everything** in Latin, Greek and Cyrillic (`to_uppercase()`, so ß → SS). Scripts without case
  (CJK, Vietnamese tone marks may be unsupported) fall back to the existing Noto/Inter outlines on the same grid.
  Never drop glyphs.
- **Character grid:** layout aligns to a monospace cell. Sizes in logical 1920×1080 px:

  | Role | Size | Example |
  | --- | --- | --- |
  | Title | 64 | `═══ MAIN MENU ═══` |
  | Menu row | 48 | `▶ 01  NETFLIX` |
  | Corner OSD | 40 | `▶ PLAY`, `12:00`, `CH 03` |
  | Hint / footer | 32 | `◀◀ REW  ▶▶ FF  ■ STOP` |

- Two-digit row numbers (`01`–`99`) are part of the label. Digit keys jump to rows.
- Letter-spacing 0. Wide spacing comes from the monospace cell, not tracking.

## 4. Layout (VHS Home)

```
 safe area: 96 px left/right, 64 px top/bottom (CRT overscan feel)
 ┌──────────────────────────────────────────────────────────┐
 │ ▶ PLAY                                     12:00   SP    │ corner OSD, row 0
 │                 ═══  M A I N   M E N U  ═══              │ title, centred
 │       ▶ 01  NETFLIX                                      │ list: favorites first,
 │         02  YOUTUBE                                      │ then ALL PROGRAMS,
 │         03  PLEX                                         │ INPUT, SETTINGS
 │         04  ALL PROGRAMS …                               │ max 9 rows visible,
 │         05  INPUT                                        │ list scrolls by row (hard step)
 │         06  SETTINGS                                     │
 │ CH 03                       ◀◀ REW   ▶▶ FF   ■ STOP      │ footer hints
 └──────────────────────────────────────────────────────────┘
```

- Focused row: text `vhs.yellow`, a `▶` cursor in the gutter that blinks 1 Hz, no box or glow.
- Corner OSD: clock (24 h per user setting). Weather renders as `18°C CLEAR` in `vhs.cyan`.
  Inputs render as `HDMI 1 ●` with a green or grey dot.
- Navigation: Up/Down moves the cursor. OK launches. Hold OK opens the existing app menu as a
  VHS-styled box list. In move mode, Up/Down reorders. Back behaves like today.
- App drawer and Settings: in phase 1 they keep their layout but use the palette, font, drop shadow and scanlines.
  Full VHS versions come later.

## 5. Motion and sound

- **No easing, no glides.** Focus jumps instantly. Lists scroll in whole-row steps. Anything that
  "slides" on Liquid Glass simply cuts here.
- **Cursor blink:** 500 ms on / 500 ms off, damage only the cursor cell.
- **Tape change** (page change, opening drawer/settings): 180–240 ms tracking glitch (see
  effects.md), then a hard cut to the new page. This is the only full-screen animation.
- **Launch:** 150 ms "PLAY ▶" flash in the corner, then the existing launch flow.
- **Sounds:** short square-wave beeps (≈1 kHz focus, ≈1.5 kHz select, ≈600 Hz back, low buzz for bump),
  40–80 ms, quiet. They live next to the existing `assets/sounds/*.wav` as `vhs-*.wav` and use the same
  **Sounds** toggle and volume. Source the sound design in `tools/sound-design/` like the current clicks.

## 6. OLED care

- The blue screen and the static OSD are mostly bright, static and high-contrast.
  Keep the LG screen saver allowed (Luma already sets `SDL_VIDEO_ALLOW_SCREENSAVER`).
- Shift the whole OSD layer by ±2 px on a slow cycle (e.g. every 4 minutes, one full redraw), the
  way set-top boxes do. Never animate continuously for this.
- Evening mode dims the blue and the text like it dims glass today.

## 7. Implementation map

| Concern | Where |
| --- | --- |
| Theme preference + picker | `src/model.rs` (enum like `WeatherStyle`), `src/storage.rs` sanitize, settings view, `src/i18n.rs` + bundled translations for new labels |
| Home layout | `view.rs::home()` dispatches to a new `vhs_home()` |
| Home navigation | `state.rs`: vertical list targets reuse existing `Target`s (`Nav`, `InputsMenu`, `SettingsMenu` …) |
| Effects | new sentinel branch in `paint.frag` (existing ones: `v_params.y < -999` glass, `< -2999` rim) or plain draw commands. See effects.md |
| Font | `assets/fonts/` + `src/font.rs`, licence file, `assets.md` |
| Sounds | `assets/sounds/vhs-*.wav`, `src/audio.rs` |
| Docs | `readme.md`, `development.md`, `assets/homebrew-description.html` (no unverified TV claims) |

Every user-facing feature needs a changeset (`pnpm changeset`, minor). Run `pnpm version:release`
on the feature branch before the PR, as `agents.md` asks for this fork.

## 8. Verify

1. `pnpm dev`, switch Design → VHS, walk Home ↔ drawer ↔ settings ↔ recents with the keyboard
   remote. Check that Back always restores the previous focus.
2. Screenshot Home in both designs (`tools/screenshots.mjs`) and compare with the layout above.
   Check that text is crisp, the shadow is hard, and scanlines are faint.
3. Idle check: with nothing pressed, only the cursor cell should redraw (profiling mode in `src/diagnostics.rs`).
4. Run `pnpm check`, `pnpm test` and `xvfb-run -a pnpm test:smoke`.
5. Ask the user to verify on the TV: readability at distance, GPU smoothness of the tape-change glitch, burn-in comfort.

## 9. Don't

- Don't port a full CRT post-processing pipeline (barrel distortion, bloom, persistent roll). Luma
  has no full-screen FBO pass, and adding one costs every frame on old TVs.
- Don't add RGB split to text by shader. Draw two faint offset copies (see effects.md) if wanted.
- Don't use `sin()`-based hash noise at mediump. Sample a small noise texture instead.
- Don't copy shader code from the web without a CC0/MIT licence and attribution in `assets/licenses/`.
- Don't make it cartoonish: no fake tape labels, no "PAUSE" jitter at rest, no VHS logo.
