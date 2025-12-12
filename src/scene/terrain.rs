//! Terrain generation and lunar DEM loading

use std::fs::File;
use std::io::BufReader;

/// Lunar Digital Elevation Model loader and sampler
pub struct LunarDEM {
    pub width: u32,
    pub height: u32,
    pub data: Vec<f32>, // Elevation data in meters
}

impl LunarDEM {
    /// Load a lunar DEM from a TIFF file
    pub fn load(path: &str) -> Option<Self> {
        use tiff::decoder::{Decoder, DecodingResult};

        let file = File::open(path).ok()?;
        let reader = BufReader::new(file);
        let mut decoder = Decoder::new(reader).ok()?;

        let (width, height) = decoder.dimensions().ok()?;

        // Read the image data
        let result = decoder.read_image().ok()?;

        // Convert to f32 vec
        let data = match result {
            DecodingResult::F32(vec) => vec,
            DecodingResult::F64(vec) => vec.iter().map(|&v| v as f32).collect(),
            _ => {
                println!("Warning: TIFF is not in float format, attempting conversion");
                return None;
            }
        };

        println!(
            "Loaded lunar DEM: {}x{} pixels, {} data points",
            width,
            height,
            data.len()
        );

        Some(Self { width, height, data })
    }

    /// Sample the DEM with latitude correction for rectangular projection
    /// 
    /// This accounts for meridian convergence at higher latitudes to prevent
    /// crater stretching near the poles.
    pub fn sample_lat_corrected(
        &self,
        center_lon_deg: f32,
        center_lat_deg: f32,
        angular_extent_deg: f32,
        output_width: usize,
        output_height: usize,
    ) -> Vec<f32> {
        use std::f32::consts::PI;

        let mut samples = Vec::with_capacity(output_width * output_height);

        // Convert to radians
        let center_lon = center_lon_deg * PI / 180.0;
        let center_lat = center_lat_deg * PI / 180.0;
        let angular_extent = angular_extent_deg * PI / 180.0;

        // For each output pixel, sample with latitude-corrected coordinates
        for y in 0..output_height {
            for x in 0..output_width {
                // Normalize to -0.5 to 0.5 range
                let nx = (x as f32 / output_width as f32) - 0.5;
                let ny = (y as f32 / output_height as f32) - 0.5;

                // Calculate latitude for this row (vertical position)
                let lat = center_lat + (ny * angular_extent);

                // Compress horizontal extent by cos(latitude) to account for meridian convergence
                // This makes craters circular instead of stretched
                let lat_scale = lat.cos().max(0.1); // Clamp to avoid division by zero at poles
                let lon = center_lon + (nx * angular_extent / lat_scale);

                // Convert lat/lon to texture coordinates (0 to 1)
                // Longitude wraps around, latitude clamps
                let tex_x = ((lon / PI + 1.0) / 2.0).rem_euclid(1.0);
                let tex_y = (-lat / PI + 0.5).clamp(0.0, 1.0);

                // Sample from the DEM
                let px = (tex_x * self.width as f32) as u32;
                let py = (tex_y * self.height as f32) as u32;
                let px = px.min(self.width - 1);
                let py = py.min(self.height - 1);

                let idx = (py * self.width + px) as usize;
                samples.push(self.data[idx]);
            }
        }

        samples
    }

    /// Normalize elevation samples and scale to a target height range
    pub fn normalize_and_scale(
        samples: &[f32],
        target_width: usize,
        target_height: usize,
        scale_factor: f32,
    ) -> Vec<f32> {
        let sample_width = (samples.len() as f32).sqrt() as usize;

        // Find min/max for normalization
        let min_elev = samples.iter().cloned().fold(f32::INFINITY, f32::min);
        let max_elev = samples.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let range = max_elev - min_elev;

        println!(
            "Lunar elevation range: {} to {} meters (range: {} m)",
            min_elev, max_elev, range
        );

        let mut result = Vec::with_capacity(target_width * target_height);

        for y in 0..target_height {
            for x in 0..target_width {
                // Use nearest neighbor sampling to preserve sharp crater features
                let src_x = ((x as f32 / target_width as f32) * sample_width as f32) as usize;
                let src_y = ((y as f32 / target_height as f32) * sample_width as f32) as usize;

                let src_x = src_x.min(sample_width - 1);
                let src_y = src_y.min(sample_width - 1);

                let elevation = samples[src_y * sample_width + src_x];

                // Normalize to 0-1 and scale
                let normalized = if range > 0.0 {
                    ((elevation - min_elev) / range) * scale_factor
                } else {
                    0.0
                };

                result.push(normalized);
            }
        }

        result
    }
}