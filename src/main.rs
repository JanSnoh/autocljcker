#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{
    default,
    sync::{Mutex, atomic::AtomicBool},
    thread::{self, JoinHandle, Thread},
    time::{Duration, Instant},
};

use clicker::{Autoclicker, AutoclickerSettings};
use eframe::egui::{self, Color32, IconData, RichText};
use rdev::{Button, Key};
use tokio::time::Interval;

fn main() -> Result<(), ()> {
    MyEguiApp::run().unwrap();
    Ok(())
}

struct MyEguiApp {
    interval_text: String,
    clicker: Autoclicker,
    clicker_settings: AutoclickerSettings,
}

impl MyEguiApp {
    pub fn run() -> Result<(), eframe::Error> {
        let native_options = eframe::NativeOptions::default();
        eframe::run_native(
            "My egui App",
            native_options,
            Box::new(|cc| Ok(Box::new(MyEguiApp::new(cc)))),
        )
    }
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        let hotkey = Key::F6;
        MyEguiApp {
            clicker: Autoclicker::new(AutoclickerSettings::default(), hotkey),
            interval_text: String::from("100"),
            clicker_settings: Default::default(),
        }
    }
    fn stop_clicker(&mut self) {
        self.clicker.pause();
    }
    fn start_clicker(&mut self) {
        self.clicker.start();
    }
    fn toggle_clicker(&mut self) {
        self.clicker.toggle();
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.key_pressed(rdev_to_egui(self.clicker.get_hotkey()))) {
            self.toggle_clicker();
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("test");
            ui.horizontal(|ui| {
                ui.label("Interval in ms: ");
                ui.text_edit_singleline(&mut self.interval_text);

                if let Ok(interval) = self.interval_text.parse::<u64>() {
                    let new_interval = Duration::from_millis(interval);
                    if new_interval != self.clicker_settings.interval{
                        self.clicker_settings.interval = new_interval;
                        self.clicker.change_interval(new_interval);
                    }
                } else {
                    ui.label(RichText::new("Invalid Interval").color(Color32::RED));
                }
            });
            let selected = &mut self.clicker_settings.button;
            egui::ComboBox::from_label("Mouse Button")
                .selected_text(format!("{:?}", selected))
                .show_ui(ui, |ui| {
                    ui.selectable_value(selected, Button::Left, "Left Mouse");
                    ui.selectable_value(selected, Button::Right, "Right Mouse");
                });
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!self.clicker.is_running(), egui::Button::new("Start! (F6)"))
                    .clicked()
                {
                    self.start_clicker();
                }
                if ui
                    .add_enabled(self.clicker.is_running(), egui::Button::new("Stop! (F6)"))
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

pub(crate) mod key_input;

mod clicker;

// #[cfg(test)]
// mod test;

fn rdev_to_egui(key: rdev::Key) -> egui::Key {
    use egui::Key as E;
    use rdev::Key as R;

    match key {
        R::ShiftLeft
        | R::ShiftRight
        | R::ControlLeft
        | R::ControlRight
        | R::Alt
        | R::AltGr
        | R::MetaLeft
        | R::MetaRight => panic!(),

        R::DownArrow => E::ArrowDown,
        R::LeftArrow => E::ArrowLeft,
        R::RightArrow => E::ArrowRight,
        R::UpArrow => E::ArrowUp,
        R::Escape => E::Escape,
        R::Tab => E::Tab,
        R::Backspace => E::Backspace,
        R::Return | R::KpReturn => E::Enter,
        R::Space => E::Space,
        R::Insert => E::Insert,
        R::Delete | R::KpDelete => E::Delete,
        R::Home => E::Home,
        R::End => E::End,
        R::PageUp => E::PageUp,
        R::PageDown => E::PageDown,

        R::Num0 | R::Kp0 => E::Num0,
        R::Num1 | R::Kp1 => E::Num1,
        R::Num2 | R::Kp2 => E::Num2,
        R::Num3 | R::Kp3 => E::Num3,
        R::Num4 | R::Kp4 => E::Num4,
        R::Num5 | R::Kp5 => E::Num5,
        R::Num6 | R::Kp6 => E::Num6,
        R::Num7 | R::Kp7 => E::Num7,
        R::Num8 | R::Kp8 => E::Num8,
        R::Num9 | R::Kp9 => E::Num9,

        R::KeyA => E::A,
        R::KeyB => E::B,
        R::KeyC => E::C,
        R::KeyD => E::D,
        R::KeyE => E::E,
        R::KeyF => E::F,
        R::KeyG => E::G,
        R::KeyH => E::H,
        R::KeyI => E::I,
        R::KeyJ => E::J,
        R::KeyK => E::K,
        R::KeyL => E::L,
        R::KeyM => E::M,
        R::KeyN => E::N,
        R::KeyO => E::O,
        R::KeyP => E::P,
        R::KeyQ => E::Q,
        R::KeyR => E::R,
        R::KeyS => E::S,
        R::KeyT => E::T,
        R::KeyU => E::U,
        R::KeyV => E::V,
        R::KeyW => E::W,
        R::KeyX => E::X,
        R::KeyY => E::Y,
        R::KeyZ => E::Z,

        R::F1 => E::F1,
        R::F2 => E::F2,
        R::F3 => E::F3,
        R::F4 => E::F4,
        R::F5 => E::F5,
        R::F6 => E::F6,
        R::F7 => E::F7,
        R::F8 => E::F8,
        R::F9 => E::F9,
        R::F10 => E::F10,
        R::F11 => E::F11,
        R::F12 => E::F12,

        R::BackQuote => E::Backtick,
        R::Minus | R::KpMinus => E::Minus,
        R::Equal => E::Equals,
        R::LeftBracket => E::OpenBracket,
        R::RightBracket => E::CloseBracket,
        R::SemiColon => E::Semicolon,
        R::Quote => E::Quote,
        R::BackSlash => E::Backslash,
        R::Comma => E::Comma,
        R::Dot => E::Period,
        R::Slash => E::Slash,

        _ => panic!("unexpected button!"),
    }
}
