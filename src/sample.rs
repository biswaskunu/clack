use hound::WavReader;
use std::fs::File;

pub struct ClickSample {
    pub data: Vec<f32>,
    pub sample_rate: u32,
}

pub fn load_sample() -> anyhow::Result<ClickSample> {
    let mut reader = WavReader::new(File::open("assets/press.wav"))?;
    let sample_rate = reader.sample_rate();
    let channels = reader.channels();
    let mut data = Vec::new();

    for sample in reader.samples::<i16>() {
        let s = sample? as f32 / i16::MAX as f32;
        data.push(s);
    }

    // If stereo, convert to mono by averaging channels
    let mono_data: Vec<f32> = if channels == 2 {
        let mut mono = Vec::new();
        let mut iter = data.chunks_exact(2);
        for chunk in &mut iter {
            mono.push((chunk[0] + chunk[1]) / 2.0);
        }
        mono
    } else {
        data
    };

    Ok(ClickSample {
        data: mono_data,
        sample_rate,
    })
}