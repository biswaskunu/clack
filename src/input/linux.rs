use crossbeam_channel::Sender;
use evdev::{Device, EventType, KeyCode, enumerate};

pub struct InputHandler;

impl InputHandler {
    /// `enumerate()` already yields opened devices (and silently skips ones
    /// we lack permission for), so no second `Device::open` is needed.
    pub fn list_keyboards() -> Vec<Device> {
        enumerate()
            .map(|(_path, dev)| dev)
            .filter(|dev| {
                dev.supported_keys()
                    .is_some_and(|keys| keys.contains(KeyCode::KEY_A))
            })
            .collect()
    }

    pub fn start_reading(keyboards: Vec<Device>, tx: Sender<()>) {
        for mut keyboard in keyboards {
            let tx = tx.clone();
            std::thread::spawn(move || {
                // fetch_events() returns one batch and must be called repeatedly.
                loop {
                    match keyboard.fetch_events() {
                        Ok(events) => {
                            for event in events {
                                // value 1 = press, 0 = release, 2 = auto-repeat
                                if event.event_type() == EventType::KEY && event.value() == 1 {
                                    // Drop the click rather than block if the queue is full.
                                    let _ = tx.try_send(());
                                }
                            }
                        }
                        // Keyboard unplugged or read error: this thread quietly ends.
                        Err(_) => break,
                    }
                }
            });
        }
    }
}
