use std::{
    thread::{JoinHandle, spawn},
    time::{Duration, Instant},
};

use rdev::{Button, Event, EventType, Key};

pub struct HotkeyManager {
    pub hotkey: Key,
    last_hotkey_time: Instant,
    _thread: Option<JoinHandle<()>>,
}

impl HotkeyManager {
    const DEBOUNCE_THRESHOLD: Duration = Duration::from_millis(500);
    pub fn with_key(hotkey: Key) -> Self {
        Self {
            hotkey,
            last_hotkey_time: Instant::now(),
            _thread: None,
        }
    }

    pub fn register_callback(
        &mut self,
        callback: impl Fn(rdev::Event) -> Option<rdev::Event> + Send + 'static,
    ) {
        let hotkey = self.hotkey;
        let handler = move |event: Event| {
            if let EventType::KeyPress(key) = event.event_type
                && key == hotkey
            {
                println!("callback called!");
                callback(event)
            } else {
                Some(event)
            }
        };

        spawn(move || {
            if let Err(error) = rdev::grab(handler) {
                eprintln!("Keyboard input error!:\n\t{:?}", error);
            }
        });
    }
}

#[test]
fn test_listen_key_input() {
    spawn_test_key_listener().join().unwrap();
}

#[allow(unused)]
pub fn spawn_test_key_listener() -> JoinHandle<()> {
    let callback: fn(Event) -> () = |event| {
        dbg!(event.event_type);
        dbg!(event.name);
    };
    spawn(move || {
        if let Err(error) = rdev::listen(callback) {
            eprintln!("Keyboard input error!:\n\t{:?}", error);
        }
    })
}
