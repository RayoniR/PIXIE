use crate::core::{ImageToolError, ProcessConfig, Result, ProcessingStats};
use crate::processors::prelude::*;
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::WalkDir;

pub struct BatchProcessor {
    config: ProcessConfig,
    max_threads: usize,
    thread_pool: Option<rayon::ThreadPool>,
}

impl BatchProcessor {
    pub fn new(config: ProcessConfig, max_threads: usize) -> Result<Self> {
        let mut processor = Self {
            config,
            max_threads,
            thread_pool: None,
        };

        // Initialize thread pool once
        if max_threads > 0 {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(max_threads)
                .build()
                .map_err(|e| {
                    ImageToolError::ProcessingError(format!("Failed to create thread pool: {}", e))
                })?;
            processor.thread_pool = Some(pool);
        }

        Ok(processor)
    }

    pub fn process_directory(
        &self,
        input_dir: &Path,
        output_dir: &Path,
        recursive: bool,
    ) -> Result<ProcessingStats> {
        self.validate_paths(input_dir, output_dir)?;

        // Collect image files
        let image_paths = self.collect_image_paths(input_dir, recursive)?;

        if image_paths.is_empty() {
            log::warn!("No image files found in {}", input_dir.display());
            return Ok(ProcessingStats::default());
        }

        log::info!(
            "Processing {} images from {}",
            image_paths.len(),
            input_dir.display()
        );

        // Create output directory
        std::fs::create_dir_all(output_dir)?;

        // Create progress bar
        let pb = self.create_progress_bar(image_paths.len());

        // Process images in parallel
        let config = Arc::new(self.config.clone());
        let output_dir = Arc::new(output_dir.to_path_buf());
        
        let results: Vec<Result<ProcessingStats>> = if let Some(pool) = &self.thread_pool {
            // Use custom thread pool
            pool.install(|| {
                image_paths
                    .par_iter()
                    .progress_with(pb.clone())
                    .map(|input_path| {
                        self.process_single_image_in_batch(
                            input_path,
                            &output_dir,
                            config.as_ref(),
                        )
                    })
                    .collect()
            })
        } else {
            // Use global thread pool
            image_paths
                .par_iter()
                .progress_with(pb.clone())
                .map(|input_path| {
                    self.process_single_image_in_batch(
                        input_path,
                        &output_dir,
                        config.as_ref(),
                    )
                })
                .collect()
        };

        // Aggregate results
        let mut stats = ProcessingStats::default();
        for result in results {
            match result {
                Ok(image_stats) => {
                    stats.processed_count += image_stats.processed_count;
                    stats.total_size_before += image_stats.total_size_before;
                    stats.total_size_after += image_stats.total_size_after;
                }
                Err(e) => {
                    stats.errors.push(("Processing error".to_string(), e.to_string()));
                }
            }
        }

        pb.finish_with_message(format!(
            "Processed {} images ({}% size reduction)",
            stats.processed_count,
            self.calculate_overall_savings(&stats)
        ));

        Ok(stats)
    }

    fn process_single_image_in_batch(
        &self,
        input_path: &Path,
        output_dir: &Arc<PathBuf>,
        config: &ProcessConfig,
    ) -> Result<ProcessingStats> {
        // Calculate output path
        let file_name = input_path
            .file_name()
            .ok_or_else(|| {
                ImageToolError::InvalidParameter(format!("Invalid file name: {}", input_path.display()))
            })?;

        let output_path = output_dir.join(file_name);

        // Create processor and process
        let processor = crate::core::processor::ImageProcessor::new(config.clone());
        processor.process(input_path, &output_path)
    }

    fn collect_image_paths(&self, input_dir: &Path, recursive: bool) -> Result<Vec<PathBuf>> {
        let walker = if recursive {
            WalkDir::new(input_dir)
        } else {
            WalkDir::new(input_dir).max_depth(1)
        };

        let image_extensions = [
            "jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "webp",
        ];

        let paths: Vec<PathBuf> = walker
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .filter(|entry| {
                entry.path().extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| {
                        let ext_lower = ext.to_lowercase();
                        image_extensions.contains(&ext_lower.as_str())
                    })
                    .unwrap_or(false)
            })
            .map(|entry| entry.into_path())
            .collect();

        Ok(paths)
    }

    fn create_progress_bar(&self, total: usize) -> ProgressBar {
        let pb = ProgressBar::new(total as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb
    }

    fn calculate_overall_savings(&self, stats: &ProcessingStats) -> f64 {
        if stats.total_size_before == 0 {
            return 0.0;
        }

        let savings = (stats.total_size_before as f64 - stats.total_size_after as f64)
            / stats.total_size_before as f64 * 100.0;
        savings.max(0.0).min(100.0)
    }

    pub fn validate_paths(&self, input_dir: &Path, output_dir: &Path) -> Result<()> {
        // Security: prevent path traversal
        if input_dir.to_string_lossy().contains("..") {
            return Err(ImageToolError::SecurityError(
                "Path traversal detected in input path".to_string()
            ));
        }

        if output_dir.to_string_lossy().contains("..") {
            return Err(ImageToolError::SecurityError(
                "Path traversal detected in output path".to_string()
            ));
        }

        if !input_dir.exists() {
            return Err(ImageToolError::InvalidParameter(
                format!("Input directory does not exist: {}", input_dir.display())
            ));
        }

        if !input_dir.is_dir() {
            return Err(ImageToolError::InvalidParameter(
                format!("Input path is not a directory: {}", input_dir.display())
            ));
        }

        if output_dir.exists() && !output_dir.is_dir() {
            return Err(ImageToolError::InvalidParameter(
                format!("Output path exists but is not a directory: {}", output_dir.display())
            ));
        }

        // Prevent processing the same directory as output
        if input_dir == output_dir {
            return Err(ImageToolError::InvalidParameter(
                "Input and output directories cannot be the same".to_string()
            ));
        }

        Ok(())
    }
}