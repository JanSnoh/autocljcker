// #![cfg_attr(debug_assertions, windows_subsystem = "windows")]
use std::{
    default, thread::{self, JoinHandle, Thread}, time::{Duration, Instant},
};

use clicker::{Autoclicker, AutoclickerSettings};
use eframe::{
    UserEvent,
    egui::{self, Color32, Key::E, RichText},
};
use enigo::{self, Button, Direction, Mouse, Settings};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyEventReceiver, hotkey::HotKey};

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
    hotkey: global_hotkey::hotkey::HotKey,
    _hotkey_manager: GlobalHotKeyManager,
}

impl MyEguiApp {
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        use global_hotkey::hotkey::Code;
        let hotkey = HotKey::new(None, Code::F6);
        let _hotkey_manager = GlobalHotKeyManager::new().expect("Couldn't create HotkeyManager");
        _hotkey_manager
            .register(hotkey)
            .expect("Could not register Hotkey");
        MyEguiApp {
            running_clicker: Default::default(),
            interval_text: Default::default(),
            clicker_settings: Default::default(),
            hotkey,
            _hotkey_manager
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
    fn global_key_pressed(key: HotKey) -> bool{
        let global_key_event_reciever = GlobalHotKeyEvent::receiver();
        let event_poll_result = global_key_event_reciever.try_recv();
        matches!(event_poll_result, Ok(key_event) if key_event.id == key.id && key_event.state == global_hotkey::HotKeyState::Released)
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        let global_key_event_reciever = GlobalHotKeyEvent::receiver();
        if ctx.input(|i| i.key_pressed(egui::Key::F7)){
            self.toggle_clicker();
        }
        if MyEguiApp::global_key_pressed(self.hotkey){
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

mod clicker {
    use enigo::{self, Direction, Mouse, Settings};
    use std::{
        thread::{self, JoinHandle},
        time::Duration,
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
    }

    impl Autoclicker {
        pub fn new(settings: AutoclickerSettings) -> Self {
            let (tx, rx) = std::sync::mpsc::channel();
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
                        let mut enigo = enigo::Enigo::new(&Settings::default()).unwrap();
                        enigo.button(button, Direction::Click).unwrap();
                    }
                });
            });
            Self {
                channel: tx,
                thread: Some(thread),
            }
        }
        pub fn is_running(&self) -> bool {
            match &self.thread {
                Some(handle) => !handle.is_finished(),
                None => false,
            }
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
mod test {
    use crate::AutoclickerSettings;
    use enigo::Mouse;
    use std::{sync::mpsc, time::Duration};
    #[test]
    fn test_clicker() {
        let settings = AutoclickerSettings {
            interval: Duration::from_millis(100),
            button: enigo::Button::Left,
        };
        let (_tx, rx) = mpsc::channel::<()>();
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
                let mut enigo = enigo::Enigo::new(&enigo::Settings::default()).unwrap();
                enigo.button(button, enigo::Direction::Click).unwrap();
            }
        });
    }
}
