// pixie/src/core/processor.rs
use super::{ImageToolError, ProcessConfig, Result, ImageMetadata, ProcessingStats};
use crate::processors::{Loader, Resizer, Compressor, MetadataProcessor};
use std::path::{Path, PathBuf};

pub struct ImageProcessor {
    config: ProcessConfig,
    loader: Loader,
    resizer: Resizer,
    compressor: Compressor,
    metadata_processor: MetadataProcessor,
}

impl ImageProcessor {
    pub fn new(config: ProcessConfig) -> Self {
        let resizer = Resizer::new(config.algorithm, config.keep_aspect);
        let compressor = Compressor::new(config.quality);
        let metadata_processor = MetadataProcessor::new();

        Self {
            config,
            loader: Loader::new(),
            resizer,
            compressor,
            metadata_processor,
        }
    }

    pub fn process<P: AsRef<Path>>(&self, input_path: P, output_path: P) -> Result<ProcessingStats> {
        self.process_single(input_path, output_path)
    }

    pub fn process_single<P: AsRef<Path>>(
        &self,
        input_path: P,
        output_path: P,
    ) -> Result<ProcessingStats> {
        let input_path = input_path.as_ref();
        let output_path = output_path.as_ref();

        self.validate_paths(input_path, output_path)?;

        // Load image with memory limit check
        let original_size = std::fs::metadata(input_path)?.len();
        if let Some(max_size) = self.config.max_file_size {
            if original_size > max_size {
                return Err(ImageToolError::MemoryLimitExceeded(
                    format!("File size {} exceeds limit {}", original_size, max_size)
                ));
            }
        }

        let mut image = self.loader.load(input_path)?;
        
        // Strip metadata if requested
        if self.config.strip_metadata {
            self.metadata_processor.strip_metadata(&mut image, input_path)?;
        }

        // Resize if needed
        if self.config.width > 0 || self.config.height > 0 || self.config.scale > 0.0 {
            let mode = if self.config.scale > 0.0 {
                crate::processors::ResizeMode::Scale(self.config.scale)
            } else {
                crate::processors::ResizeMode::Absolute(self.config.width, self.config.height)
            };
            
            image = self.resizer.resize(&image, mode);
        }

        // Determine output format
        let output_format = match self.config.format {
            Some(crate::core::OutputFormat::Jpeg) => image::ImageFormat::Jpeg,
            Some(crate::core::OutputFormat::Png) => image::ImageFormat::Png,
            Some(crate::core::OutputFormat::WebP) => image::ImageFormat::WebP,
            _ => self.loader.detect_format(input_path)?,
        };

        // Compress and save
        self.compressor.save_with_format(&image, output_path, output_format)?;

        let new_size = std::fs::metadata(output_path)?.len();
        
        let mut stats = ProcessingStats::default();
        stats.processed_count = 1;
        stats.total_size_before = original_size;
        stats.total_size_after = new_size;
        
        Ok(stats)
    }

    pub fn get_metadata<P: AsRef<Path>>(&self, path: P) -> Result<ImageMetadata> {
        let path = path.as_ref();
        
        if !path.exists() {
            return Err(ImageToolError::InvalidParameter(
                format!("File does not exist: {}", path.display())
            ));
        }

        let metadata = std::fs::metadata(path)?;
        let (width, height, format) = self.loader.get_dimensions_and_format(path)?;
        let has_exif = self.metadata_processor.has_metadata(path)?;

        Ok(ImageMetadata {
            width,
            height,
            format,
            has_exif,
            file_size: metadata.len(),
        })
    }

    fn validate_paths(&self, input_path: &Path, output_path: &Path) -> Result<()> {
        // Security: Prevent path traversal
        if input_path.to_string_lossy().contains("..") {
            return Err(ImageToolError::SecurityError(
                "Path traversal detected in input path".to_string()
            ));
        }

        if output_path.to_string_lossy().contains("..") {
            return Err(ImageToolError::SecurityError(
                "Path traversal detected in output path".to_string()
            ));
        }

        if !input_path.exists() {
            return Err(ImageToolError::InvalidParameter(
                format!("Input file does not exist: {}", input_path.display())
            ));
        }

        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        Ok(())
    }
}