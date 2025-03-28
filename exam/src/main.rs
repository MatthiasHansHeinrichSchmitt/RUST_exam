use std::env;
use std::fs::File;
use std::io::{self, BufRead};
use image::{Rgb, RgbImage, Luma, GrayImage};
use colorgrad;

fn main() -> io::Result<()> {

    // Get the file path from command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path_to_asc_file>", args[0]);
        return Ok(());
    }
    let filename = &args[1];

    // Open the ASC file
    let file = File::open(filename)?;
    let reader = io::BufReader::new(file);

    let mut data: Vec<Vec<f32>> = Vec::new(); // 2D Vector for data
    let mut ncols = 0; // Number of columns
    let mut nrows = 0; // Number of rows
    let mut nodata_value = -99999.0; // NoData value
    let mut reading_data = false;

    // Read the file line by line
    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.is_empty() {
            continue;
        }

        // Parse metadata
        if parts[0].to_lowercase() == "ncols" {
            ncols = parts[1].parse().unwrap_or(0);
        } else if parts[0].to_lowercase() == "nrows" {
            nrows = parts[1].parse().unwrap_or(0);
        } else if parts[0].to_lowercase() == "nodata_value" {
            nodata_value = parts[1].parse().unwrap_or(-99999.0);
        } else {
            // If we reach data, start parsing rows
            reading_data = true;
        }

        // Read data values after headers
        if reading_data {
            let row: Vec<f32> = parts.iter().map(|&x| x.parse().unwrap_or(nodata_value)).collect();
            if row.len() == ncols as usize {
                data.push(row);
            } else {
                eprintln!("Warning: A row does not match the expected column count. Skipping it.");
            }
        }
    }

    // Validate the number of rows
    if data.len() != nrows as usize {
        eprintln!("Error: Expected {} rows, but found {} rows.", nrows, data.len());
        return Ok(());
    }

    // Find min and max elevation values
    let mut min_elevation = f32::MAX;
    let mut max_elevation = f32::MIN;

    for row in &data {
        for &val in row {
            if val != nodata_value {
                if val < min_elevation {
                    min_elevation = val;
                }
                if val > max_elevation {
                    max_elevation = val;
                }
            }
        }
    }

    // Create color gradient
    let grad = colorgrad::viridis();
    let mut color_img = RgbImage::new(ncols as u32, nrows as u32);

    // Placeholder for hillshade data
    let mut hillshade_image = GrayImage::new(ncols as u32, nrows as u32);
    
    // Hillshade parameters (same as before)
    let cell_size = 30.0;
    let z_factor = 1.0;
    let azimuth = 315.0;
    let altitude = 45.0;

    // Calculate hillshade values
    for (y, row) in data.iter().enumerate() {
        for (x, &val) in row.iter().enumerate() {
            if val == nodata_value {
                hillshade_image.put_pixel(x as u32, y as u32, Luma([0])); // Black for NoData in hillshade
            } else {
                let shade = calculate_hillshade(
                    &data.iter().map(|row| row.iter().map(|&val| val as f64).collect::<Vec<f64>>()).collect::<Vec<Vec<f64>>>(), 
                    x, y, 
                    cell_size, 
                    z_factor, 
                    azimuth, 
                    altitude, 
                    nodata_value as f64
                );
                hillshade_image.put_pixel(x as u32, y as u32, Luma([shade]));
            }
        }
    }

    // Combine color map with hillshade
    for (y, row) in data.iter().enumerate() {
        for (x, &val) in row.iter().enumerate() {
            let pixel_value = if val == nodata_value {
                // Set NoData values as black
                Rgb([0, 0, 0]) // Black for NoData (or transparent if you prefer)
            } else {
                // Scale elevation value to the range [0, 255]
                let norm = (val - min_elevation) / (max_elevation - min_elevation);
                let color = grad.at(norm.clamp(0.0, 1.0) as f64);
                let rgba = color.rgba();
                let mut color = Rgb([
                    (rgba.0 * 255.0) as u8,
                    (rgba.1 * 255.0) as u8,
                    (rgba.2 * 255.0) as u8,
                ]);

                // Get the hillshade value for the current pixel
                let hillshade_value = hillshade_image.get_pixel(x as u32, y as u32).0[0];
                
                // Adjust the color based on the hillshade (darken or lighten)
                let factor = hillshade_value as f32 / 255.0; // hillshade value between 0 and 1
                color.0[0] = (color.0[0] as f32 * (1.0 - factor)) as u8;
                color.0[1] = (color.0[1] as f32 * (1.0 - factor)) as u8;
                color.0[2] = (color.0[2] as f32 * (1.0 - factor)) as u8;

                color
            };

            // Set the pixel in the final image
            color_img.put_pixel(x as u32, y as u32, pixel_value);
        }
    }

    // Save the output image
    let output_filename = format!("{}_colored_hillshade_map.png", filename);
    color_img.save(output_filename).expect("Failed to save image");
    println!("Image with hillshade saved successfully!");

    Ok(())
}

// Hillshade calculation function (same as before)
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
            aspect += std::f64::consts::PI;
        } else if dzdy < 0.0 {
            aspect += 2.0 * std::f64::consts::PI;
        }
        aspect
    } else {
        if dzdy > 0.0 { std::f64::consts::PI / 2.0 } else { 3.0 * std::f64::consts::PI / 2.0 }
    };

    let azimuth_rad = azimuth.to_radians();
    let altitude_rad = altitude.to_radians();

    let hillshade = 255.0 * ((altitude_rad.sin() * slope.sin()) +
                             (altitude_rad.cos() * slope.cos() * (azimuth_rad - aspect).cos()));

    hillshade.max(0.0).min(255.0) as u8
}
