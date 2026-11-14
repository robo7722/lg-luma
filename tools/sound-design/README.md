# Sound design – UI click sounds

Raw material and recipes behind `assets/sounds/*.wav`. Nothing in here is shipped in the IPK.

## Final selection (what the user chose)

| `assets/sounds/` | Source variant          | Recipe                    |
| ---------------- | ----------------------- | ------------------------- |
| `focus.wav`      | `variants/focus.wav`    | tick only                 |
| `select.wav`     | `variants/select-b.wav` | double tick (rising)      |
| `back.wav`       | `variants/back-b.wav`   | double tick (falling)     |
| `bump.wav`       | `variants/bump-b.wav`   | dull low double tick      |

The user liked `elevenlabs/focus-1.mp3` best. All final sounds are built from that one tick so the set
sounds consistent. The user did **not** like the first mixes (`variants/first-mix-*.wav`, tick layered
with the wooden knock from `elevenlabs/bump-5.mp3`) nor the a/c variants.

## Files

- `elevenlabs/` – raw ElevenLabs Sound Effects v2 output (0.48 s MP3, 48 kHz stereo).
  `focus-*`, `select-1..3`, `back-1..3`, `bump-1..3` came from a first prompt set; most of
  `select-1/2`, `back-1/2`, `bump-1..3` are nearly silent (peak ≈ 1 %). `select-4/5`, `back-4/5`,
  `bump-4/5` are a second, louder prompt set.
- `variants/` – mixed candidates, 48 kHz mono 16-bit WAV.
- `listen.html` – listening page for the variants. Serve this folder over HTTP (e.g.
  `python -m http.server -d tools/sound-design`) and open `/listen.html`; `file://` does not
  play the sounds.

## Recipe

All variants were rendered in the browser with Web Audio (`OfflineAudioContext`, 48 kHz, mono):

- `tick` = first 40 ms of `elevenlabs/focus-1.mp3`, downmixed to mono.
- `knock` = 160 ms of `elevenlabs/bump-5.mp3` starting at 17.6 ms (just before its onset).
- Each layer: `playbackRate` (pitch), optional biquad filters, gain, start offset `at` (s).
- Finish: normalise peak to `target`, 1 ms fade-in, 10 ms linear fade-out.

| Variant    | Length | Layers                                                                     | Peak |
| ---------- | ------ | -------------------------------------------------------------------------- | ---- |
| `focus`    | 60 ms  | tick                                                                       | 0.45 |
| `select-b` | 120 ms | tick; tick at 45 ms, rate 1.15, gain 0.9                                   | 0.60 |
| `back-b`   | 120 ms | tick rate 1.1; tick at 45 ms, rate 0.8, gain 0.9                           | 0.50 |
| `bump-b`   | 140 ms | tick rate 0.5, lowpass 1500 Hz; tick at 50 ms, rate 0.45, gain 0.5, lowpass 1200 Hz | 0.45 |

Then every `select-*`, `back-*`, `bump-*` variant was loudness-matched against `focus.wav`
(RMS of the samples above 5 % of the peak): target RMS = focus RMS × 1.4 for select and × 1.6
for back/bump, followed by a soft limiter `tanh(1.2·x) / tanh(1.2)`. Resulting peaks are
0.75–0.89.

Licence/origin: see `assets/licenses/ui-sounds.md` (generated with ElevenLabs on a free plan).
