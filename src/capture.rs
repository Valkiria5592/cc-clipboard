use crate::config::Config;
use image::{DynamicImage, RgbaImage};
use std::path::PathBuf;

/// Captures every connected monitor. Returns (image, x, y, width, height) per monitor.
pub fn capture_all() -> Vec<(RgbaImage, i32, i32, u32, u32)> {
    let Ok(monitors) = xcap::Monitor::all() else {
        return vec![];
    };
    monitors
        .iter()
        .filter_map(|m| {
            let img = m.capture_image().ok()?;
            Some((img, m.x(), m.y(), m.width(), m.height()))
        })
        .collect()
}

pub fn crop_and_save(img: &RgbaImage, x: u32, y: u32, w: u32, h: u32, config: &Config) -> Option<PathBuf> {
    if w == 0 || h == 0 {
        return None;
    }
    let cropped = DynamicImage::ImageRgba8(img.clone()).crop_imm(x, y, w, h);
    let filename = format!("cc_{}.png", chrono::Local::now().format("%Y%m%d_%H%M%S"));
    let path = config.save_folder.join(&filename);
    cropped.save(&path).ok()?;
    Some(path)
}
