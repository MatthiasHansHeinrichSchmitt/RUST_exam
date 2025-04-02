use std::env; //allows to read command-line arguments
use std::fs::File; //opens and reads files
use std::io::{self, BufRead}; //tools to read line by line
use image::{Rgb, RgbImage}; 
use colorgrad; // No need to import `Gradient` directly



fn main() -> io::Result<()> {//I/O error as return of the function

    // Get the file path from command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path_to_asc_file>", args[0]);
        return Ok(());
    }
    let filename = &args[1];

    // Open the ASC file
    let file = File::open(filename)?;//returning an error if it fails to open
    let reader = io::BufReader::new(file);//BufReader allows efficient line-by-line reading

    let mut data: Vec<Vec<f32>> = Vec::new(); //creating a 2D Vector for the data
    let mut ncols = 0; //init ncols
    let mut nrows = 0; //init nrows
    let mut nodata_value = -99999.0; //representing missing/no data in the grid
    let mut reading_data = false;//false as long we are reading the header of the asc file

    // Read the file line by line
    for line in reader.lines() {
        let line = line?; //in case of error giving an error message
        let parts: Vec<&str> = line.split_whitespace().collect();//gathering all elements of the iterator split_whitespace in one vector of references to a string

        if parts.is_empty() {
            continue; // Skip empty lines
        }

        // Parse metadata
        if parts[0].to_lowercase() == "ncols" {
            ncols = parts[1].parse().unwrap_or(0); // Default to 0 if invalid
        } else if parts[0].to_lowercase() == "nrows" {
            nrows = parts[1].parse().unwrap_or(0); // Default to 0 if invalid
        } else if parts[0].to_lowercase() == "nodata_value" {
            nodata_value = parts[1].parse().unwrap_or(-99999.0); // Default to -99999 if invalid
        } else {
            // If we reach the actual data, start parsing rows
            reading_data = true;
        }

        // Read data values after headers
        if reading_data {
            // Ensure we have exactly ncols elements in each row
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

    // Normalize and convert to grayscale image
    let grad = colorgrad::viridis();
    let mut color_img = RgbImage::new(ncols as u32, nrows as u32);

    for (y, row) in data.iter().enumerate() {
        for (x, &val) in row.iter().enumerate() {
            let pixel_value = if val == nodata_value {
                // NoData handling (set as black)
                Rgb([0, 0, 0]) // Black for NoData
            } else {
                // Scale value to the range [0, 255]
                let norm = (val - min_elevation) / (max_elevation - min_elevation);
                let color = grad.at(norm.clamp(0.0, 1.0)as f64);
                let rgba = color.rgba();
                Rgb([
                    (rgba.0 * 255.0) as u8,
                    (rgba.1 * 255.0) as u8,
                    (rgba.2 * 255.0) as u8,
                ])
            };



            color_img.put_pixel(x as u32, y as u32, pixel_value);//visualising the values with Lum
        }
    }

    // Save the output image
    let output_filename = format!("{}_colored.png", filename);
    color_img.save(output_filename).expect("Failed to save image");
    println!("Image saved successfully!");

    Ok(())
}

