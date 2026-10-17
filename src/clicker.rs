use crate::key_input::HotkeyManager;
use rdev::{Button, EventType, Key};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, TryRecvError},
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoclickerSettings {
    pub interval: Duration,
    pub button: Button,
}

impl Default for AutoclickerSettings {
    fn default() -> Self {
        Self {
            interval: Default::default(),
            button: Button::Left,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickerCommand {
    Start,
    Pause,
    Toggle,
    OverwriteSettings(AutoclickerSettings),
    Terminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickerState {
    Running,
    Paused,
}

pub struct Autoclicker {
    command_channel: mpsc::Sender<ClickerCommand>,
    state: Arc<Mutex<ClickerState>>,
    thread: Option<JoinHandle<()>>,
    hotkey_manager: HotkeyManager,
}

impl Autoclicker {
    pub fn new(settings: AutoclickerSettings, hotkey: Key) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let state: Arc<Mutex<ClickerState>> = Arc::new(Mutex::new(ClickerState::Paused));
        // Copy of the active flag for moving into the closure
        let _state = state.clone();
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
                loop {
                    match rx.try_recv() {
                        Err(TryRecvError::Disconnected) | Ok(ClickerCommand::Terminate) => break,
                        Ok(ClickerCommand::OverwriteSettings(new_settings)) => (), // overwrite the current timer
                        Ok(ClickerCommand::Pause) => {
                            *_state.lock().await = ClickerState::Paused;
                        }
                        Ok(ClickerCommand::Start) => {
                            *_state.lock().await = ClickerState::Paused;
                        }
                        _ => (),
                    }
                    timer.tick().await.unwrap();
                    Self::click(button);
                }
            });
        });
        let mut hotkey_manager = HotkeyManager::with_key(hotkey);
        {
            let sender = tx.clone();
            hotkey_manager.register_callback(Box::new(move |event| {
                let _ = sender.send(ClickerCommand::Toggle);
                None
            }));
        }
        Self {
            command_channel: tx,
            state,
            thread: Some(thread),
            hotkey_manager,
        }
    }

    pub fn click(button: Button) {
        rdev::simulate(&rdev::EventType::ButtonPress(button)).unwrap();
        rdev::simulate(&rdev::EventType::ButtonRelease(button)).unwrap();
    }
    pub fn is_running(&self) -> bool {
        match &self.thread {
            Some(handle) => {
                !handle.is_finished() && *self.state.blocking_lock() == ClickerState::Running
            }
            None => false,
        }
    }
    pub fn set_active(&self) {
        if let Some(thread) = &self.thread {
            thread.thread().unpark();
        }
        let _ = self.command_channel.send(ClickerCommand::Start);
    }
    pub fn set_inactive(&self) {
        let _ = self.command_channel.send(ClickerCommand::Pause);
    }
}

impl Drop for Autoclicker {
    fn drop(&mut self) {
        self.command_channel
            .send(ClickerCommand::Terminate)
            .expect("Autoclicker Thread has paniced!");
        self.set_active();
        eprintln!("running clicker destructor");
        // join the thread and panic if joining failed.
        self.thread
            .take()
            .map(JoinHandle::join)
            .transpose()
            .unwrap();
        eprintln!("running clicker destructor");
    }
}
