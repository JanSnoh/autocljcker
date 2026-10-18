use crate::key_input::HotkeyManager;
use rdev::{Button, EventType, Key};
use std::{
    sync::{
        Arc,
    }, thread::{self, JoinHandle}, time::Duration,
};
use tokio::{sync::{Mutex, mpsc}, time::{self, error::Elapsed, timeout}};

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
        let (tx, rx) = mpsc::channel(15);
        let state: Arc<Mutex<ClickerState>> = Arc::new(Mutex::new(ClickerState::Paused));
        // Copy of the active flag for moving into the closure
        let _state = state.clone();
        let thread = spawn_clicker_thread(settings, rx, _state);
        let mut _hotkey_manager = HotkeyManager::with_key(hotkey);
        {
            let sender = tx.clone();
            _hotkey_manager.register_callback(move |_| {
                let _ = sender.blocking_send(ClickerCommand::Toggle);
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
        Ok(*self.state.blocking_lock())
    }
    pub fn is_running(&self) -> bool {
        match &self.thread {
            Some(handle) => {
                !handle.is_finished() && self.get_state().unwrap() == ClickerState::Running
            }
            None => false,
        }
    }
    pub fn start(&self) {
        if let Some(thread) = &self.thread {
            thread.thread().unpark();
        }
        self.send_command(ClickerCommand::Start);
    }
    pub fn pause(&self) {
        self.send_command(ClickerCommand::Pause);
    }
    pub fn toggle(&self) {
        self.send_command(ClickerCommand::Toggle);
    }
    pub fn get_hotkey(&self) -> Key{
        self._hotkey_manager.hotkey
    }
    
    pub fn change_interval(&mut self, new_duration: Duration){
        self.send_command(ClickerCommand::OverwriteDuration(new_duration))
    }
    fn send_command(&self, command: ClickerCommand){
        self.command_channel.blocking_send(command).unwrap()
    }
}

impl Drop for Autoclicker {
    fn drop(&mut self) {
        self.command_channel
            .blocking_send(ClickerCommand::Terminate)
            .expect("Autoclicker Thread has paniced!");
        self.start();
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

fn spawn_clicker_thread(settings:AutoclickerSettings, mut rx: mpsc::Receiver<ClickerCommand>, state: Arc<Mutex<ClickerState>>) -> JoinHandle<()>{
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
            tokio::select!{
                command = rx.recv() => {
                    match command
                        .expect("Channel to clicker closed unexpectedly") {
                        ClickerCommand::Terminate => return (),
                        ClickerCommand::OverwriteDuration(new_duration) => {
                            interval = dbg!(new_duration);
                            timer = time::interval(new_duration)
                        }, // overwrite the current timer
                        command @ (ClickerCommand::Pause | ClickerCommand::Start | ClickerCommand::Toggle)=> {
                            dbg!(command);
                            let mut state_lock = state.lock().await;
                            let prev_state = *state_lock;
                            if prev_state.should_pause(command){
                                eprintln!("pausing");
                                *state_lock = ClickerState::Paused;
                            } else if prev_state.should_resume(command){
                                eprintln!("resuming");
                                *state_lock = ClickerState::Running;
                            }
                        },
                    }
                }
                _ = timer.tick() => {
                    match *state.lock().await {
                        ClickerState::Paused => continue,
                        ClickerState::Running => Autoclicker::click(button),
                    }
                }
            };
        }
    });
    })
}

#[test]
fn sanity_check(){
    let (tx, mut rx) = mpsc::channel(15);
    let tx2 = tx.clone();
    tx2.blocking_send(()).unwrap();
    assert_eq!(Some(()), rx.blocking_recv());
}