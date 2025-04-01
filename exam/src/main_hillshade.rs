use std::fs::{self, File};
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use image::{Rgb, RgbImage};
use colorgrad;

fn main() -> io::Result<()> {
    let input_dir = "./dataset";
    let output_root = "./output";

    fs::create_dir_all(output_root)?;

    for entry in fs::read_dir(input_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "asc").unwrap_or(false) {
            let stem = path.file_stem().unwrap().to_string_lossy();
            let output_dir: PathBuf = [output_root, &stem].iter().collect();
            fs::create_dir_all(&output_dir)?;

            println!("Rendering hillshade for {:?}", stem);

            if let Ok((data, ncols, nrows, nodata)) = load_asc(&path) {
                render_hillshade_image(&data, ncols, nrows, nodata, &output_dir)?;
            } else {
                eprintln!("Failed to process file {:?}", path);
            }
        }
    }

    println!("Hillshading complete.");
    Ok(())
}

/// Reads an .asc file and returns elevation data and metadata
fn load_asc(path: &Path) -> io::Result<(Vec<Vec<f32>>, usize, usize, f32)> {
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    let mut data = Vec::new();
    let mut ncols = 0;
    let mut nrows = 0;
    let mut nodata = -99999.0;
    let mut reading_data = false;

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0].to_lowercase().as_str() {
            "ncols" => ncols = parts[1].parse().unwrap_or(0),
            "nrows" => nrows = parts[1].parse().unwrap_or(0),
            "nodata_value" => nodata = parts[1].parse().unwrap_or(-99999.0),
            _ if reading_data || parts[0].parse::<f32>().is_ok() => {
                reading_data = true;
                let row: Vec<f32> = parts.iter().map(|&x| x.parse().unwrap_or(nodata)).collect();
                if row.len() == ncols { data.push(row); }
            }
            _ => {}
        }
    }

    if data.len() != nrows {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Row count mismatch"));
    }

    Ok((data, ncols, nrows, nodata))
}

/// Renders hillshaded color image to hillside.png inside output_dir
fn render_hillshade_image(data: &[Vec<f32>], ncols: usize, nrows: usize, nodata: f32, output_dir: &Path) -> io::Result<()> {
    let grad = colorgrad::viridis();
    let (min, max) = find_min_max(data, nodata);
    let elev: Vec<Vec<f64>> = data.iter().map(|r| r.iter().map(|&v| v as f64).collect()).collect();
    let mut img = RgbImage::new(ncols as u32, nrows as u32);

    let cell_size = 30.0;
    let zf = 1.0;
    let azimuth = 315.0;
    let altitude = 45.0;

    for y in 0..nrows {
        for x in 0..ncols {
            let val = data[y][x];
            let base = if val == nodata {
                Rgb([0, 0, 0])
            } else {
                let norm = (val - min) / (max - min);
                let (r, g, b, _) = grad.at(norm.clamp(0.0, 1.0) as f64).rgba();
                Rgb([(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8])
            };

            let shade = if val == nodata {
                0
            } else {
                calculate_hillshade(&elev, x, y, cell_size, zf, azimuth, altitude, nodata as f64)
            };

            let factor = shade as f32 / 255.0;
            let final_pixel = Rgb([
                (base[0] as f32 * (1.0 - factor)) as u8,
                (base[1] as f32 * (1.0 - factor)) as u8,
                (base[2] as f32 * (1.0 - factor)) as u8,
            ]);

            img.put_pixel(x as u32, y as u32, final_pixel);
        }
    }

    let output_path = output_dir.join("hillside.png");
    img.save(output_path)?;
    Ok(())
}

/// Finds minimum and maximum valid elevation
fn find_min_max(data: &[Vec<f32>], nodata: f32) -> (f32, f32) {
    let mut min = f32::MAX;
    let mut max = f32::MIN;
    for row in data {
        for &v in row {
            if v != nodata {
                min = min.min(v);
                max = max.max(v);
            }
        }
    }
    (min, max)
}

/// Calculates hillshade intensity using Horn's method
fn calculate_hillshade(elev: &[Vec<f64>], x: usize, y: usize, cell: f64, zf: f64, az: f64, alt: f64, nodata: f64) -> u8 {
    let get = |dx: isize, dy: isize| -> f64 {
        let nx = x as isize + dx;
        let ny = y as isize + dy;
        if nx >= 0 && ny >= 0 && (nx as usize) < elev[0].len() && (ny as usize) < elev.len() {
            elev[ny as usize][nx as usize]
        } else {
            nodata
        }
    };

    let dzdx = ((get(1,-1) + 2.0*get(1,0) + get(1,1)) - (get(-1,-1) + 2.0*get(-1,0) + get(-1,1))) / (8.0 * cell) * zf;
    let dzdy = ((get(-1,1) + 2.0*get(0,1) + get(1,1)) - (get(-1,-1) + 2.0*get(0,-1) + get(1,-1))) / (8.0 * cell) * zf;

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
    let shade = 255.0 * ((alt_rad.sin() * slope.sin()) + (alt_rad.cos() * slope.cos() * (az_rad - aspect).cos()));

    shade.clamp(0.0, 255.0) as u8
}
