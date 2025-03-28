use image::{GrayImage, Luma}; //creating and saving grayscale images
use std::env; //allows to read command-line arguments
use std::fs::File; //opens and reads files
use std::io::{self, BufRead}; //tools to read line by line
use image::{GrayImage, Rgb, RgbImage};

fn gray_to_color_gradient(gray_image: &GrayImage) -> RgbImage {
    let (width, height) = gray_image.dimensions();
    let mut color_image = RgbImage::new(width, height);

    for (x, y, gray_pixel) in gray_image.enumerate_pixels() {
        let gray_value = gray_pixel[0];  // Grayscale value (0-255)

        // Map gray value to a color gradient.
        let r = gray_value;      // Red channel (0-255)
        let g = 255 - gray_value; // Green channel (inverted for contrast)
        let b = (gray_value / 2) as u8; // Blue channel (half the grayscale value)

        // Set the pixel in the color image.
        color_image.put_pixel(x, y, Rgb([r, g, b]));
    }

    color_image
}

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
    let mut img = GrayImage::new(ncols as u32, nrows as u32);//using GrayImage of size ncolsxnrows

    for (y, row) in data.iter().enumerate() {
        for (x, &val) in row.iter().enumerate() {
            let pixel_value = if val == nodata_value {
                // NoData handling (set as black)
                0 // Black for NoData
            } else {
                // Scale value to the range [0, 255]
                let scaled_value = (((val - min_elevation) / (max_elevation - min_elevation)) * 255.0)
                    .min(255.0)
                    .max(0.0) as u8;//ensuring the limits of 255 and 0

                scaled_value
            };



            img.put_pixel(x as u32, y as u32, Luma([pixel_value]));//visualising the values with Lum
        }
    }

    // Save the output image
    let output_filename = format!("{}_grayscale.png", filename);
    img.save(output_filename).expect("Failed to save image");
    println!("Image saved successfully!");

    Ok(())
}

