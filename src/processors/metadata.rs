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
        _path: &Path,
    ) -> Result<()> {
        log::debug!("Metadata stripping requested");
        
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
        output.push_str("=== EXIF Metadata ===\n");

        let common_fields = vec![
            (Tag::ImageDescription, "Description"),
            (Tag::Make, "Camera Make"),
            (Tag::Model, "Camera Model"),
            (Tag::DateTime, "Date/Time"),
            (Tag::DateTimeOriginal, "Original Date/Time"),
            (Tag::DateTimeDigitized, "Digitized Date/Time"),
            (Tag::ExposureTime, "Exposure Time"),
            (Tag::FNumber, "Aperture"),
            (Tag::FocalLength, "Focal Length"),
            (Tag::PhotographicSensitivity, "ISO"),
            (Tag::ExposureProgram, "Exposure Program"),
            (Tag::MeteringMode, "Metering Mode"),
            (Tag::Flash, "Flash"),
            (Tag::WhiteBalance, "White Balance"),
            (Tag::Orientation, "Orientation"),
            (Tag::XResolution, "X Resolution"),
            (Tag::YResolution, "Y Resolution"),
            (Tag::Software, "Software"),
            (Tag::Artist, "Artist"),
            (Tag::Copyright, "Copyright"),
            (Tag::GPSLatitude, "GPS Latitude"),
            (Tag::GPSLongitude, "GPS Longitude"),
            (Tag::GPSAltitude, "GPS Altitude"),
        ];

        for field in exif.fields() {
            // Check if this is a common field
            let mut found = false;
            for (tag, label) in &common_fields {
                if field.tag == *tag {
                    let value = field.display_value().with_unit(exif).to_string();
                    output.push_str(&format!("{:25}: {}\n", label, value));
                    found = true;
                    break;
                }
            }
            
            // If not a common field, show it in a general section
            if !found {
                let value = field.display_value().with_unit(exif).to_string();
                output.push_str(&format!("{:25}: {}\n", field.tag.to_string(), value));
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
                | Tag::DateTimeOriginal
                | Tag::DateTimeDigitized
                | Tag::ExposureTime
                | Tag::FNumber
                | Tag::FocalLength
                | Tag::PhotographicSensitivity
                | Tag::ExposureProgram
                | Tag::MeteringMode
                | Tag::Flash
                | Tag::WhiteBalance
                | Tag::Orientation
                | Tag::XResolution
                | Tag::YResolution
                | Tag::Software
                | Tag::Artist
                | Tag::Copyright => {
                    let value = field.display_value().with_unit(exif).to_string();
                    metadata.push((field.tag.to_string(), value));
                }
                _ => {}
            }
        }

        metadata
    }

    pub fn extract_gps_coordinates(&self, exif: &Exif) -> Option<(f64, f64, Option<f64>)> {
        let lat = exif.get_field(Tag::GPSLatitude, In::PRIMARY)?;
        let lat_ref = exif.get_field(Tag::GPSLatitudeRef, In::PRIMARY)?;
        let lon = exif.get_field(Tag::GPSLongitude, In::PRIMARY)?;
        let lon_ref = exif.get_field(Tag::GPSLongitudeRef, In::PRIMARY)?;
        let alt = exif.get_field(Tag::GPSAltitude, In::PRIMARY);
        let alt_ref = exif.get_field(Tag::GPSAltitudeRef, In::PRIMARY);

        let latitude = self.degrees_to_decimal(lat, lat_ref)?;
        let longitude = self.degrees_to_decimal(lon, lon_ref)?;
        let altitude = alt.and_then(|a| {
            let value = a.value.get_rational(0).ok_or_else(|| exif::Error::InvalidFormat("No rational value"))?;
            let mut altitude = value.to_f64();
            
            // Check if altitude is below sea level
            if let Some(ref_field) = alt_ref {
                if ref_field.value.get_uint(0) == Some(1) {
                    altitude = -altitude;
                }
            }
            Some(altitude)
        });

        Some((latitude, longitude, altitude))
    }

    fn degrees_to_decimal(&self, degrees: &exif::Field, ref_field: &exif::Field) -> Option<f64> {
        let components = degrees.value.as_rational()
            .ok_or_else(|| exif::Error::InvalidFormat("Not a rational value"))?;
        if components.len() < 3 {
            return None;
        }

        let deg = components[0].to_f64();
        let min = components[1].to_f64();
        let sec = components[2].to_f64();

        let decimal = deg + (min / 60.0) + (sec / 3600.0);

        // Apply reference (N/S, E/W)
        let ref_value = ref_field.value.get_string()
            .ok_or_else(|| exif::Error::InvalidFormat("Not a string value"))?;
        match ref_value.as_slice() {
            b"S" | b"W" => Some(-decimal),
            _ => Some(decimal),
        }
    }

    pub fn get_camera_info(&self, exif: &Exif) -> Option<(String, String)> {
        let make = exif.get_field(Tag::Make, In::PRIMARY)
            .and_then(|f| {
                let display = f.value.display_as(f.tag);
                Some(format!("{}", display))
            });
        let model = exif.get_field(Tag::Model, In::PRIMARY)
            .and_then(|f| {
                let display = f.value.display_as(f.tag);
                Some(format!("{}", display))
            });

        match (make, model) {
            (Some(m), Some(modl)) => Some((m, modl)),
            _ => None,
        }
    }

    pub fn get_exposure_info(&self, exif: &Exif) -> Option<(String, String, String, String)> {
        let exposure_time = exif.get_field(Tag::ExposureTime, In::PRIMARY)
            pub fn get_camera_info(&self, exif: &Exif) -> Option<(String, String)> {
    let make = exif.get_field(Tag::Make, In::PRIMARY)
        .and_then(|f| {
            let display = f.value.display_as(f.tag);
            Some(format!("{}", display))
        });
    let model = exif.get_field(Tag::Model, In::PRIMARY)
        .and_then(|f| {
            let display = f.value.display_as(f.tag);
            Some(format!("{}", display))
        });

    match (make, model) {
        (Some(m), Some(modl)) => Some((m, modl)),
        _ => None,
    }
}

pub fn get_exposure_info(&self, exif: &Exif) -> Option<(String, String, String, String)> {
    let exposure_time = exif.get_field(Tag::ExposureTime, In::PRIMARY)
        .and_then(|f| {
            let display = f.value.display_as(f.tag);
            Some(format!("{}", display))
        });
    let aperture = exif.get_field(Tag::FNumber, In::PRIMARY)
        .and_then(|f| {
            let display = f.value.display_as(f.tag);
            Some(format!("{}", display))
        });
    let iso = exif.get_field(Tag::PhotographicSensitivity, In::PRIMARY)
        .and_then(|f| {
            let display = f.value.display_as(f.tag);
            Some(format!("{}", display))
        });
    let focal_length = exif.get_field(Tag::FocalLength, In::PRIMARY)
        .and_then(|f| {
            let display = f.value.display_as(f.tag);
            Some(format!("{}", display))
        });

    match (exposure_time, aperture, iso, focal_length) {
        (Some(et), Some(ap), Some(i), Some(fl)) => Some((et, ap, i, fl)),
        _ => None,
    }
}
        let aperture = exif.get_field(Tag::FNumber, In::PRIMARY)
            .and_then(|f| Some(f.value.display_as(f.tag).to_string()));
        let iso = exif.get_field(Tag::PhotographicSensitivity, In::PRIMARY)
            .and_then(|f| Some(f.value.display_as(f.tag).to_string()));
        let focal_length = exif.get_field(Tag::FocalLength, In::PRIMARY)
            .and_then(|f| Some(f.value.display_as(f.tag).to_string()));

        match (exposure_time, aperture, iso, focal_length) {
            (Some(et), Some(ap), Some(i), Some(fl)) => Some((et, ap, i, fl)),
            _ => None,
        }
    }
}

impl Default for MetadataProcessor {
    fn default() -> Self {
        Self::new()
    }
}