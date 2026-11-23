//! Скрытый режим снимков экрана для проверки вёрстки.
//! Включается только переменными окружения и не меняет обычный запуск.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use eframe::egui::{self, ColorImage, Vec2, ViewportCommand};

pub struct Capture {
    pub scene: String,
    pub path: String,
    pub width: f32,
    pub height: f32,
    frame: u32,
    saved: Arc<AtomicBool>,
}

impl Capture {
    pub fn from_env() -> Option<Self> {
        let scene = std::env::var("ELECTRO_QUEST_SHOT").ok()?;
        let path = std::env::var("ELECTRO_QUEST_SHOT_PATH")
            .unwrap_or_else(|_| "/tmp/electro-shot.ppm".to_owned());
        let (width, height) = parse_size().unwrap_or((1280.0, 720.0));
        Some(Self {
            scene,
            path,
            width,
            height,
            frame: 0,
            saved: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn tick(&mut self, ctx: &egui::Context) {
        self.frame += 1;
        if self.frame <= 3 {
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(Vec2::new(
                self.width,
                self.height,
            )));
        }
        if self.frame == 12 {
            let saved = Arc::clone(&self.saved);
            let path = self.path.clone();
            ctx.request_screenshot(move |image| {
                if write_ppm(&path, &image).is_ok() {
                    saved.store(true, Ordering::SeqCst);
                }
            });
        }
        if self.saved.load(Ordering::SeqCst) || self.frame > 80 {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        ctx.request_repaint();
    }
}

pub fn parse_size() -> Option<(f32, f32)> {
    let value = std::env::var("ELECTRO_QUEST_SIZE").ok()?;
    let (width, height) = value.split_once('x')?;
    Some((width.parse().ok()?, height.parse().ok()?))
}

fn write_ppm(path: &str, image: &ColorImage) -> std::io::Result<()> {
    use std::io::Write;
    let [width, height] = image.size;
    let mut file = std::fs::File::create(path)?;
    writeln!(file, "P6\n{width} {height}\n255")?;
    let mut rgb = Vec::with_capacity(width * height * 3);
    for pixel in &image.pixels {
        let [r, g, b, _] = pixel.to_array();
        rgb.extend_from_slice(&[r, g, b]);
    }
    file.write_all(&rgb)
}
