use std::fs;
use image::{GrayImage, Luma};


struct Header {
    ncols: usize,
    nrows: usize,
    xllcorner: f64,
    yllcorner: f64,
    cellsize: f64,
    nodata: f64,
}

fn main() {
    let path: &str = "dataset/LITTO3D_FRA_0927_6223_MNT_20150529_LAMB93_RGF93_IGN69.asc";
    let content = fs::read_to_string(path).unwrap();

    let lines: Vec<&str> = content.lines().collect();

    let header = Header {
        ncols: lines[0].split_whitespace().nth(1).unwrap().parse().unwrap(),
        nrows: lines[1].split_whitespace().nth(1).unwrap().parse().unwrap(),
        xllcorner: lines[2].split_whitespace().nth(1).unwrap().parse().unwrap(),
        yllcorner: lines[3].split_whitespace().nth(1).unwrap().parse().unwrap(),
        cellsize: lines[4].split_whitespace().nth(1).unwrap().parse().unwrap(),
        nodata: lines[5].split_whitespace().nth(1).unwrap().parse().unwrap(),
    };

    let data_lines = &lines[6..];

    let mut grid: Vec<Vec<f64>> = Vec::new();

    for line in data_lines{
        let row: Vec<f64> = line.split_whitespace().map(|val|val.parse::<f64>().unwrap()).collect();
        grid.push(row);
    }

    // First, find min and max (excluding nodata)
    let mut min = f64::MAX;
    let mut max = f64::MIN;

    for row in &grid {
        for &val in row {
            if val != header.nodata {
                if val < min { min = val; }
                if val > max { max = val; }
            }
        }
    }

    // Create image
    let mut imgbuf = GrayImage::new(header.ncols as u32, header.nrows as u32);

    // Fill it with pixels
    for (y, row) in grid.iter().enumerate() {
        for (x, &val) in row.iter().enumerate() {
            let pixel_val = if val == header.nodata {
                0 // Black for nodata
            } else {
                // Normalize to 0–255
                let norm = ((val - min) / (max - min)) * 255.0;
                norm as u8
            };
            imgbuf.put_pixel(x as u32, y as u32, Luma([pixel_val]));
        }
    }

    // Save image
    imgbuf.save("dem_output.png").expect("Failed to save image");

        
    println!("DEM size: {} x {}", header.ncols, header.nrows);
    println!("Cell size: {}", header.cellsize);
    println!("No-data value: {}", header.nodata);

    println!("yipeeeeeee!");
}
