use evdev::{Device, Key, KeyState, list_devices};
use crossbeam_queue::Sender;

pub struct InputHandler;

impl InputHandler {
    pub fn list_keyboards() -> Vec<Device> {
        let devices = list_devices().unwrap_or_default();
        let mut keyboards = Vec::new();

        for dev_info in &devices {
            let device = Device::new(dev_info).expect("Failed to open device");
            if device.capabilities().contains(evdev::InputEventType::KEY) {
                let keys: Vec<Key> = device.key_keys().collect();
                if keys.contains(&Key::KeyA) {
                    keyboards.push(device);
                }
            }
        }
        keyboards
    }

    pub fn start_reading(keyboards: Vec<Device>, tx: Sender<evdev::InputEvent>) {
        for keyboard in keyboards {
            let tx = tx.clone();
            std::thread::spawn(move || {
                if let Err(e) = keyboard.read_events(|event| {
                    if event.value() == KeyState::Pressed {
                        let _ = tx.try_send(event.clone());
                    }
                }) {
                    eprintln!("clack: error reading events: {}", e);
                }
            });
        }
    }
}