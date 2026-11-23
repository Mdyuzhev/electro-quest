#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod board;
mod gameplay;
mod levels;
mod model;
mod simulation;

use app::ElectroApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([1100.0, 680.0])
            .with_title("ЭлектроКвест"),
        ..Default::default()
    };

    eframe::run_native(
        "ЭлектроКвест",
        options,
        Box::new(|cc| Ok(Box::new(ElectroApp::new(cc)))),
    )
}
