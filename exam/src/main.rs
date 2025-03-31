use std::fs::{self, File};
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use image::{RgbImage, Rgb, GrayImage, Luma};
use colorgrad;

fn main() -> io::Result<()> {
    let input_dir = "./dataset"; // Folder containing .asc files
    let output_dir = "./images";   // Folder for output images

    // Create the output folder if it doesn't exist
    if !Path::new(output_dir).exists() {
        fs::create_dir_all(output_dir)?;
    }

    // Process all .asc files
    for entry in fs::read_dir(input_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "asc").unwrap_or(false) {
            println!("Processing {:?}", path.file_name().unwrap());

            if let Err(e) = process_file(&path, output_dir) {
                eprintln!("Failed to process {:?}: {}", path.file_name().unwrap(), e);
            }
        }
    }

    println!("All files processed!");
    Ok(())
}

fn process_file(path: &Path, output_dir: &str) -> io::Result<()> {
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
        return Err(io::Error::new(io::ErrorKind::Other, "Row count mismatch"));
    }

    let (min_elevation, max_elevation) = find_min_max(&data, nodata_value);
    let grad = colorgrad::viridis();
    let mut color_img = RgbImage::new(ncols as u32, nrows as u32);
    let mut hillshade_image = GrayImage::new(ncols as u32, nrows as u32);

    let cell_size = 30.0;
    let z_factor = 1.0;
    let azimuth = 315.0;
    let altitude = 45.0;

    let elev_f64: Vec<Vec<f64>> = data.iter()
        .map(|row| row.iter().map(|&val| val as f64).collect())
        .collect();

    for y in 0..nrows {
        for x in 0..ncols {
            let val = data[y][x];
            let shade = if val == nodata_value {
                0
            } else {
                calculate_hillshade(&elev_f64, x, y, cell_size, z_factor, azimuth, altitude, nodata_value as f64)
            };
            hillshade_image.put_pixel(x as u32, y as u32, Luma([shade]));
        }
    }

    for y in 0..nrows {
        for x in 0..ncols {
            let val = data[y][x];
            let color = if val == nodata_value {
                Rgb([0, 0, 0])
            } else {
                let norm = (val - min_elevation) / (max_elevation - min_elevation);
                let color = grad.at(norm.clamp(0.0, 1.0) as f64).rgba();
                let mut rgb = Rgb([
                    (color.0 * 255.0) as u8,
                    (color.1 * 255.0) as u8,
                    (color.2 * 255.0) as u8,
                ]);
                let shade = hillshade_image.get_pixel(x as u32, y as u32).0[0];
                let factor = shade as f32 / 255.0;
                for c in &mut rgb.0 {
                    *c = (*c as f32 * (1.0 - factor)) as u8;
                }
                rgb
            };
            color_img.put_pixel(x as u32, y as u32, color);
        }
    }

    let filename = path.file_stem().unwrap().to_string_lossy();
    let output_path: PathBuf = [output_dir, &format!("{}.png", filename)].iter().collect();
    color_img.save(output_path).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    Ok(())
}

fn find_min_max(data: &Vec<Vec<f32>>, nodata: f32) -> (f32, f32) {
    let mut min = f32::MAX;
    let mut max = f32::MIN;
    for row in data {
        for &val in row {
            if val != nodata {
                if val < min { min = val; }
                if val > max { max = val; }
            }
        }
    }
    (min, max)
}

fn calculate_hillshade(elevation: &Vec<Vec<f64>>, x: usize, y: usize, cell_size: f64, zf: f64, az: f64, alt: f64, nodata: f64) -> u8 {
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
        let mut aspect = (dzdy / dzdx).atan();
        if dzdx < 0.0 { aspect += std::f64::consts::PI; }
        else if dzdy < 0.0 { aspect += 2.0 * std::f64::consts::PI; }
        aspect
    } else {
        if dzdy > 0.0 { std::f64::consts::FRAC_PI_2 } else { 3.0 * std::f64::consts::FRAC_PI_2 }
    };

    let az_rad = az.to_radians();
    let alt_rad = alt.to_radians();

    let shade = 255.0 * ((alt_rad.sin() * slope.sin()) +
                         (alt_rad.cos() * slope.cos() * (az_rad - aspect).cos()));

    shade.clamp(0.0, 255.0) as u8
}
