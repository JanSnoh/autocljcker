use crate::key_input::HotkeyManager;
use rdev::{Button, EventType, Key};
use std::{
    sync::{
        Arc, Mutex, atomic::{AtomicBool, Ordering}, mpsc::{self, Receiver, TryRecvError},
    }, thread::{self, JoinHandle}, time::Duration,
};
use tokio::{time::{self, Interval}};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoclickerSettings {
    pub interval: Duration,
    pub button: Button,
}

impl Default for AutoclickerSettings {
    fn default() -> Self {
        Self {
            interval: Duration::from_millis(500),
            button: Button::Left,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickerCommand {
    Start,
    Pause,
    Toggle,
    OverwriteDuration(Duration),
    Terminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickerState {
    Running,
    Paused,
}
impl ClickerState{
    pub fn should_pause(&self, command: ClickerCommand) -> bool{
        matches![command, ClickerCommand::Toggle if *self == Self::Running] || command == ClickerCommand::Pause
    }
    pub fn should_resume(&self, command: ClickerCommand) -> bool{
        matches![command, ClickerCommand::Toggle if *self == Self::Paused] || command == ClickerCommand::Start
    }
}

pub struct Autoclicker {
    command_channel: mpsc::Sender<ClickerCommand>,
    state: Arc<Mutex<ClickerState>>,
    thread: Option<JoinHandle<()>>,
    _hotkey_manager: HotkeyManager,
}

impl Autoclicker {
    pub fn new(settings: AutoclickerSettings, hotkey: Key) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let state: Arc<Mutex<ClickerState>> = Arc::new(Mutex::new(ClickerState::Paused));
        // Copy of the active flag for moving into the closure
        let _state = state.clone();
        let thread = spawn_clicker_thread(settings, rx, _state);
        let mut _hotkey_manager = HotkeyManager::with_key(hotkey);
        {
            let sender = tx.clone();
            _hotkey_manager.register_callback(move |_| {
                let _ = sender.send(ClickerCommand::Toggle);
                None
            });
        }
        Self {
            command_channel: tx,
            state,
            thread: Some(thread),
            _hotkey_manager,
        }
    }

    pub fn click(button: Button) {
        println!("clicking!");
        rdev::simulate(&rdev::EventType::ButtonPress(button)).unwrap();
        rdev::simulate(&rdev::EventType::ButtonRelease(button)).unwrap();
    }
    pub fn get_state(&self) -> Result<ClickerState, std::sync::PoisonError<std::sync::MutexGuard<'_, ClickerState>>>   {
        Ok(*self.state.lock()?)
    }
    pub fn is_running(&self) -> bool {
        match &self.thread {
            Some(handle) => {
                !handle.is_finished() && *self.state.lock().unwrap() == ClickerState::Running
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

fn spawn_clicker_thread(settings:AutoclickerSettings, rx: Receiver<ClickerCommand>, state: Arc<Mutex<ClickerState>>) -> JoinHandle<()>{
    thread::spawn(move || {
        let AutoclickerSettings { mut interval, button } = settings;
        let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    eprintln!("clicker started");
    rt.block_on(async {
            let mut timer = time::interval(interval);
            loop {
                match rx.try_recv() {
                    Err(TryRecvError::Disconnected) | Ok(ClickerCommand::Terminate) => return (),
                    Ok(ClickerCommand::OverwriteDuration(new_duration)) => {
                        interval = new_duration;
                        timer = time::interval(new_duration)
                    }, // overwrite the current timer
                    Ok(command @ (ClickerCommand::Pause | ClickerCommand::Start | ClickerCommand::Toggle)) => {
                        let mut state_lock = state.lock().unwrap();
                        let prev_state = *state_lock;
                        if prev_state.should_pause(command){
                            *state_lock = ClickerState::Paused;
                        } else if prev_state.should_resume(command){
                            *state_lock = ClickerState::Running;
                        }
                    },
                    Err(TryRecvError::Empty) => (),
                }
                match *state.lock().unwrap(){
                    ClickerState::Paused => continue,
                    ClickerState::Running => (),
                }
                
                timer.tick().await;
                Autoclicker::click(button);
            }
        });
    })
}