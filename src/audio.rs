//! Short UI click sounds, plus the optional background music of the Retro TV design. Clips are
//! embedded, decoded once at startup and mixed in SDL's audio callback thread, so playing a cue
//! only pushes a voice (and switching the music only flips a flag) under a short lock and never
//! blocks the SDL/render thread. Without a working audio device the launcher stays silent.
use std::sync::{Arc, Mutex};

/// A navigation cue queued by `State` and played by the event loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    /// Remote focus moved to another control.
    Focus,
    /// OK / Enter on a target.
    Select,
    /// Back, or closing an overlay or subpage.
    Back,
    /// A direction key reached an edge and nothing moved.
    Bump,
    /// Under the Settings section glide: a faint glass shimmer.
    Glide,
    /// Retro TV design: synth blips in place of the clicks above.
    RetroFocus,
    RetroSelect,
    RetroBack,
    RetroBump,
    /// Retro TV design: a short burst of tape tracking noise as Home appears.
    Tracking,
}

impl Cue {
    const ALL: [Self; 10] = [
        Self::Focus,
        Self::Select,
        Self::Back,
        Self::Bump,
        Self::Glide,
        Self::RetroFocus,
        Self::RetroSelect,
        Self::RetroBack,
        Self::RetroBump,
        Self::Tracking,
    ];

    /// The Retro TV design's version of a click; other cues are shared.
    pub fn retro(self) -> Self {
        match self {
            Self::Focus => Self::RetroFocus,
            Self::Select => Self::RetroSelect,
            Self::Back => Self::RetroBack,
            Self::Bump => Self::RetroBump,
            other => other,
        }
    }

    /// Mix level: all cues at a quarter of the clips' level, the glide far below the clicks.
    fn gain(self) -> f32 {
        let master = 0.25;
        master
            * match self {
                Self::Glide => 0.3,
                Self::Tracking => 0.6,
                _ => 1.,
            }
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|cue| *cue == self).unwrap()
    }

    fn wav(self) -> &'static [u8] {
        match self {
            Self::Focus => include_bytes!("../assets/sounds/focus.wav"),
            Self::Select => include_bytes!("../assets/sounds/select.wav"),
            Self::Back => include_bytes!("../assets/sounds/back.wav"),
            Self::Bump => include_bytes!("../assets/sounds/bump.wav"),
            Self::Glide => include_bytes!("../assets/sounds/glide.wav"),
            Self::RetroFocus => include_bytes!("../assets/sounds/retro-focus.wav"),
            Self::RetroSelect => include_bytes!("../assets/sounds/retro-select.wav"),
            Self::RetroBack => include_bytes!("../assets/sounds/retro-back.wav"),
            Self::RetroBump => include_bytes!("../assets/sounds/retro-bump.wav"),
            Self::Tracking => include_bytes!("../assets/sounds/retro-tracking.wav"),
        }
    }
}

/// Sample rate of the bundled clips.
pub const CLIP_RATE: i32 = 48_000;
/// The Retro TV background loop: mono at half the clip rate keeps the package small.
const MUSIC_WAV: &[u8] = include_bytes!("../assets/sounds/retro-music.wav");
const MUSIC_RATE: u32 = 24_000;
/// Music level against the clips' full scale: well under the clicks.
const MUSIC_GAIN: f32 = 0.14;
/// Seconds to fade the music in when Home appears, and out when it should stop.
const MUSIC_FADE_IN: f32 = 0.8;
const MUSIC_FADE_OUT: f32 = 0.25;
/// Overlapping voices while scrolling quickly; older ones are dropped beyond this.
const MAX_VOICES: usize = 6;

/// Decodes a 16-bit PCM mono WAV at 48 kHz into samples.
pub fn decode_wav(bytes: &[u8]) -> Result<Vec<i16>, String> {
    decode_wav_at(bytes, CLIP_RATE as u32)
}

