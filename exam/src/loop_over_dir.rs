use std::fs::{self, File};
use std::io::{Write, BufWriter};
use std::path::Path;
use walkdir::WalkDir;

fn main() {
    let root_dir = "/Users/MatthiasSchmitt/Desktop/Studium/9.Semester/VL/rust/RUST_exam/exam/dataset"; // Change this to your target directory

    let extensions = vec!["asc", "tif", "xyz", "txt"];
    let mut file_lists: Vec<(String, Vec<String>)> = extensions
        .iter()
        .map(|ext| (format!("file_paths_{}.txt", ext), Vec::new()))
        .collect();

    // Walk through the directory recursively
    for entry in WalkDir::new(root_dir).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() {
            if let Some(ext) = entry.path().extension().and_then(|s| s.to_str()) {
                for (filename, list) in &mut file_lists {
                    if filename.contains(ext) {
                        list.push(entry.path().display().to_string());
                    }
                }
            }
        }
    }

    // Save each list to a separate text file
    for (filename, list) in file_lists {
        if let Ok(file) = File::create(&filename) {
            let mut writer = BufWriter::new(file);
            for path in list {
                writeln!(writer, "{}", path).expect("Failed to write to file");
            }
            println!("Saved paths to {}", filename);
        } else {
            eprintln!("Failed to create {}", filename);
        }
    }
}
