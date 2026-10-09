const MAX_VOICES: usize = 32;

#[derive(Clone, Copy)]
struct Voice {
    pos: usize,
    active: bool,
    started: u64,
}

/// Pure mixing logic: no I/O, no allocation after `set_sample`.
#[derive(Clone)]
pub struct Mixer {
    sample: Vec<f32>,
    voices: [Voice; MAX_VOICES],
    gain: f32,
    seq: u64,
}

impl Mixer {
    pub fn new(gain: f32) -> Self {
        Mixer {
            sample: Vec::new(),
            voices: [Voice {
                pos: 0,
                active: false,
                started: 0,
            }; MAX_VOICES],
            gain,
            seq: 0,
        }
    }

    pub fn set_sample(&mut self, sample: Vec<f32>) {
        self.sample = sample;
    }

    #[allow(dead_code)]
    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain;
    }

    /// Start a new voice, stealing the oldest if the pool is full.
    /// Returns the slot used, or None if there is no sample loaded.
    pub fn trigger(&mut self) -> Option<usize> {
        if self.sample.is_empty() {
            return None;
        }
        self.seq += 1;

        let slot = match self.voices.iter().position(|v| !v.active) {
            Some(i) => i,
            None => self
                .voices
                .iter()
                .enumerate()
                .min_by_key(|(_, v)| v.started)
                .map(|(i, _)| i)
                .unwrap_or(0),
        };

        self.voices[slot] = Voice {
            pos: 0,
            active: true,
            started: self.seq,
        };
        Some(slot)
    }

    /// Fill `out` (interleaved, `channels` per frame) with the mix of all voices.
    /// Each voice advances one sample per *frame*, not per buffer.
    pub fn render(&mut self, out: &mut [f32], channels: usize) {
        let channels = channels.max(1);
        let sample = &self.sample;
        let voices = &mut self.voices;
        let gain = self.gain;

        for frame in out.chunks_mut(channels) {
            let mut acc = 0.0f32;
            for voice in voices.iter_mut() {
                if voice.active {
                    acc += sample[voice.pos];
                    voice.pos += 1;
                    if voice.pos >= sample.len() {
                        voice.active = false;
                    }
                }
            }
            let s = (acc * gain).clamp(-1.0, 1.0);
            frame.fill(s); // same mono sample on every channel
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mixer_with(sample: Vec<f32>, gain: f32) -> Mixer {
        let mut m = Mixer::new(gain);
        m.set_sample(sample);
        m
    }

    #[test]
    fn no_sample_means_no_voice() {
        let mut m = Mixer::new(1.0);
        assert!(m.trigger().is_none());
    }

    #[test]
    fn plays_sample_across_frames_and_channels() {
        let mut m = mixer_with(vec![0.1, 0.2, 0.3], 1.0);
        m.trigger();
        let mut out = [0.0f32; 8]; // 4 frames x 2 channels
        m.render(&mut out, 2);
        assert_eq!(out, [0.1, 0.1, 0.2, 0.2, 0.3, 0.3, 0.0, 0.0]);
    }

    #[test]
    fn gain_applies() {
        let mut m = mixer_with(vec![0.5], 0.5);
        m.trigger();
        let mut out = [0.0f32; 2];
        m.render(&mut out, 1);
        assert_eq!(out[0], 0.25);
    }

    #[test]
    fn clips_to_range() {
        let mut m = mixer_with(vec![0.9; 4], 1.0);
        m.trigger();
        m.trigger();
        let mut out = [0.0f32; 1];
        m.render(&mut out, 1);
        assert_eq!(out[0], 1.0);
    }

    #[test]
    fn steals_oldest_when_full() {
        let mut m = mixer_with(vec![0.1; 1000], 1.0);
        for _ in 0..MAX_VOICES {
            m.trigger();
        }
        // slot 0 was started first, so it is the oldest
        assert_eq!(m.trigger(), Some(0));
        assert_eq!(m.trigger(), Some(1));
    }

    #[test]
    fn voice_finishes() {
        let mut m = mixer_with(vec![0.1, 0.1], 1.0);
        m.trigger();
        let mut out = [0.0f32; 4];
        m.render(&mut out, 1);
        assert!(m.voices.iter().all(|v| !v.active));
    }
}
