//! Short UI click sounds. Clips are embedded, decoded once at startup and mixed in SDL's
//! audio callback thread, so playing a cue only pushes a voice under a short lock and never
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
}

impl Cue {
    const ALL: [Self; 5] = [
        Self::Focus,
        Self::Select,
        Self::Back,
        Self::Bump,
        Self::Glide,
    ];

    /// Mix level: all cues at a quarter of the clips' level, the glide far below the clicks.
    fn gain(self) -> f32 {
        let master = 0.25;
        master
            * match self {
                Self::Glide => 0.3,
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
        }
    }
}

/// Sample rate of the bundled clips.
pub const CLIP_RATE: i32 = 48_000;
/// Overlapping voices while scrolling quickly; older ones are dropped beyond this.
const MAX_VOICES: usize = 6;

/// Decodes a 16-bit PCM mono WAV at 48 kHz into samples.
pub fn decode_wav(bytes: &[u8]) -> Result<Vec<i16>, String> {
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
                if format != Some((1, 1, CLIP_RATE as u32, 16)) {
                    return Err("Expected 16-bit PCM mono 48 kHz".into());
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

struct Mixer {
    clips: Arc<[Vec<i16>; 5]>,
    voices: Arc<Mutex<Vec<Voice>>>,
    /// Clip samples per output sample, for devices that do not open at 48 kHz.
    step: f32,
}

impl sdl2::audio::AudioCallback for Mixer {
    type Channel = i16;

    fn callback(&mut self, out: &mut [i16]) {
        out.fill(0);
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
/// The output can disappear under the launcher: another app (e.g. a streaming app) takes
/// the TV's audio while Luma is in the background, or the user switches to a soundbar. The
/// event loop therefore calls `reconnect` when Luma returns to the foreground or SDL reports
/// an audio device change, which reopens the default output with the already decoded clips.
pub struct Sounds {
    device: Option<sdl2::audio::AudioDevice<Mixer>>,
    voices: Arc<Mutex<Vec<Voice>>>,
    clips: Option<Arc<[Vec<i16>; 5]>>,
}

impl Sounds {
    pub fn silent() -> Self {
        Self {
            device: None,
            voices: Arc::new(Mutex::new(Vec::new())),
            clips: None,
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
        sounds.clips = Some(Arc::new(clips.try_into().unwrap()));
        sounds.reconnect(sdl);
        sounds
    }

    /// (Re)opens the current default output. Without decoded clips (silent player) it does
    /// nothing; on failure the player stays silent until the next reconnect.
    pub fn reconnect(&mut self, sdl: &sdl2::Sdl) {
        let Some(clips) = self.clips.clone() else {
            return;
        };
        // Close the old stream first: it may point at an output that no longer exists.
        self.device = None;
        if let Ok(mut voices) = self.voices.lock() {
            voices.clear();
        }
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
        match audio.open_playback(None, &desired, |spec| Mixer {
            clips,
            voices,
            step: CLIP_RATE as f32 / spec.freq.max(1) as f32,
        }) {
            Ok(device) => {
                device.resume();
                self.device = Some(device);
            }
            Err(error) => eprintln!("Luma sounds: could not open output: {error}"),
        }
    }

    pub fn play(&self, cue: Cue) {
        let Some(device) = &self.device else {
            return;
        };
        // A stream the system paused (e.g. while another app had the audio) is resumed.
        if device.status() != sdl2::audio::AudioStatus::Playing {
            device.resume();
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
            // At most 200 ms: navigation clicks must not linger.
            assert!(samples.len() <= CLIP_RATE as usize / 5, "{cue:?}");
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
        let clips = Arc::new([
            vec![100i16; 4],
            vec![i16::MAX; 2],
            vec![1; 1],
            vec![1; 1],
            vec![1; 1],
        ]);
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
    }
}
