// #![cfg_attr(debug_assertions, windows_subsystem = "windows")]
use std::{
    default, sync::{Mutex, atomic::AtomicBool}, thread::{self, JoinHandle, Thread}, time::{Duration, Instant},
};

use clicker::{Autoclicker, AutoclickerSettings};
use eframe::{
    egui::{self, Color32, RichText},
};
use rdev::{Button, Key};


fn main() -> Result<(), ()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "My egui App",
        native_options,
        Box::new(|cc| Ok(Box::new(MyEguiApp::new(cc)))),
    )
    .unwrap();
    Ok(())
}


struct MyEguiApp {
    interval_text: String,
    running_clicker: Option<Autoclicker>,
    clicker_settings: AutoclickerSettings,
    hotkey: Key,
}

impl MyEguiApp {
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        let hotkey = Key::F6;
        MyEguiApp {
            running_clicker: Default::default(),
            interval_text: Default::default(),
            clicker_settings: Default::default(),
            hotkey
        }
    }
    fn stop_clicker(&mut self) {
        self.running_clicker = None;
    }
    fn start_clicker(&mut self) {
        self.running_clicker = Some(Autoclicker::new(self.clicker_settings.clone(), self.hotkey))
    }
    fn toggle_clicker(&mut self) {
        match self.running_clicker {
            Some(_) => self.stop_clicker(),
            None => self.start_clicker(),
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.key_pressed(egui::Key::F7)){
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

pub(crate) mod key_input;

mod clicker;

#[cfg(test)]
mod test;
