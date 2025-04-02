use image::{GrayImage, Luma, Rgb, RgbImage}; // Image types
use std::env; // For reading command-line arguments
use std::fs::{self, File}; // For reading files and directories
use std::io::{self, BufRead}; // Buffered reader for line-by-line reading
use std::path::{Path, PathBuf}; // Path utilities

// Convert grayscale to a basic RGB gradient (optional; not used here but useful for extension)
fn gray_to_color_gradient(gray_image: &GrayImage) -> RgbImage {
    let (width, height) = gray_image.dimensions();
    let mut color_image = RgbImage::new(width, height);

    for (x, y, gray_pixel) in gray_image.enumerate_pixels() {
        let gray_value = gray_pixel[0];
        let r = gray_value;
        let g = 255 - gray_value;
        let b = (gray_value / 2) as u8;
        color_image.put_pixel(x, y, Rgb([r, g, b]));
    }

    color_image
}

fn main() -> io::Result<()> {
    let input_dir = "./dataset"; // Directory with .asc files
    let output_dir = "./images/grayscale"; // Grayscale image output directory

    // Create output directory if it doesn't exist
    if !Path::new(output_dir).exists() {
        fs::create_dir_all(output_dir)?;
    }

    // Loop through all .asc files
    for entry in fs::read_dir(input_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|ext| ext == "asc").unwrap_or(false) {
            println!("Processing {:?}", path.file_name().unwrap());

            if let Err(e) = process_asc_to_grayscale(&path, output_dir) {
                eprintln!("Failed to process {:?}: {}", path.file_name().unwrap(), e);
            }
        }
    }

    println!("All files processed!");
    Ok(())
}

// Converts a single .asc file to a grayscale image
fn process_asc_to_grayscale(path: &Path, output_dir: &str) -> io::Result<()> {
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    let mut data: Vec<Vec<f32>> = Vec::new();
    let mut ncols = 0;
    let mut nrows = 0;
    let mut nodata_value = -99999.0;
    let mut reading_data = false;

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.is_empty() {
            continue;
        }

        // Read header or elevation data
        if parts[0].to_lowercase() == "ncols" 
        {
            ncols = parts[1].parse().unwrap_or(0);
        } 
        else if parts[0].to_lowercase() == "nrows" 
        {
            nrows = parts[1].parse().unwrap_or(0);
        } 
        else if parts[0].to_lowercase() == "nodata_value" {
            nodata_value = parts[1].parse().unwrap_or(-99999.0);
        } 
        else 
        {
            reading_data = true;
        }

        if reading_data {
            let row: Vec<f32> = parts.iter().map(|&x| x.parse().unwrap_or(nodata_value)).collect();
            if row.len() == ncols {
                data.push(row);
            } else {
                eprintln!("Warning: row length mismatch, skipping row.");
            }
        }
    }

    // Validate row count
    if data.len() != nrows {
        eprintln!("Error: expected {} rows, but got {}", nrows, data.len());
        return Ok(());
    }

    // Find min/max elevation
    let mut min_elevation = f32::MAX;
    let mut max_elevation = f32::MIN;
    for row in &data {
        for &val in row {
            if val != nodata_value {
                if val < min_elevation { min_elevation = val; }
                if val > max_elevation { max_elevation = val; }
            }
        }
    }

    // Create grayscale image
    let mut img = GrayImage::new(ncols as u32, nrows as u32);

    for (y, row) in data.iter().enumerate() {
        for (x, &val) in row.iter().enumerate() {
            let pixel_value = if val == nodata_value {
                0 // Black for NoData
            } else {
                let scaled = ((val - min_elevation) / (max_elevation - min_elevation)) * 255.0;
                scaled.clamp(0.0, 255.0) as u8
            };
            img.put_pixel(x as u32, y as u32, Luma([pixel_value]));
        }
    }

    // Save the image in grayscale output folder
    let filename = path.file_stem().unwrap().to_string_lossy();
    let output_path: PathBuf = [output_dir, &format!("{}_grayscale.png", filename)].iter().collect();
    img.save(output_path).expect("Failed to save grayscale image");
    println!("Saved: {}", filename);

    Ok(())
}
