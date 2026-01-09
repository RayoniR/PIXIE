#[cfg(test)]
mod tests {
    use assert_fs::prelude::*;
    use assert_fs::TempDir;
    use image_tool_rs::{ImageProcessor, ProcessConfig, ResizeAlgorithm};
    use std::fs;

    #[test]
    fn test_resize_image() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = temp_dir.child("test.jpg");
        
        // Create a simple test image (1x1 pixel)
        let img = image::RgbImage::new(1, 1);
        img.save(input_path.path()).unwrap();
        
        let output_path = temp_dir.child("output.jpg");
        
        let config = ProcessConfig {
            width: 100,
            height: 100,
            scale: 0.0,
            quality: 90,
            keep_aspect: true,
            strip_metadata: false,
            algorithm: ResizeAlgorithm::Lanczos3,
        };
        
        let processor = ImageProcessor::new(config);
        let result = processor.process(input_path.path(), output_path.path());
        
        assert!(result.is_ok());
        assert!(output_path.path().exists());
    }
    
    #[test]
    fn test_invalid_file() {
        let config = ProcessConfig::default();
        let processor = ImageProcessor::new(config);
        
        let result = processor.process("nonexistent.jpg", "output.jpg");
        
        assert!(result.is_err());
    }
}