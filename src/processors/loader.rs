// pixie/src/processors/loader.rs
use crate::core::{ImageToolError, Result};
use image::{DynamicImage, ImageFormat, ImageReader, GenericImageView};
use crate::utils::image_format_to_string;
use std::path::Path;

#[derive(Clone)]
pub struct Loader {
    max_dimensions: Option<(u32, u32)>,
}

impl Loader {
    pub fn new() -> Self {
        Self {
            max_dimensions: Some((100_000, 100_000)),
        }
    }

    pub fn with_max_dimensions(mut self, width: u32, height: u32) -> Self {
        self.max_dimensions = Some((width, height));
        self
    }

    pub fn load(&self, path: &Path) -> Result<DynamicImage> {
        log::debug!("Loading image from: {}", path.display());

        self.validate_path(path)?;

        let image = ImageReader::open(path)?
            .with_guessed_format()?
            .decode()
            .map_err(|e| {
                ImageToolError::ProcessingError(format!("Failed to decode image: {}", e))
            })?;

        // Validate dimensions
        if let Some((max_w, max_h)) = self.max_dimensions {
            let (width, height) = image.dimensions();
            if width > max_w || height > max_h {
                return Err(ImageToolError::MemoryLimitExceeded(
                    format!("Image dimensions {}x{} exceed maximum {}x{}", 
                        width, height, max_w, max_h)
                ));
            }
        }

        let (width, height) = image.dimensions();
        let format = image.color();

        log::info!(
            "Loaded image: {}x{} pixels, format: {:?}",
            width, height, format
        );

        Ok(image)
    }

    pub fn load_from_bytes(&self, data: &[u8]) -> Result<DynamicImage> {
        let image = image::load_from_memory(data)
            .map_err(|e| {
                ImageToolError::ProcessingError(format!("Failed to decode image from bytes: {}", e))
            })?;

        Ok(image)
    }

    pub fn get_dimensions_and_format(&self, path: &Path) -> Result<(u32, u32, String)> {
        let file = std::fs::File::open(path)?;
        let reader = image::io::Reader::new(std::io::BufReader::new(file))
            .with_guessed_format()?;
        
        let format = reader.format()
            .map(|f| image_format_to_string(f))
            .unwrap_or_else(|| "Unknown".to_string());
        
        let dimensions = reader.into_dimensions()?;
        
        Ok((dimensions.0, dimensions.1, format))
    }

    pub fn detect_format(&self, path: &Path) -> Result<ImageFormat> {
        let format = image::ImageFormat::from_path(path)
            .map_err(|_| ImageToolError::ProcessingError(format!("Failed to detect format for: {}", path.display())))?;
        
        Ok(format)
    }

    fn validate_path(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            return Err(ImageToolError::InvalidParameter(
                format!("File does not exist: {}", path.display())
            ));
        }

        let metadata = path.metadata()?;
        if metadata.len() == 0 {
            return Err(ImageToolError::InvalidParameter(
                format!("File is empty: {}", path.display())
            ));
        }

        Ok(())
    }
}

impl Default for Loader {
    fn default() -> Self {
        Self::new()
    }
}