use std::fs;
use image::{RgbImage, GrayImage, Luma, Rgb};
use colorgrad::Gradient;

struct Header {
    ncols: usize,
    nrows: usize,
    xllcorner: f64,
    yllcorner: f64,
    cellsize: f64,
    nodata: f64,
}

fn calculate_slope(grid: &Vec<Vec<f64>>, x: usize, y: usize, cellsize: f64, nodata: f64) -> Option<f64> {
    let rows = grid.len();
    let cols = grid[0].len();

    if x == 0 || y == 0 || x >= cols - 1 || y >= rows - 1 {
        return None;
    }

    let get = |x: usize, y: usize| -> Option<f64> {
        let val: f64 = grid[y][x];  // 👈 force type to fix inference
        if val == nodata {
            None
        } else {
            Some(val)
        }
    };
    

    let dzdx = ((get(x + 1, y - 1)? + 2.0 * get(x + 1, y)? + get(x + 1, y + 1)?) -
                (get(x - 1, y - 1)? + 2.0 * get(x - 1, y)? + get(x - 1, y + 1)?)) / (8.0 * cellsize);

    let dzdy = ((get(x - 1, y + 1)? + 2.0 * get(x, y + 1)? + get(x + 1, y + 1)?) -
                (get(x - 1, y - 1)? + 2.0 * get(x, y - 1)? + get(x + 1, y - 1)?)) / (8.0 * cellsize);

    let slope = (dzdx.powi(2) + dzdy.powi(2)).sqrt();
    Some(slope)
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
    for line in data_lines {
        let row: Vec<f64> = line.split_whitespace().map(|val| val.parse::<f64>().unwrap()).collect();
        grid.push(row);
    }

    // Find min/max elevation (excluding nodata)
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

    // COLOR GRADIENT IMAGE
    let grad = colorgrad::viridis();
    let mut color_img = RgbImage::new(header.ncols as u32, header.nrows as u32);
    for (y, row) in grid.iter().enumerate() {
        for (x, &val) in row.iter().enumerate() {
            let pixel = if val == header.nodata {
                Rgb([0, 0, 0])
            } else {
                let norm = (val - min) / (max - min);
                let color = grad.at(norm.clamp(0.0, 1.0));
                Rgb([
                    (color.r * 255.0) as u8,
                    (color.g * 255.0) as u8,
                    (color.b * 255.0) as u8,
                ])
            };
            color_img.put_pixel(x as u32, y as u32, pixel);
        }
    }
    color_img.save("dem_colored.png").expect("Failed to save colored image");

    // HILLSHADE IMAGE
    let mut shade_img = GrayImage::new(header.ncols as u32, header.nrows as u32);
    for y in 0..header.nrows {
        for x in 0..header.ncols {
            let brightness = match calculate_slope(&grid, x, y, header.cellsize, header.nodata) {
                Some(slope) => {
                    let brightness = 255.0 - (slope * 255.0).min(255.0);
                    brightness as u8
                }
                None => 0,
            };
            shade_img.put_pixel(x as u32, y as u32, Luma([brightness]));
        }
    }
    shade_img.save("dem_hillshade.png").expect("Failed to save hillshade image");

    println!("DEM size: {} x {}", header.ncols, header.nrows);
    println!("Colored and hillshaded images saved!");
}