/// Decodes a 16-bit PCM mono WAV at `rate` into samples.
fn decode_wav_at(bytes: &[u8], rate: u32) -> Result<Vec<i16>, String> {
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("Not a WAV file".into());
    }
    let mut offset = 12;
    let mut format = None;
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let body = offset + 8;
        let end = body.checked_add(size).filter(|end| *end <= bytes.len());
        let Some(end) = end else {
            return Err("Truncated WAV chunk".into());
        };
        match id {
            b"fmt " if size >= 16 => {
                let field =
                    |at: usize| u16::from_le_bytes([bytes[body + at], bytes[body + at + 1]]);
                let rate = u32::from_le_bytes(bytes[body + 4..body + 8].try_into().unwrap());
                format = Some((field(0), field(2), rate, field(14)));
            }
            b"data" => {
                if format != Some((1, 1, rate, 16)) {
                    return Err(format!("Expected 16-bit PCM mono {rate} Hz"));
                }
                return Ok(bytes[body..end]
                    .chunks_exact(2)
                    .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
                    .collect());
            }
            _ => {}
        }
        // Chunks are word aligned.
        offset = end + (size & 1);
    }
    Err("WAV has no data chunk".into())
}

struct Voice {
    clip: usize,
    position: f32,
}

/// The background loop's playback state, shared with the audio thread.
#[derive(Default)]
struct Music {
    /// Whether it should be heard; it fades towards this.
    on: bool,
    /// Position in loop samples; kept while paused so the music resumes where it stopped.
    position: f32,
    /// Current fade level, 0–1.
    level: f32,
}

struct Mixer {
    clips: Arc<[Vec<i16>]>,
    voices: Arc<Mutex<Vec<Voice>>>,
    /// Clip samples per output sample, for devices that do not open at 48 kHz.
    step: f32,
    music: Option<Arc<Vec<i16>>>,
    music_state: Arc<Mutex<Music>>,
    /// Loop samples per output sample.
    music_step: f32,
    /// Fade change per output sample, in and out.
    fade: (f32, f32),
}

impl Mixer {
    fn mix_music(&self, out: &mut [i16]) {
        let Some(music) = &self.music else {
            return;
        };
        let Ok(mut state) = self.music_state.lock() else {
            return;
        };
        if !state.on && state.level == 0. {
            return;
        }
        let length = music.len() as f32;
        for sample in out.iter_mut() {
            state.level = if state.on {
                (state.level + self.fade.0).min(1.)
            } else {
                (state.level - self.fade.1).max(0.)
            };
            if state.level == 0. {
                break;
            }
            // Linear interpolation upsamples the 24 kHz loop without stepping artefacts.
            let index = state.position as usize;
            let t = state.position.fract();
            let a = f32::from(music[index]);
            let b = f32::from(music[(index + 1) % music.len()]);
            let value = (a + (b - a) * t) * MUSIC_GAIN * state.level;
            *sample = sample.saturating_add(value as i16);
            state.position += self.music_step;
            if state.position >= length {
                state.position -= length;
            }
        }
    }
}

impl sdl2::audio::AudioCallback for Mixer {
    type Channel = i16;

    fn callback(&mut self, out: &mut [i16]) {
        out.fill(0);
        self.mix_music(out);
        let Ok(mut voices) = self.voices.lock() else {
            return;
        };
        voices.retain_mut(|voice| {
            let clip = &self.clips[voice.clip];
            for sample in out.iter_mut() {
                let index = voice.position as usize;
                let Some(value) = clip.get(index) else {
                    return false;
                };
                *sample = sample.saturating_add(*value);
                voice.position += self.step;
            }
            (voice.position as usize) < clip.len()
        });
    }
}

/// Plays cues on the default output; inert when audio is unavailable.
///
/// The output is opened once at startup and never reopened or resumed from the event loop:
/// 3.6.2 reopened it on foreground and audio-device events, and on TVs that froze the whole
/// launcher (no remote input) until it was downgraded. Keep it that way unless a change has
/// been verified on a TV first.
pub struct Sounds {
    device: Option<sdl2::audio::AudioDevice<Mixer>>,
    voices: Arc<Mutex<Vec<Voice>>>,
    clips: Option<Arc<[Vec<i16>]>>,
    music: Option<Arc<Vec<i16>>>,
    music_state: Arc<Mutex<Music>>,
}

impl Sounds {
    pub fn silent() -> Self {
        Self {
            device: None,
            voices: Arc::new(Mutex::new(Vec::new())),
            clips: None,
            music: None,
            music_state: Arc::new(Mutex::new(Music::default())),
        }
    }

