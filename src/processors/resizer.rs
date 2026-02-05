// pixie/src/processors/resizer.rs
use crate::core::{ImageToolError, ResizeAlgorithm, Result};
use image::{DynamicImage, imageops::FilterType};

#[derive(Debug, Clone, Copy)]
pub enum ResizeMode {
    Absolute(u32, u32),
    Scale(f32),
    Width(u32),
    Height(u32),
}

pub struct Resizer {
    algorithm: ResizeAlgorithm,
    keep_aspect: bool,
}

impl Resizer {
    pub fn new(algorithm: ResizeAlgorithm, keep_aspect: bool) -> Self {
        Self { algorithm, keep_aspect }
    }

    pub fn resize(&self, image: &DynamicImage, mode: ResizeMode) -> DynamicImage {
        let (width, height) = self.calculate_dimensions(image, mode);
        
        if width == image.width() && height == image.height() {
            log::debug!("Image dimensions unchanged, skipping resize");
            return image.clone();
        }

        log::debug!(
            "Resizing image from {}x{} to {}x{}",
            image.width(),
            image.height(),
            width,
            height
        );

        let filter = self.get_filter_type();

        if self.keep_aspect {
            image.resize(width, height, filter)
        } else {
            image.resize_exact(width, height, filter)
        }
    }

    pub fn resize_exact(&self, image: &DynamicImage, width: u32, height: u32) -> DynamicImage {
        if width == image.width() && height == image.height() {
            return image.clone();
        }

        let filter = self.get_filter_type();
        image.resize_exact(width, height, filter)
    }

    fn calculate_dimensions(&self, image: &DynamicImage, mode: ResizeMode) -> (u32, u32) {
        let (orig_width, orig_height) = image.dimensions();
        
        match mode {
            ResizeMode::Absolute(w, h) => {
                if w == 0 && h == 0 {
                    (orig_width, orig_height)
                } else if self.keep_aspect {
                    self.preserve_aspect(orig_width, orig_height, w, h)
                } else {
                    (
                        if w == 0 { orig_width } else { w },
                        if h == 0 { orig_height } else { h }
                    )
                }
            }
            ResizeMode::Scale(scale) => {
                if scale <= 0.0 {
                    return (orig_width, orig_height);
                }
                let new_width = (orig_width as f32 * scale / 100.0).round() as u32;
                let new_height = (orig_height as f32 * scale / 100.0).round() as u32;
                (new_width.max(1), new_height.max(1))
            }
            ResizeMode::Width(width) => {
                if width == 0 || width == orig_width {
                    return (orig_width, orig_height);
                }
                let ratio = width as f32 / orig_width as f32;
                let height = (orig_height as f32 * ratio).round() as u32;
                (width, height.max(1))
            }
            ResizeMode::Height(height) => {
                if height == 0 || height == orig_height {
                    return (orig_width, orig_height);
                }
                let ratio = height as f32 / orig_height as f32;
                let width = (orig_width as f32 * ratio).round() as u32;
                (width.max(1), height)
            }
        }
    }

    fn preserve_aspect(&self, orig_w: u32, orig_h: u32, target_w: u32, target_h: u32) -> (u32, u32) {
        if target_w == 0 && target_h == 0 {
            return (orig_w, orig_h);
        }

        if target_w == 0 {
            let ratio = target_h as f32 / orig_h as f32;
            let width = (orig_w as f32 * ratio).round() as u32;
            return (width.max(1), target_h);
        }

        if target_h == 0 {
            let ratio = target_w as f32 / orig_w as f32;
            let height = (orig_h as f32 * ratio).round() as u32;
            return (target_w, height.max(1));
        }

        let ratio_w = target_w as f32 / orig_w as f32;
        let ratio_h = target_h as f32 / orig_h as f32;
        let ratio = ratio_w.min(ratio_h);

        let new_w = (orig_w as f32 * ratio).round() as u32;
        let new_h = (orig_h as f32 * ratio).round() as u32;

        (new_w.max(1), new_h.max(1))
    }

    fn get_filter_type(&self) -> FilterType {
        match self.algorithm {
            ResizeAlgorithm::Nearest => FilterType::Nearest,
            ResizeAlgorithm::Bilinear => FilterType::Triangle,
            ResizeAlgorithm::Bicubic => FilterType::CatmullRom,
            ResizeAlgorithm::Lanczos3 => FilterType::Lanczos3,
        }
    }

    pub fn calculate_mode_from_config(width: u32, height: u32, scale: f32) -> ResizeMode {
        if scale > 0.0 {
            ResizeMode::Scale(scale)
        } else if width > 0 && height > 0 {
            ResizeMode::Absolute(width, height)
        } else if width > 0 {
            ResizeMode::Width(width)
        } else if height > 0 {
            ResizeMode::Height(height)
        } else {
            ResizeMode::Absolute(0, 0)
        }
    }
}