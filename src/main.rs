// #![cfg_attr(debug_assertions, windows_subsystem = "windows")]
use std::{
    default, sync::{Mutex, atomic::AtomicBool}, thread::{self, JoinHandle, Thread}, time::{Duration, Instant},
};

use clicker::{Autoclicker, AutoclickerSettings};
use eframe::{
    egui::{self, Color32, RichText},
};
use enigo::{self, Button, Direction, Mouse, Settings};
use global_hotkey::{GlobalHotKeyEvent, hotkey::HotKey};

use global_hotkey::GlobalHotKeyManager;

fn main() -> Result<(), ()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "My egui App",
        native_options,
        Box::new(|cc| Ok(Box::new(MyEguiApp::new(cc)))),
    )
    .unwrap();

    let mut enigo = enigo::Enigo::new(&Settings::default()).unwrap();
    enigo.button(Button::Right, Direction::Click).unwrap();
    Ok(())
}


struct MyEguiApp {
    interval_text: String,
    running_clicker: Option<Autoclicker>,
    clicker_settings: AutoclickerSettings,
    hotkey_manager: HotkeyManager,
}

pub fn test(){

}

impl MyEguiApp {
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        use global_hotkey::hotkey::Code;
        let hotkey = HotKey::new(None, Code::F6);
        let hotkey_manager = HotkeyManager::with_key(hotkey);
        MyEguiApp {
            running_clicker: Default::default(),
            interval_text: Default::default(),
            clicker_settings: Default::default(),
            hotkey_manager
        }
    }
    fn stop_clicker(&mut self) {
        self.running_clicker = None;
    }
    fn start_clicker(&mut self) {
        self.running_clicker = Some(Autoclicker::new(self.clicker_settings.clone()))
    }
    fn toggle_clicker(&mut self) {
        match self.running_clicker {
            Some(_) => self.stop_clicker(),
            None => self.start_clicker(),
        }
    }
    fn hotkey_pressed(&mut self) -> bool{
        self.hotkey_manager.global_key_pressed()
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.key_pressed(egui::Key::F7)){
            self.toggle_clicker();
        }
        if self.hotkey_pressed(){
            self.toggle_clicker();
        }
        if let Some(clicker) = &self.running_clicker {
            if !clicker.is_running() {
                eprintln!("Clicker exited unexpectedly");
                self.running_clicker = None;
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("test");
            ui.horizontal(|ui| {
                ui.label("Interval in ms: ");
                ui.text_edit_singleline(&mut self.interval_text);

                if let Ok(interval) = self.interval_text.parse::<u64>() {
                    self.clicker_settings.interval = Duration::from_millis(interval);
                } else {
                    ui.label(RichText::new("Invalid Interval").color(Color32::RED));
                }
            });
            let selected = &mut self.clicker_settings.button;
            let before = *selected;
            egui::ComboBox::from_label("Mouse Button")
                .selected_text(format!("{:?}", selected))
                .show_ui(ui, |ui| {
                    ui.selectable_value(selected, Button::Left, "Left Mouse");
                    ui.selectable_value(selected, Button::Right, "Right Mouse");
                });
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.running_clicker.is_none(),
                        egui::Button::new("Start! (F6)"),
                    )
                    .clicked()
                {
                    self.start_clicker();
                }
                if ui
                    .add_enabled(
                        self.running_clicker.is_some(),
                        egui::Button::new("Stop! (F6)"),
                    )
                    .clicked()
                {
                    self.stop_clicker();
                }
            });
        });
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        dbg!(&self.interval_text);
    }
}

struct HotkeyManager{
    pub hotkey: global_hotkey::hotkey::HotKey,
    last_hotkey_time: Instant,
    active: AtomicBool,
    _hotkey_manager: GlobalHotKeyManager,
}

impl HotkeyManager{
    const DEBOUNCE_THRESHOLD: Duration = Duration::from_millis(500);
    pub fn with_key(hotkey: HotKey) -> Self{
        let _hotkey_manager = GlobalHotKeyManager::new().expect("Couldn't create HotkeyManager");
        _hotkey_manager
            .register(hotkey)
            .expect("Could not register Hotkey");
        Self { hotkey, last_hotkey_time: Instant::now(), active: false.into(), _hotkey_manager }
    }
    pub fn register_callback(&mut self, callback: Box<dyn Fn() + Send + Sync>){
        let hotkey = self.hotkey;
        let last_hotkey_time_mutex = Mutex::new(self.last_hotkey_time);
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.id() != hotkey.id(){
                return;
            }
            let now = Instant::now();
            let mut hotkey_lock = last_hotkey_time_mutex.lock().unwrap();
            if dbg!(now.duration_since(*hotkey_lock)) <= Self::DEBOUNCE_THRESHOLD{
                return;
            }
            *hotkey_lock = now;
            callback();

        }));
    }
    pub fn unregister_callback(&mut self){
        GlobalHotKeyEvent::set_event_handler::<fn(GlobalHotKeyEvent)>(None);
    }
    pub fn global_key_pressed(&mut self) -> bool{
        let global_key_event_reciever = GlobalHotKeyEvent::receiver();
        //matches!(event_poll_result, Ok(key_event) if key_event.id == self.hotkey.id && key_event.state == global_hotkey::HotKeyState::Released)
        while let Ok(key_event) = global_key_event_reciever.try_recv(){
            if dbg!(key_event).id() != self.hotkey.id(){
                continue;
            }
            let now = Instant::now();
            if dbg!(now.duration_since(self.last_hotkey_time)) <= Self::DEBOUNCE_THRESHOLD{
                continue;
            }
            self.last_hotkey_time = now;
            return true;
        }
        false
    }
}

mod clicker {
    use enigo::{self, Direction, Mouse, Settings};
    use std::{
        sync::{Arc, atomic::{AtomicBool, Ordering}}, thread::{self, JoinHandle, Thread}, time::Duration,
    };

    #[derive(Debug, Default, Clone, PartialEq, Eq)]
    pub struct AutoclickerSettings {
        pub interval: Duration,
        pub button: enigo::Button,
    }

    #[derive(Debug)]
    pub struct Autoclicker {
        channel: std::sync::mpsc::Sender<()>,
        thread: Option<JoinHandle<()>>,
        active: Arc<AtomicBool>
    }

    impl Autoclicker {
        pub fn new(settings: AutoclickerSettings) -> Self {
            let (tx, rx) = std::sync::mpsc::channel();
            let active: Arc<AtomicBool> = Arc::new(false.into());
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
                rt.block_on(async {
                    loop {
                        timer.tick().await.unwrap();
                        if rx.try_recv().is_ok() {
                            break;
                        }
                        // If the ticker is inactive we can sleep until we are woken up
                        if !_active.load(Ordering::Relaxed){
                                thread::park();
                        }
                        let mut enigo = enigo::Enigo::new(&Settings::default()).unwrap();
                        enigo.button(button, Direction::Click).unwrap();
                    }
                });
            });
            Self {
                channel: tx,
                thread: Some(thread),
                active,
            }
        }
        pub fn is_running(&self) -> bool {
            match &self.thread {
                Some(handle) => !handle.is_finished() && !self.active.load(Ordering::Relaxed),
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
            // join the thread and panic if joining failed.
            self.thread
                .take()
                .map(JoinHandle::join)
                .transpose()
                .unwrap();
        }
    }
}

#[cfg(test)]
mod test;
