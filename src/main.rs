#![windows_subsystem = "windows"]

slint::include_modules!();

mod api;
mod app;
mod autorun;
mod coreup;
mod dialog;
mod engine;
mod ffi;
mod icon;
mod json;
mod net;
mod pick;
mod profiles;
mod settings;
mod sysproxy;
mod tray;
mod util;
mod view;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let wait = args.iter().any(|a| a == "--wait");
    let hidden = args.iter().any(|a| a == "--hidden");
    let Some(_lock) = tray::lock(wait) else {
        tray::wake();
        return;
    };
    let Ok(ui) = AppWindow::new() else { return };
    tray::init(app::on_tray, app::tray_state);
    app::init(&ui);
    app::repaint(&ui);
    if !hidden {
        let _ = slint::ComponentHandle::show(&ui);
    }
    let _ = slint::run_event_loop_until_quit();
    app::shutdown();
}
