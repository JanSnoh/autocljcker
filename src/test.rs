use crate::{AutoclickerSettings, HotkeyManager};
use enigo::Mouse;
use enigo::Keyboard;
use global_hotkey::hotkey::Code;
use global_hotkey::hotkey::{Code::F6, HotKey};
use std::{sync::mpsc, thread::sleep, time::Duration};
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
#[test]
fn manual_test_input(){
    let hotkey = HotKey::new(None, Code::Space);
    let mut man = HotkeyManager::with_key(hotkey);
    let settings = enigo::Settings::default();
    let mut enigo = enigo::Enigo::new(&settings).unwrap();
    loop{
        if man.global_key_pressed(){
            println!("Key pressed!");
        }
        sleep(Duration::from_millis(1000));
        enigo.key(enigo::Key::F6, enigo::Direction::Press).unwrap();
    }
}
#[test]
fn test_callback(){
    let hotkey = HotKey::new(None, Code::Space);
    let mut man = HotkeyManager::with_key(hotkey);
    let settings = enigo::Settings::default();
    let mut enigo = enigo::Enigo::new(&settings).unwrap();
    man.register_callback(Box::new(||println!("Key press!")));
    loop {
        
    }
    loop{
        if man.global_key_pressed(){
            println!("Key pressed!");
        }
        sleep(Duration::from_millis(1000));
        enigo.key(enigo::Key::F6, enigo::Direction::Press).unwrap();
    }
}
