# Sound design – Retro TV

Sources behind `assets/sounds/retro-*.wav`. Nothing in here is shipped in the IPK.

## Final selection (user's choice: set 1, music 1)

| `assets/sounds/`     | Source                         |
| -------------------- | ------------------------------ |
| `retro-focus.wav`    | `variants/focus-1.wav`         |
| `retro-select.wav`   | `variants/select-1.wav`        |
| `retro-back.wav`     | `variants/back-1.wav`          |
| `retro-bump.wav`     | `variants/bump-1.wav`          |
| `retro-tracking.wav` | `variants/tracking-1.wav`      |
| `retro-music.wav`    | `music/infochannel-1.mp3` loop |

## Generation

- `elevenlabs/*.mp3`: ElevenLabs Sound Effects (`eleven_text_to_sound_v2`, `prompt_influence` 0.6,
  0.5 s; tracking 0.8 s). Prompts: focus "single short clean synthesizer blip, 1990s cable TV info
  channel menu cursor move, soft square wave beep"; select "two quick rising synth beeps"; back "two
  quick falling soft synth beeps"; bump "very short dull low synth buzz"; tracking "short burst of
  analog VHS tape tracking static".
- `music/infochannel-1.mp3`: ElevenLabs Music (`music_v1`, 60 s): mellow early-90s smooth jazz,
  electric piano, fretless bass, brushed drums, saxophone, 90 BPM, loopable, no vocals.
  `infochannel-2.mp3` (synth-pop/new age) was not chosen.

## Processing

- Variants: leading silence removed (−40 dB), cut to focus 0.10 s, select/back 0.22 s, bump 0.18 s,
  tracking 0.45 s, 1 ms fade-in, 15 ms fade-out, 48 kHz mono 16-bit, mean volume matched to the
  Liquid Glass clicks (focus −26 dB, select −22.4, back −20.7, bump −18.4, tracking −24),
  limiter at 0.89.
- Music: downmixed to 24 kHz mono, near-silence trimmed, the last 3 s crossfaded into the first 3 s
  (seamless loop), peak-normalised to 0.89. The launcher mixes it at 0.14.
- `demo-sounds-{1,2}.wav`: navigation demos used for the choice.
