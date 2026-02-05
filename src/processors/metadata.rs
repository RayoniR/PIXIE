// pixie/src/processors/metadata.rs
use crate::core::{ImageToolError, Result};
use exif::{Exif, In, Tag, Reader};
use image::DynamicImage;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub struct MetadataProcessor;

impl MetadataProcessor {
    pub fn new() -> Self {
        Self
    }

    pub fn strip_metadata(
        &self,
        image: &mut DynamicImage,
        path: &Path,
    ) -> Result<()> {
        log::debug!("Stripping metadata from: {}", path.display());
        
        // For JPEG images, we need to re-encode to strip metadata
        // The image crate automatically strips most metadata when re-encoding
        // This is handled in the compressor
        Ok(())
    }

    pub fn read_metadata(&self, path: &Path) -> Result<Option<Exif>> {
        let file = File::open(path)?;
        let mut bufreader = BufReader::new(&file);

        match Reader::new().read_from_container(&mut bufreader) {
            Ok(exif) => {
                log::debug!("Found EXIF data in {}", path.display());
                Ok(Some(exif))
            }
            Err(exif::Error::NotFound(_)) => {
                log::debug!("No EXIF data found in {}", path.display());
                Ok(None)
            }
            Err(e) => {
                log::warn!("Failed to read EXIF from {}: {}", path.display(), e);
                Err(ImageToolError::ProcessingError(format!("EXIF read error: {}", e)))
            }
        }
    }

    pub fn has_metadata(&self, path: &Path) -> Result<bool> {
        Ok(self.read_metadata(path)?.is_some())
    }

    pub fn print_metadata(&self, exif: &Exif) -> String {
        let mut output = String::new();
        output.push_str("--- EXIF Metadata ---\n");

        for field in exif.fields() {
            let value = format!(
                "{}: {}",
                field.tag,
                field.display_value().with_unit(exif)
            );
            output.push_str(&value);
            output.push('\n');

            // Common fields with better formatting
            match field.tag {
                Tag::ImageDescription => {
                    output.push_str(&format!("  Description: {}\n", field.display_value()));
                }
                Tag::Make => {
                    output.push_str(&format!("  Camera Make: {}\n", field.display_value()));
                }
                Tag::Model => {
                    output.push_str(&format!("  Camera Model: {}\n", field.display_value()));
                }
                Tag::DateTime => {
                    output.push_str(&format!("  Date Time: {}\n", field.display_value()));
                }
                Tag::ExposureTime => {
                    output.push_str(&format!("  Exposure: {}\n", field.display_value()));
                }
                Tag::FNumber => {
                    output.push_str(&format!("  Aperture: f/{}\n", field.display_value()));
                }
                Tag::FocalLength => {
                    output.push_str(&format!("  Focal Length: {}mm\n", field.display_value()));
                }
                Tag::IsoSpeedRatings => {
                    output.push_str(&format!("  ISO: {}\n", field.display_value()));
                }
                _ => {}
            }
        }

        output
    }

    pub fn extract_common_metadata(&self, exif: &Exif) -> Vec<(String, String)> {
        let mut metadata = Vec::new();

        for field in exif.fields() {
            match field.tag {
                Tag::ImageDescription
                | Tag::Make
                | Tag::Model
                | Tag::DateTime
                | Tag::ExposureTime
                | Tag::FNumber
                | Tag::FocalLength
                | Tag::IsoSpeedRatings => {
                    metadata.push((
                        field.tag.to_string(),
                        field.display_value().to_string(),
                    ));
                }
                _ => {}
            }
        }

        metadata
    }
}

impl Default for MetadataProcessor {
    fn default() -> Self {
        Self::new()
    }
}