#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod board;
mod gameplay;
mod levels;
mod model;
mod shot;
mod simulation;
mod theme;

use app::ElectroApp;

fn main() -> eframe::Result<()> {
    let (inner, min_inner) = window_size();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size(inner)
            .with_min_inner_size(min_inner)
            .with_title("ЭлектроКвест"),
        ..Default::default()
    };

    eframe::run_native(
        "ЭлектроКвест",
        options,
        Box::new(|cc| Ok(Box::new(ElectroApp::new(cc)))),
    )
}

fn window_size() -> ([f32; 2], [f32; 2]) {
    let default = [1280.0, 720.0];
    let min = [1100.0, 680.0];
    let Some((width, height)) = shot::parse_size() else {
        return (default, min);
    };
    ([width, height], [width.min(min[0]), height.min(min[1])])
}
