use std::fs::{self, File};
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use image::{GrayImage, Luma, Rgb, RgbImage};
use colorgrad;

fn main() -> io::Result<()> {
    let input_dir = "./dataset";       // Directory containing .asc files change this as you like :D
    let output_root = "./output";      // Root output folder for all processed images same for this you can change it

    fs::create_dir_all(output_root)?;  // Ensure the root output folder exists

    // Iterate over all .asc files in dataset
    for entry in fs::read_dir(input_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "asc").unwrap_or(false) {
            let stem = path.file_stem().unwrap().to_string_lossy();
            let output_dir: PathBuf = [output_root, &stem].iter().collect();

            println!("Processing {:?} → Saving to {:?}", path.file_name().unwrap(), output_dir);

            fs::create_dir_all(&output_dir)?; // Create per-file output directory

            // Load and process the .asc file
            match load_asc(&path) {
                Ok((data, ncols, nrows, nodata_value)) => {
                    save_grayscale_image(&data, ncols, nrows, nodata_value, &output_dir)?;
                    save_colored_image(&data, ncols, nrows, nodata_value, &output_dir)?;
                    save_color_hillshade_image(&data, ncols, nrows, nodata_value, &output_dir)?;
                }
                Err(e) => eprintln!("Failed to read {:?}: {}", path, e),
            }
        }
    }

    println!("All files processed successfully!, have a wonderful day :D !");
    Ok(())
}

/// Parses a .asc file into 2D elevation data + metadata
fn load_asc(path: &Path) -> io::Result<(Vec<Vec<f32>>, usize, usize, f32)> {
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    let mut data = Vec::new();
    let mut ncols = 0;
    let mut nrows = 0;
    let mut nodata_value = -99999.0;
    let mut reading_data = false;

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0].to_lowercase().as_str() {
            "ncols" => ncols = parts[1].parse().unwrap_or(0),
            "nrows" => nrows = parts[1].parse().unwrap_or(0),
            "nodata_value" => nodata_value = parts[1].parse().unwrap_or(-99999.0),
            _ if reading_data || parts[0].parse::<f32>().is_ok() => {
                reading_data = true;
                let row: Vec<f32> = parts.iter().map(|&x| x.parse().unwrap_or(nodata_value)).collect();
                if row.len() == ncols { data.push(row); }
            }
            _ => {}
        }
    }

    if data.len() != nrows {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Row count mismatch"));
    }

    Ok((data, ncols, nrows, nodata_value))
}

/// Finds min and max elevation values, ignoring NoData
fn find_min_max(data: &[Vec<f32>], nodata: f32) -> (f32, f32) {
    let mut min = f32::MAX;
    let mut max = f32::MIN;
    for row in data {
        for &val in row {
            if val != nodata {
                min = min.min(val);
                max = max.max(val);
            }
        }
    }
    (min, max)
}

/// Generates and saves grayscale elevation image to <output_dir>/grayscale.png
fn save_grayscale_image(data: &[Vec<f32>], ncols: usize, nrows: usize, nodata: f32, output_dir: &Path) -> io::Result<()> {
    let (min, max) = find_min_max(data, nodata);
    let mut img = GrayImage::new(ncols as u32, nrows as u32);

    for (y, row) in data.iter().enumerate() {
        for (x, &val) in row.iter().enumerate() {
            let pixel = if val == nodata {
                0
            } else {
                ((val - min) / (max - min) * 255.0).clamp(0.0, 255.0) as u8
            };
            img.put_pixel(x as u32, y as u32, Luma([pixel]));
        }
    }

    let output_path = output_dir.join("grayscale.png");
    img.save(output_path).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    Ok(())
}

