use hound::{SampleFormat, WavReader};
use std::io::Cursor;

// Bundled into the binary, so there's no runtime file path to get wrong.
static PRESS_WAV: &[u8] = include_bytes!("../assets/press.wav");

/// Decode the click, mix to mono, and resample to `target_rate`.
/// Runs once at startup, never on the audio thread.
pub fn load_sample(target_rate: u32) -> anyhow::Result<Vec<f32>> {
    let mut reader = WavReader::new(Cursor::new(PRESS_WAV))?;
    let spec = reader.spec();
    let channels = spec.channels as usize;

    let interleaved: Vec<f32> = match spec.sample_format {
        SampleFormat::Float => reader.samples::<f32>().collect::<Result<_, _>>()?,
        SampleFormat::Int => {
            let max = (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.map(|v| v as f32 / max))
                .collect::<Result<_, _>>()?
        }
    };

    // Average channels down to mono.
    let mono: Vec<f32> = interleaved
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect();

    Ok(resample_linear(&mono, spec.sample_rate, target_rate))
}

fn resample_linear(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let ratio = to as f64 / from as f64;
    let out_len = (input.len() as f64 * ratio).round() as usize;
    let last = input.len() - 1;

    (0..out_len)
        .map(|i| {
            let pos = i as f64 / ratio;
            let idx = (pos.floor() as usize).min(last);
            let frac = (pos - idx as f64) as f32;
            let a = input[idx];
            let b = input[(idx + 1).min(last)];
            a + (b - a) * frac
        })
        .collect()
}
