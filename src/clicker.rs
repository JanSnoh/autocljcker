use enigo::{self, Direction, Mouse, Settings};
use rdev::{Button, EventType, Key};
use crate::key_input::HotkeyManager;
use std::{
    sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc::TryRecvError}, thread::{self, JoinHandle, Thread}, time::Duration,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoclickerSettings {
    pub interval: Duration,
    pub button: Button,
}

impl Default for AutoclickerSettings{
    fn default() -> Self {
        Self { interval: Default::default(), button: Button::Left }
    }
}

pub struct Autoclicker {
    channel: std::sync::mpsc::Sender<()>,
    thread: Option<JoinHandle<()>>,
    active: Arc<AtomicBool>,
    hotkey_manager: HotkeyManager,
}

impl Autoclicker {
    pub fn new(settings: AutoclickerSettings, hotkey: Key) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let active: Arc<AtomicBool> = Arc::new(true.into());
        // Copy of the active flag for moving into the closure
        let _active = active.clone();
        let thread = thread::spawn(move || {
            let AutoclickerSettings { interval, button } = settings;
            let mut timer = systime::tokio::ClockType::IgnoreSleep
                .interval(interval)
                .unwrap();
            let rt = tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap();
            eprintln!("clicker started");
            rt.block_on(async {
                while let Err(TryRecvError::Empty) = rx.try_recv() {
                    timer.tick().await.unwrap();
                    // If the ticker is inactive we can sleep until we are woken up
                    if !_active.load(Ordering::Relaxed){
                            thread::park();
                    }
                    Self::click(settings.button);
                }
            });
        });
        let mut hotkey_manager = HotkeyManager::with_key(hotkey);
        {
            let active_flag_ref = active.clone();
            hotkey_manager.register_callback(Box::new(move |event| {
                if event.event_type == EventType::KeyPress(hotkey) {
                    active_flag_ref.fetch_xor(true, Ordering::Relaxed);
                }
            }));
        }
        Self {
            channel: tx,
            thread: Some(thread),
            active,
            hotkey_manager
        }
    }

    pub fn click(button: Button){
        rdev::simulate(&rdev::EventType::ButtonPress(button)).unwrap();
        rdev::simulate(&rdev::EventType::ButtonRelease(button)).unwrap();
    }
    pub fn is_running(&self) -> bool {
        match &self.thread {
            Some(handle) => !handle.is_finished() && self.active.load(Ordering::Relaxed),
            None => false,
        }
    }
    pub fn set_active(&mut self){
        if let Some(thread) = &self.thread{
            thread.thread().unpark();
        }
        self.active.store(true.into(), Ordering::Relaxed);
    }
    pub fn set_inactive(&mut self){
        self.active.store(false.into(), Ordering::Relaxed);
    }
}

impl Drop for Autoclicker {
    fn drop(&mut self) {
        self.channel
            .send(())
            .expect("Autoclicker Thread has paniced!");
        self.set_active();
        // join the thread and panic if joining failed.
        self.thread
            .take()
            .map(JoinHandle::join)
            .transpose()
            .unwrap();
    }
}
