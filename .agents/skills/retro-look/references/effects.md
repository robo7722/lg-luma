# Retro TV effects for Luma's GLES2 renderer

Luma renders the logical 1920×1080 scene as a list of draw commands with **partial redraw**
(`src/damage.rs`). There is no full-screen framebuffer pass. Shaders are GLSL ES 1.0 with a
`PAINT_PRECISION` macro (may be `mediump` on TVs). Every recipe below is designed for that:
static when idle, cheap, and expressible as draw commands or a new `paint.frag` branch.

The recipes are written for this project. Godot Shaders' CC0 "VHS and CRT monitor effect" and
"Realistic CRT shader" were used as inspiration. If you port code from them, add attribution in `assets/licenses/`.

## Effect budget

| Effect | When | Cost | How |
| --- | --- | --- | --- |
| Scanlines | always (Retro TV) | static, 1 quad | repeated 1×4 alpha texture or `paint.frag` branch, opacity 8–12 % |
| Text drop shadow | always | 1 extra text draw | same text, black 60 %, +3,+3 px |
| Chroma fringe on title | optional | 2 extra text draws | red copy −2 px x, cyan copy +2 px x, each 18 % alpha, under the main text |
| Vignette | always | static, 1 quad | radial darkening ≤ 20 % at corners, drawn once |
| Cursor blink | at rest | tiny damage rect | toggle visibility at 500 ms |
| Tracking glitch | page change only, 180–240 ms | full redraw for ~12 frames | `paint.frag` branch with `u_retro_time` |
| Noise | only inside the glitch | in the glitch shader | 64×64 noise texture, scrolled |

Nothing on the list runs continuously except the cursor.

## Scanlines (static)

With a texture: build a 1×4 RGBA tile once at startup (rows: 0, 0, 0.5, 0 alpha) with
`GL_REPEAT` vertically (power-of-two size, so GLES2 can repeat it). Draw one full-screen quad with
UV `y = logical_y / 4.0` and a 10 % tint. Because the quad never changes, damage tracking ignores
it, except that it is redrawn inside every damaged rect, which is correct.

As a shader branch (no texture), using `v_logical_y` which `paint.vert` already provides:

```glsl
// scanline darkening, one dark line every 4 logical px, softened
float line = mod(v_logical_y, 4.0);
float dark = smoothstep(2.0, 3.0, line) * (1.0 - smoothstep(3.0, 4.0, line));
gl_FragColor = vec4(0.0, 0.0, 0.0, dark * u_scan_strength); // u_scan_strength ≈ 0.10
```

At 4K drawables the logical grid still maps to 4 logical px, so line density stays CRT-like.

## Tracking glitch (page change only)

New sentinel branch in `paint.frag` (follow the existing pattern: glass uses `v_params.y < -999.0`,
the rim `< -2999.0`; pick e.g. `< -3999.0` for "retro sample"). The quad samples the *previous or
next page* texture, if one is available. Otherwise apply it as a darkening/brightening overlay band
over the freshly drawn page. That is cheaper and needs no extra texture:

```glsl
// t: 0..1 progress of the glitch (uniform), y: logical y
float band_center = mix(1180.0, -100.0, t);          // band rolls upward once
float band = 1.0 - smoothstep(0.0, 90.0, abs(v_logical_y - band_center));
vec2 nuv = vec2(v_logical_x / 64.0, v_logical_y / 64.0 + t * 7.0);
float n = texture2D(u_noise, nuv).r;                  // 64×64 noise tile, GL_REPEAT
float alpha = band * (0.35 + 0.4 * n) * (1.0 - t * 0.6);
gl_FragColor = vec4(vec3(0.92), alpha);               // white-ish tracking band with noise
```

Horizontal jitter of the content (the classic "tear") can be faked without sampling: draw the new
page's top-level commands with a per-row-band x offset of ±6–12 px for the first 80 ms only.
For that, the view emits a `retro_jitter` offset that `renderer.rs` applies to commands inside the band.

Rules:
- Duration 180–240 ms, ease none, a single pass. Never loop.
- Declare a full-screen damage for the glitch frames, then return to normal partial redraws.
- Respect the existing "reduce motion" behaviour if added later: skip to a hard cut.

## Noise texture

Generate a 64×64 single-channel tile with a seeded xorshift at startup (or at build time in
`tools/`). Upload as `GL_LUMINANCE`, `GL_REPEAT`, `GL_NEAREST`. Do **not** compute hash noise with
`fract(sin(dot(...)) * 43758.5)`. At mediump it bands or turns into stripes on Mali/PowerVR TV GPUs.

## Chroma fringe (text)

No shader needed. For title text only:

1. red `#D8352C` @ 18 %, offset (−2, 0)
2. cyan `#5CD6E8` @ 18 %, offset (+2, 0)
3. black shadow @ 60 %, offset (+3, +3)
4. main text

Skip the fringe on menu rows and small text, because it costs legibility.

## Vignette

One full-screen quad with a radial alpha falloff (`smoothstep(0.55, 1.05, length(uv - 0.5) * 1.4)` × 0.2,
black). Static. It may be combined into the scanline branch to save a draw.

## Checklist before merging an effect

- [ ] Idle frames draw nothing new (diagnostics profile).
- [ ] Text at 32 px is still crisp with the effect on (desktop screenshot, 100 % zoom).
- [ ] Liquid Glass design shows zero difference (screenshot diff).
- [ ] No new uniform is set while the Retro TV design is off.
- [ ] TV verification requested from the user (not claimed).