/// Generates and saves color-mapped elevation image to <output_dir>/colored.png
fn save_colored_image(data: &[Vec<f32>], ncols: usize, nrows: usize, nodata: f32, output_dir: &Path) -> io::Result<()> {
    let (min, max) = find_min_max(data, nodata);
    let grad = colorgrad::viridis();
    let mut img = RgbImage::new(ncols as u32, nrows as u32);

    for (y, row) in data.iter().enumerate() {
        for (x, &val) in row.iter().enumerate() {
            let rgb = if val == nodata {
                Rgb([0, 0, 0])
            } else {
                let norm = (val - min) / (max - min);
                let (r, g, b, _) = grad.at(norm.clamp(0.0, 1.0) as f64).rgba();
                Rgb([(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8])
            };
            img.put_pixel(x as u32, y as u32, rgb);
        }
    }

    let output_path = output_dir.join("colored.png");
    img.save(output_path).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    Ok(())
}

/// Generates and saves hillshaded color image to <output_dir>/hillside.png
fn save_color_hillshade_image(data: &[Vec<f32>], ncols: usize, nrows: usize, nodata: f32, output_dir: &Path) -> io::Result<()> {
    let (min, max) = find_min_max(data, nodata);
    let grad = colorgrad::viridis();
    let mut img = RgbImage::new(ncols as u32, nrows as u32);

    let elev_f64: Vec<Vec<f64>> = data.iter().map(|row| row.iter().map(|&x| x as f64).collect()).collect();
    let cell_size = 30.0;
    let z_factor = 1.0;
    let azimuth = 315.0;
    let altitude = 45.0;

    for y in 0..nrows {
        for x in 0..ncols {
            let val = data[y][x];
            let base_color = if val == nodata {
                Rgb([0, 0, 0])
            } else {
                let norm = (val - min) / (max - min);
                let (r, g, b, _) = grad.at(norm.clamp(0.0, 1.0) as f64).rgba();
                Rgb([(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8])
            };

            let shade = if val == nodata {
                0
            } else {
                calculate_hillshade(&elev_f64, x, y, cell_size, z_factor, azimuth, altitude, nodata as f64)
            };

            let factor = shade as f32 / 255.0;
            let shaded = Rgb([
                (base_color[0] as f32 * (1.0 - factor)) as u8,
                (base_color[1] as f32 * (1.0 - factor)) as u8,
                (base_color[2] as f32 * (1.0 - factor)) as u8,
            ]);

            img.put_pixel(x as u32, y as u32, shaded);
        }
    }

    let output_path = output_dir.join("hillside.png");
    img.save(output_path).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    Ok(())
}

/// Computes hillshade value at a given cell using Horn’s method
fn calculate_hillshade(elevation: &[Vec<f64>], x: usize, y: usize, cell_size: f64, zf: f64, az: f64, alt: f64, nodata: f64) -> u8 {
    let get = |dx: isize, dy: isize| -> f64 {
        let nx = x as isize + dx;
        let ny = y as isize + dy;
        if nx >= 0 && ny >= 0 && (nx as usize) < elevation[0].len() && (ny as usize) < elevation.len() {
            elevation[ny as usize][nx as usize]
        } else {
            nodata
        }
    };

    let dzdx = ((get(1, -1) + 2.0 * get(1, 0) + get(1, 1)) -
                (get(-1, -1) + 2.0 * get(-1, 0) + get(-1, 1))) / (8.0 * cell_size) * zf;
    let dzdy = ((get(-1, 1) + 2.0 * get(0, 1) + get(1, 1)) -
                (get(-1, -1) + 2.0 * get(0, -1) + get(1, -1))) / (8.0 * cell_size) * zf;

    let slope = (dzdx.powi(2) + dzdy.powi(2)).sqrt().atan();
    let aspect = if dzdx != 0.0 {
        let mut a = (dzdy / dzdx).atan();
        if dzdx < 0.0 { a += std::f64::consts::PI; }
        else if dzdy < 0.0 { a += 2.0 * std::f64::consts::PI; }
        a
    } else {
        if dzdy > 0.0 { std::f64::consts::FRAC_PI_2 } else { 3.0 * std::f64::consts::FRAC_PI_2 }
    };

    let az_rad = az.to_radians();
    let alt_rad = alt.to_radians();
    let shade = 255.0 * ((alt_rad.sin() * slope.sin()) + (alt_rad.cos() * slope.cos() * (az_rad - aspect).cos()));

    shade.clamp(0.0, 255.0) as u8
}
