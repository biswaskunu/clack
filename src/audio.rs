use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, StreamConfig,
};
use mixer::Mixer;
use crossbeam_channel::Receiver;
use anyhow::Error;

pub struct AudioPlayer {
    mixer: Mixer,
    rx: Receiver<evdev::InputEvent>,
}

impl AudioPlayer {
    pub fn new(gain: f32, rx: Receiver<evdev::InputEvent>) -> Result<(), Error> {
        let mixer = Mixer::new(gain);
        Ok(AudioPlayer { mixer, rx })
    }

    pub fn run(self) -> Result<(), Error> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("no output device available");
        let config = device.default_output_config().unwrap().into();

        let stream = device
            .build_output_stream::<f32, _, _>(
                config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    Self::callback(data, &mut self.mixer, &self.rx);
                },
                |err| eprintln!("clack: audio stream error: {err}"),
                None,
            )
            .expect("failed to build output stream");
        stream.play()?;

        // Keep thread alive - the output stream callback
        // is called by cpal at the device's sampling rate
        std::thread::sleep(std::time::Duration::from_secs(3600));

        Ok(())
    }

    fn callback(
        data: &mut [f32],
        mixer: &mut Mixer,
        rx: &crossbeam_channel::Receiver<evdev::InputEvent>,
    ) {
        // Drain queue of pending key presses and trigger voices
        while let Ok(_event) = rx.try_recv() {
            // Trigger a voice for each pending key press
            let _ = mixer.trigger();
        }

        // Render audio: mix all active voices into output buffer
        // The mixer handles zeroing, summing voices, and clamping
        mixer.render(data);
    }
}