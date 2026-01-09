mod cli;
mod loader;
mod resizer;
mod compressor;
mod metadata;
mod batch;
mod utils;

use crate::cli::{Algorithm, Cli, Commands};
use clap::Parser;
use log::LevelFilter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    // Initialize logger
    env_logger::Builder::new()
        .filter_level(if cli.verbose {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        })
        .init();
    
    match cli.command {
        Commands::Resize {
            input,
            output,
            width,
            height,
            scale,
            quality,
            keep_aspect,
            strip_metadata,
            algorithm,
        } => {
            process_resize(
                input,
                output,
                width,
                height,
                scale,
                quality,
                keep_aspect,
                strip_metadata,
                algorithm,
            )?;
        }
        Commands::Batch {
            input,
            output,
            width,
            height,
            quality,
            threads,
            recursive,
            strip_metadata,
            algorithm,
        } => {
            process_batch(
                input,
                output,
                width,
                height,
                quality,
                threads,
                recursive,
                strip_metadata,
                algorithm,
            )?;
        }
        Commands::Optimize {
            input,
            output,
            quality,
            strip_metadata,
        } => {
            process_optimize(input, output, quality, strip_metadata)?;
        }
        Commands::Info { input } => {
            process_info(input)?;
        }
    }
    
    Ok(())
}

fn process_resize(
    input: std::path::PathBuf,
    output: Option<std::path::PathBuf>,
    width: u32,
    height: u32,
    scale: f32,
    quality: u8,
    keep_aspect: bool,
    strip_metadata: bool,
    algorithm: Algorithm,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::utils::generate_output_path;
    
    let output_path = generate_output_path(&input, output.as_deref(), "resized");
    
    let config = crate::ProcessConfig {
        width,
        height,
        scale,
        quality,
        keep_aspect,
        strip_metadata,
        algorithm: algorithm.into(),
        ..Default::default()
    };
    
    let processor = crate::ImageProcessor::new(config);
    processor.process(&input, &output_path)?;
    
    println!("Resized image saved to: {}", output_path.display());
    
    Ok(())
}

fn process_batch(
    input: std::path::PathBuf,
    output: std::path::PathBuf,
    width: u32,
    height: u32,
    quality: u8,
    threads: usize,
    recursive: bool,
    strip_metadata: bool,
    algorithm: Algorithm,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::ProcessConfig {
        width,
        height,
        scale: 0.0,
        quality,
        keep_aspect: true,
        strip_metadata,
        algorithm: algorithm.into(),
    };
    
    let processor = batch::BatchProcessor::new(config, threads);
    
    processor.validate_paths(&input, &output)?;
    
    let processed = processor.process_directory(&input, &output, recursive)?;
    
    println!(
        "Batch processing complete. Processed {} images to: {}",
        processed,
        output.display()
    );
    
    Ok(())
}

fn process_optimize(
    input: std::path::PathBuf,
    output: Option<std::path::PathBuf>,
    quality: u8,
    strip_metadata: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::utils::generate_output_path;
    
    let output_path = generate_output_path(&input, output.as_deref(), "optimized");
    
    let config = crate::ProcessConfig {
        width: 0,
        height: 0,
        scale: 0.0,
        quality,
        keep_aspect: true,
        strip_metadata,
        algorithm: crate::ResizeAlgorithm::Lanczos3,
    };
    
    let processor = crate::ImageProcessor::new(config);
    processor.process(&input, &output_path)?;
    
    println!("Optimized image saved to: {}", output_path.display());
    
    Ok(())
}

fn process_info(input: std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use crate::metadata::MetadataStripper;
    use crate::utils::{format_file_size, get_image_info};
    
    if !input.exists() {
        return Err(format!("File does not exist: {}", input.display()).into());
    }
    
    let metadata_stripper = MetadataStripper::new();
    
    // Get file size
    let metadata = std::fs::metadata(&input)?;
    let file_size = metadata.len();
    
    // Get image dimensions and format
    let (width, height, format) = get_image_info(&input)?;
    let aspect_ratio = width as f32 / height as f32;
    
    // Try to read EXIF metadata
    let has_exif = metadata_stripper.read_metadata(&input)?.is_some();
    
    println!("=== Image Information ===");
    println!("File: {}", input.display());
    println!("Size: {}", format_file_size(file_size));
    println!("Dimensions: {} x {} pixels", width, height);
    println!("Aspect Ratio: {:.2}:1 ({:.2})", aspect_ratio, aspect_ratio);
    println!("Format: {}", format);
    println!("Has EXIF metadata: {}", has_exif);
    
    if has_exif {
        if let Ok(Some(exif)) = metadata_stripper.read_metadata(&input) {
            println!("\n=== EXIF Metadata ===");
            for field in exif.fields() {
                println!(
                    "{}: {}",
                    field.tag,
                    field.display_value().with_unit(&exif)
                );
            }
        }
    }
    
    Ok(())
}