    pub fn open(sdl: &sdl2::Sdl) -> Self {
        let mut sounds = Self::silent();
        let clips = match Cue::ALL
            .iter()
            .map(|cue| {
                decode_wav(cue.wav()).map(|samples| {
                    samples
                        .into_iter()
                        .map(|sample| (f32::from(sample) * cue.gain()) as i16)
                        .collect::<Vec<_>>()
                })
            })
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(clips) => clips,
            Err(error) => {
                eprintln!("Luma sounds: {error}");
                return sounds;
            }
        };
        sounds.clips = Some(clips.into());
        match decode_wav_at(MUSIC_WAV, MUSIC_RATE) {
            Ok(music) if !music.is_empty() => sounds.music = Some(Arc::new(music)),
            Ok(_) => {}
            Err(error) => eprintln!("Luma music: {error}"),
        }
        sounds.connect(sdl);
        sounds
    }

    /// Opens the default output once, at startup. On failure the player stays silent.
    fn connect(&mut self, sdl: &sdl2::Sdl) {
        let Some(clips) = self.clips.clone() else {
            return;
        };
        let audio = match sdl.audio() {
            Ok(audio) => audio,
            Err(error) => {
                eprintln!("Luma sounds: audio unavailable: {error}");
                return;
            }
        };
        let desired = sdl2::audio::AudioSpecDesired {
            freq: Some(CLIP_RATE),
            channels: Some(1),
            // About 10 ms per buffer keeps clicks in step with focus movement.
            samples: Some(512),
        };
        let voices = self.voices.clone();
        let music = self.music.clone();
        let music_state = self.music_state.clone();
        match audio.open_playback(None, &desired, |spec| {
            let freq = spec.freq.max(1) as f32;
            Mixer {
                clips,
                voices,
                step: CLIP_RATE as f32 / freq,
                music,
                music_state,
                music_step: MUSIC_RATE as f32 / freq,
                fade: (1. / (MUSIC_FADE_IN * freq), 1. / (MUSIC_FADE_OUT * freq)),
            }
        }) {
            Ok(device) => {
                device.resume();
                self.device = Some(device);
            }
            Err(error) => eprintln!("Luma sounds: could not open output: {error}"),
        }
    }

    /// Fades the background loop in or out; it resumes where it stopped.
    pub fn set_music(&self, on: bool) {
        if self.music.is_none() {
            return;
        }
        // Only a flag for the audio thread: never touch the device from the event loop.
        if let Ok(mut state) = self.music_state.lock() {
            state.on = on;
        }
    }

    pub fn play(&self, cue: Cue) {
        if self.device.is_none() {
            return;
        }
        if let Ok(mut voices) = self.voices.lock() {
            if voices.len() >= MAX_VOICES {
                voices.remove(0);
            }
            voices.push(Voice {
                clip: cue.index(),
                position: 0.,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bundled_clip_is_short_48k_mono_pcm() {
        for cue in Cue::ALL {
            let samples = decode_wav(cue.wav()).unwrap();
            assert!(!samples.is_empty(), "{cue:?}");
            // Navigation clicks must not linger: 200 ms, 250 ms for the retro blips, and
            // half a second for the tracking noise that covers the Home cut.
            let limit = match cue {
                Cue::Tracking => CLIP_RATE as usize / 2,
                Cue::RetroFocus | Cue::RetroSelect | Cue::RetroBack | Cue::RetroBump => {
                    CLIP_RATE as usize / 4
                }
                _ => CLIP_RATE as usize / 5,
            };
            assert!(samples.len() <= limit, "{cue:?}");
            assert!(
                samples.iter().any(|sample| sample.unsigned_abs() > 200),
                "{cue:?}"
            );
        }
    }

    #[test]
    fn the_glide_is_much_quieter_than_the_clicks() {
        let peak = |cue: Cue| {
            decode_wav(cue.wav())
                .unwrap()
                .iter()
                .map(|sample| (f32::from(*sample) * cue.gain()).abs())
                .fold(0., f32::max)
        };
        assert!(peak(Cue::Glide) < peak(Cue::Focus) * 0.6);
        // Every cue plays at a quarter of the level of its bundled clip.
        assert_eq!(Cue::Focus.gain(), 0.25);
    }

    #[test]
    fn malformed_or_unexpected_wavs_are_rejected() {
        assert!(decode_wav(b"nope").is_err());
        let mut bytes = Cue::Focus.wav().to_vec();
        // Claim 44.1 kHz in the fmt chunk.
        let fmt = bytes.windows(4).position(|w| w == b"fmt ").unwrap();
        bytes[fmt + 12..fmt + 16].copy_from_slice(&44_100u32.to_le_bytes());
        assert!(decode_wav(&bytes).is_err());
        let truncated = &Cue::Focus.wav()[..40];
        assert!(decode_wav(truncated).is_err());
    }

    #[test]
    fn the_mixer_sums_voices_and_drops_finished_ones() {
        use sdl2::audio::AudioCallback;
        let clips: Arc<[Vec<i16>]> = vec![
            vec![100i16; 4],
            vec![i16::MAX; 2],
            vec![1; 1],
            vec![1; 1],
            vec![1; 1],
        ]
        .into();
        let voices = Arc::new(Mutex::new(vec![
            Voice {
                clip: 0,
                position: 0.,
            },
            Voice {
                clip: 1,
                position: 0.,
            },
        ]));
        let mut mixer = Mixer {
            clips,
            voices: voices.clone(),
            step: 1.,
            music: None,
            music_state: Arc::new(Mutex::new(Music::default())),
            music_step: 1.,
            fade: (1., 1.),
        };
        let mut out = [0i16; 3];
        mixer.callback(&mut out);
        // Saturates instead of wrapping, and the 2-sample clip ends after its data.
        assert_eq!(out, [i16::MAX, i16::MAX, 100]);
        assert_eq!(voices.lock().unwrap().len(), 1);
        mixer.callback(&mut out);
        assert_eq!(out, [100, 0, 0]);
        assert!(voices.lock().unwrap().is_empty());
        // A silent player ignores cues.
        Sounds::silent().play(Cue::Bump);
        Sounds::silent().set_music(true);
    }

    #[test]
    fn retro_clicks_replace_only_the_navigation_cues() {
        assert_eq!(Cue::Focus.retro(), Cue::RetroFocus);
        assert_eq!(Cue::Select.retro(), Cue::RetroSelect);
        assert_eq!(Cue::Back.retro(), Cue::RetroBack);
        assert_eq!(Cue::Bump.retro(), Cue::RetroBump);
        assert_eq!(Cue::Glide.retro(), Cue::Glide);
        assert_eq!(Cue::Tracking.retro(), Cue::Tracking);
    }

    #[test]
    fn the_music_loop_is_a_quiet_24k_mono_minute() {
        let music = decode_wav_at(MUSIC_WAV, MUSIC_RATE).unwrap();
        let seconds = music.len() as f32 / MUSIC_RATE as f32;
        assert!((40. ..=70.).contains(&seconds), "{seconds}");
        assert!(music.iter().any(|sample| sample.unsigned_abs() > 10_000));
        assert!(MUSIC_GAIN < Cue::Focus.gain());
        // The loop seam: the last sample leads smoothly into the first.
        let jump = (i32::from(music[0]) - i32::from(*music.last().unwrap())).abs();
        assert!(jump < 4_000, "{jump}");
    }

    #[test]
    fn music_fades_in_loops_and_fades_out_keeping_its_place() {
        use sdl2::audio::AudioCallback;
        let state = Arc::new(Mutex::new(Music::default()));
        let mut mixer = Mixer {
            clips: vec![vec![0i16; 1]].into(),
            voices: Arc::new(Mutex::new(Vec::new())),
            step: 1.,
            music: Some(Arc::new(vec![10_000i16; 4])),
            music_state: state.clone(),
            music_step: 1.,
            fade: (0.5, 0.5),
        };
        let mut out = [0i16; 6];
        mixer.callback(&mut out);
        assert_eq!(out, [0; 6], "off by default");
        state.lock().unwrap().on = true;
        mixer.callback(&mut out);
        let full = (10_000. * MUSIC_GAIN) as i16;
        assert_eq!(out[0], (10_000. * MUSIC_GAIN * 0.5) as i16);
        assert_eq!(out[1..], [full; 5]);
        // Six samples through a four-sample loop wrap to position 2.
        assert_eq!(state.lock().unwrap().position, 2.);
        state.lock().unwrap().on = false;
        mixer.callback(&mut out);
        assert_eq!(out[0], (10_000. * MUSIC_GAIN * 0.5) as i16);
        assert_eq!(out[1..], [0; 5]);
        assert_eq!(state.lock().unwrap().position, 3.);
    }
}
