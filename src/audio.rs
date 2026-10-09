use anyhow::{Context, Result, anyhow, bail};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};
use crossbeam_channel::Receiver;

use crate::mixer::Mixer;
use crate::sample;

/// Open the default output device, load the click at the device's sample rate,
/// and start playing. The returned `Stream` must be kept alive by the caller.
pub fn start(gain: f32, rx: Receiver<()>) -> Result<Stream> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow!("no audio output device found.\n  Check your sound settings (PipeWire / PulseAudio / ALSA) and try again."))?;

    let supported = device
        .default_output_config()
        .context("could not read the audio device's default config")?;
    if supported.sample_format() != SampleFormat::F32 {
        bail!(
            "audio device uses {:?} samples; only f32 is supported for now",
            supported.sample_format()
        );
    }

    let config: cpal::StreamConfig = supported.into();
    let channels = config.channels as usize;
    let rate = config.sample_rate.0;

    // All decoding / resampling happens here, before the callback exists.
    let mut mixer = Mixer::new(gain);
    mixer.set_sample(sample::load_sample(rate)?);

    let click = sample::load_sample(rate)?;
    let peak = click.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    eprintln!(
        "[debug] device: {:?}, rate {rate}, channels {channels}, sample len {}, peak {peak}",
        device.name().ok(),
        click.len()
    );
    mixer.set_sample(click);
    
    let stream = device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // Real-time rules: no locks, no allocation, no printing here.
                while rx.try_recv().is_ok() {
                    mixer.trigger();
                }
                mixer.render(data, channels);
            },
            |err| eprintln!("clack: audio stream error: {err}"),
            None,
        )
        .context("failed to build audio output stream")?;

    stream.play().context("failed to start audio stream")?;
    Ok(stream)
}
