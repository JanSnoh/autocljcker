use std::{
    default, sync::{Mutex, atomic::AtomicBool}, thread::{self, JoinHandle, Thread, spawn}, time::{Duration, Instant},
};

use eframe::{
    egui::{self, Color32, RichText},
};
use rdev::{Button, Key};

pub struct HotkeyManager {
    pub hotkey: Key,
    last_hotkey_time: Instant,
    active: AtomicBool,
    thread: Option<JoinHandle<()>>,
}

impl HotkeyManager {
    const DEBOUNCE_THRESHOLD: Duration = Duration::from_millis(500);
    pub fn with_key(hotkey: Key) -> Self {
        Self {
            hotkey,
            last_hotkey_time: Instant::now(),
            active: false.into(),
            thread: None,
        }
    }

    pub fn register_callback(&mut self, callback: Box<dyn Fn(rdev::Event)->() + Send>){
        spawn(move ||{
            if let Err(error) = rdev::listen(callback) {
                eprintln!("Keyboard input error!:\n\t{:?}", error);
            }
        });
    }
}
