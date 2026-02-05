// pixie/src/processors/compressor.rs
use crate::core::{ImageToolError, Result};
use image::{DynamicImage, ImageFormat, ImageOutputFormat};
use oxipng::{optimize_from_memory, Options};
use std::fs::File;
use std::io::{BufWriter, Cursor};
use std::path::Path;

pub struct Compressor {
    quality: u8,
    optimize_png: bool,
    progressive_jpeg: bool,
}

impl Compressor {
    pub fn new(quality: u8) -> Self {
        Self {
            quality: quality.clamp(1, 100),
            optimize_png: true,
            progressive_jpeg: false,
        }
    }

    pub fn with_png_optimization(mut self, optimize: bool) -> Self {
        self.optimize_png = optimize;
        self
    }

    pub fn with_progressive_jpeg(mut self, progressive: bool) -> Self {
        self.progressive_jpeg = progressive;
        self
    }

    pub fn save(&self, image: &DynamicImage, path: &Path) -> Result<()> {
        let format = self.detect_format(path);
        self.save_with_format(image, path, format)
    }

    pub fn save_with_format(
        &self,
        image: &DynamicImage,
        path: &Path,
        format: ImageFormat,
    ) -> Result<()> {
        log::debug!(
            "Saving image to {} with format {:?}, quality: {}",
            path.display(),
            format,
            self.quality
        );

        match format {
            ImageFormat::Jpeg => self.save_jpeg(image, path),
            ImageFormat::Png => self.save_png(image, path),
            ImageFormat::WebP => self.save_webp(image, path),
            _ => self.save_generic(image, path, format),
        }
    }

    fn save_jpeg(&self, image: &DynamicImage, path: &Path) -> Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);

        let output_format = if self.progressive_jpeg {
            ImageOutputFormat::JpegProgressive(self.quality)
        } else {
            ImageOutputFormat::Jpeg(self.quality)
        };

        image.write_to(writer, output_format)?;
        self.log_save_result(path)
    }

    fn save_png(&self, image: &DynamicImage, path: &Path) -> Result<()> {
        if self.optimize_png {
            // First save to memory
            let mut buffer = Cursor::new(Vec::new());
            image.write_to(&mut buffer, ImageOutputFormat::Png)?;
            
            // Optimize with oxipng
            let optimized_data = optimize_from_memory(&buffer.into_inner(), &Options::default())
                .map_err(|e| ImageToolError::ProcessingError(format!("PNG optimization failed: {}", e)))?;
            
            // Write optimized data
            std::fs::write(path, optimized_data)?;
        } else {
            let file = File::create(path)?;
            let writer = BufWriter::new(file);
            image.write_to(writer, ImageOutputFormat::Png)?;
        }

        self.log_save_result(path)
    }

    fn save_webp(&self, image: &DynamicImage, path: &Path) -> Result<()> {
        #[cfg(feature = "webp")]
        {
            use image::codecs::webp::WebPEncoder;
            let file = File::create(path)?;
            let mut writer = BufWriter::new(file);
            
            let encoder = WebPEncoder::new_lossy(&mut writer);
            encoder.encode(
                image.as_bytes(),
                image.width(),
                image.height(),
                image.color(),
            )?;
        }
        
        #[cfg(not(feature = "webp"))]
        {
            return Err(ImageToolError::UnsupportedFormat(
                "WebP support requires 'webp' feature flag".to_string()
            ));
        }

        self.log_save_result(path)
    }

    fn save_generic(
        &self,
        image: &DynamicImage,
        path: &Path,
        format: ImageFormat,
    ) -> Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        image.write_to(writer, ImageOutputFormat::from(format))?;
        
        self.log_save_result(path)
    }

    pub fn compress_to_bytes(
        &self,
        image: &DynamicImage,
        format: ImageFormat,
    ) -> Result<Vec<u8>> {
        let mut buffer = Cursor::new(Vec::new());

        match format {
            ImageFormat::Jpeg => {
                let output_format = if self.progressive_jpeg {
                    ImageOutputFormat::JpegProgressive(self.quality)
                } else {
                    ImageOutputFormat::Jpeg(self.quality)
                };
                image.write_to(&mut buffer, output_format)?;
            }
            ImageFormat::Png => {
                image.write_to(&mut buffer, ImageOutputFormat::Png)?;
                if self.optimize_png {
                    return self.optimize_png_bytes(&buffer.into_inner());
                }
            }
            _ => {
                image.write_to(&mut buffer, ImageOutputFormat::from(format))?;
            }
        }

        Ok(buffer.into_inner())
    }

    fn optimize_png_bytes(&self, data: &[u8]) -> Result<Vec<u8>> {
        optimize_from_memory(data, &Options::default())
            .map_err(|e| ImageToolError::ProcessingError(format!("PNG optimization failed: {}", e)))
    }

    fn detect_format(&self, path: &Path) -> ImageFormat {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("jpg") | Some("jpeg") => ImageFormat::Jpeg,
            Some("png") => ImageFormat::Png,
            Some("gif") => ImageFormat::Gif,
            Some("bmp") => ImageFormat::Bmp,
            Some("webp") => ImageFormat::WebP,
            Some("tiff") | Some("tif") => ImageFormat::Tiff,
            _ => ImageFormat::Jpeg,
        }
    }

    fn log_save_result(&self, path: &Path) -> Result<()> {
        let file_size = std::fs::metadata(path)?.len();
        log::info!("Saved image: {} ({} bytes)", path.display(), file_size);
        Ok(())
    }

    pub fn calculate_savings(&self, original_size: u64, compressed_size: u64) -> f64 {
        if original_size == 0 {
            return 0.0;
        }

        let savings = (original_size as f64 - compressed_size as f64) / original_size as f64 * 100.0;
        savings.max(0.0)
    }
}