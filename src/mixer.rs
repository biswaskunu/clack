use std::sync::atomic::{AtomicU64, Ordering};

const MAX_VOICES: usize = 32;

#[derive(Clone, Copy)]
struct Voice {
    pos: usize,
    active: bool,
    started: u64,
}

#[derive(Clone)]
pub struct Mixer {
    sample: Vec<f32>,
    voices: [Voice; MAX_VOICES],
    gain: f32,
    seq: u64,
}

impl Mixer {
    pub fn new(gain: f32) -> Self {
        let sample = Vec::new();
        let voices = [Voice {
            pos: 0,
            active: false,
            started: 0,
        }; MAX_VOICES];
        Mixer {
            sample,
            voices,
            gain,
            seq: 0,
        }
    }

    pub fn set_sample(&mut self, sample: Vec<f32>) {
        self.sample = sample;
    }

    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain;
    }

    /// Trigger a new voice, stealing the oldest if pool is full.
    /// Returns the index of the voice triggered.
    pub fn trigger(&mut self) -> usize {
        self.seq += 1;
        let slot = self
            .voices
            .iter()
            .position(|v| !v.active)
            .unwrap_or_else(|| {
                let oldest = self
                    .voices
                    .iter()
                    .min_by_key(|v| v.started)
                    .map(|v| v.started)
                    .unwrap();
                self.voices
                    .iter()
                    .position(|v| v.started == oldest)
                    .map(|i| {
                        self.voices[i].started = self.seq;
                        i
                    })
                    .unwrap_or(0)
            });

        self.voices[slot].active = true;
        self.voices[slot].pos = 0;
        self.voices[slot].started = self.seq;
        slot
    }

    /// Render one buffer: mix all active voices into out, advance positions,
    /// deactivate finished voices, and clamp output to [-1, 1].
    pub fn render(&mut self, out: &mut [f32]) {
        // Zero the output buffer
        for sample in out.iter_mut() {
            *sample = 0.0;
        }

        // Sum all active voices into the buffer
        for voice in self.voices.iter_mut() {
            if voice.active && voice.pos < self.sample.len() {
                let val = self.sample[voice.pos] * self.gain;
                // Add to each sample in the output buffer
                for sample_idx in 0..out.len() {
                    out[sample_idx] += val;
                }
                voice.pos += 1;
                if voice.pos >= self.sample.len() {
                    voice.active = false;
                }
            }
        }

        // Clip to [-1, 1]
        for sample in out.iter_mut() {
            if *sample > 1.0 {
                *sample = 1.0;
            } else if *sample < -1.0 {
                *sample = -1.0;
            }
        }
    }
}