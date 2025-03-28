use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use image::{GrayImage, Luma};
use std::f64::consts::PI;

/// Reads an ASCII raster file and returns the elevation data along with metadata.
fn read_asc_file(filename: &str) -> io::Result<(Vec<Vec<f64>>, usize, usize, f64)> {
    let file = File::open(filename)?;
    let reader = BufReader::new(file);

    let mut ncols = 0;
    let mut nrows = 0;
    let mut nodata_value = -9999.0;
    let mut data = Vec::new();
    let mut reading_data = false;

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.is_empty() {
            continue;
        }

        if !reading_data {
            match parts[0].to_lowercase().as_str() {
                "ncols" => ncols = parts[1].parse().unwrap_or(0),
                "nrows" => nrows = parts[1].parse().unwrap_or(0),
                "nodata_value" => nodata_value = parts[1].parse().unwrap_or(-9999.0),
                _ => reading_data = true,
            }
        }

        if reading_data {
            let row: Vec<f64> = parts.iter()
                .map(|&x| x.parse().unwrap_or(nodata_value))
                .collect();
            if row.len() == ncols {
                data.push(row);
            } else {
                eprintln!("Warning: A row does not match the expected column count. Skipping it.");
            }
        }
    }

    if data.len() != nrows {
        eprintln!("Error: Expected {} rows, but found {} rows.", nrows, data.len());
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Row count mismatch"));
    }

    Ok((data, ncols, nrows, nodata_value))
}

/// Calculates the hillshade value for a given cell using the ArcGIS method.
fn calculate_hillshade(elevation_data: &Vec<Vec<f64>>, x: usize, y: usize, cell_size: f64, z_factor: f64, azimuth: f64, altitude: f64, nodata_value: f64) -> u8 {
    let get_elevation = |dx: isize, dy: isize| -> f64 {
        let nx = x as isize + dx;
        let ny = y as isize + dy;
        if nx >= 0 && ny >= 0 && (nx as usize) < elevation_data[0].len() && (ny as usize) < elevation_data.len() {
            elevation_data[ny as usize][nx as usize]
        } else {
            nodata_value
        }
    };

    // Apply Z-Factor scaling
    let dzdx = ((get_elevation(1, -1) + 2.0 * get_elevation(1, 0) + get_elevation(1, 1)) -
                (get_elevation(-1, -1) + 2.0 * get_elevation(-1, 0) + get_elevation(-1, 1))) /
               (8.0 * cell_size) * z_factor;

    let dzdy = ((get_elevation(-1, 1) + 2.0 * get_elevation(0, 1) + get_elevation(1, 1)) -
                (get_elevation(-1, -1) + 2.0 * get_elevation(0, -1) + get_elevation(1, -1))) /
               (8.0 * cell_size) * z_factor;

    let slope = (dzdx.powi(2) + dzdy.powi(2)).sqrt().atan();
    let aspect = if dzdx != 0.0 {
        let mut aspect = (dzdy / dzdx).atan();
        if dzdx < 0.0 {
            aspect += PI;
        } else if dzdy < 0.0 {
            aspect += 2.0 * PI;
        }
        aspect
    } else {
        if dzdy > 0.0 { PI / 2.0 } else { 3.0 * PI / 2.0 }
    };

    let azimuth_rad = azimuth.to_radians();
    let altitude_rad = altitude.to_radians();

    let hillshade = 255.0 * ((altitude_rad.sin() * slope.sin()) +
                             (altitude_rad.cos() * slope.cos() * (azimuth_rad - aspect).cos()));

    hillshade.max(0.0).min(255.0) as u8
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path_to_asc_file>", args[0]);
        return Ok(());
    }
    let filename = &args[1];

    let (elevation_data, ncols, nrows, nodata_value) = read_asc_file(filename)?;

    let cell_size = 30.0; // Adjust based on your data's resolution
    let z_factor = 1.0;   // Adjust to scale elevation values appropriately
    let azimuth = 315.0;  // Sun's direction in degrees
    let altitude = 45.0;  // Sun's angle above the horizon

    let mut hillshade_image = GrayImage::new(ncols as u32, nrows as u32);

    for y in 0..nrows {
        for x in 0..ncols {
            if elevation_data[y][x] == nodata_value {
                hillshade_image.put_pixel(x as u32, y as u32, Luma([0]));
            } else {
                let shade = calculate_hillshade(&elevation_data, x, y, cell_size, z_factor, azimuth, altitude, nodata_value);
                hillshade_image.put_pixel(x as u32, y as u32, Luma([shade]));
            }
        }
    }

    let output_filename = format!("{}_hillshade.png", filename);
    
    if let Err(e) = hillshade_image.save(&output_filename) {
        eprintln!("Failed to save image: {}", e);
        return Ok(()); 
    }

    println!("Hillshade image saved successfully!");

    Ok(())
}
