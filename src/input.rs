use evdev::{Device, InputEvent, KeyCode, enumerate};

pub struct InputHandler;

impl InputHandler {
    pub fn list_keyboards() -> Vec<Device> {
        let devices = enumerate().collect::<Vec<_>>();
        let mut keyboards = Vec::new();

        for (path, raw_dev) in devices {
            // Check if device supports key events and has KEY_A
            if let Some(keys) = raw_dev.supported_keys() {
                if keys.contains(KeyCode::KEY_A) {
                    // Open the device and add to keyboards
                    match Device::open(&path) {
                        Ok(device) => keyboards.push(device),
                        Err(_) => continue,
                    }
                }
            }
        }
        keyboards
    }

    pub fn start_reading(keyboards: Vec<Device>, tx: crossbeam_channel::Sender<InputEvent>) {
        for mut keyboard in keyboards {
            let tx = tx.clone();
            std::thread::spawn(move || {
                // Read events directly from the device
                if let Ok(mut events) = keyboard.fetch_events() {
                    for event in &mut events {
                        // value 1 = press, value 0 = release, value 2 = auto-repeat
                        // Only handle press events
                        if event.value() == 1 {
                            let _ = tx.try_send(event);
                        }
                    }
                }
            });
        }
    }
}