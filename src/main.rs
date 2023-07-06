

use std::env;
use std::fs;
use std::path::{Path};
use std::process::Command;
use regex::Regex;



// Function to resize an image using ffmpeg
fn resize_image_ffmpeg(input_file: &str, output_file: &str, width: u32) {
    Command::new("ffmpeg")
        .args(&[
            "-i",
            input_file,
            "-vf",
            &format!("scale={}:{}", width, -1),
            output_file,
        ])
        .output()
        .expect("Failed to execute ffmpeg");
}


// Function to resize images using ffmpeg
fn resize_images_ffmpeg(directory: &str) {
    let entries = fs::read_dir(directory).expect("Failed to read directory");

    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_dir() {
                // Recursive call to handle nested directories
                let path_string = path.to_str().unwrap();
                let (_, last_folder) = path_string.rsplit_once('/').unwrap();


                if !["default", "20", "200", "400", "600", "800", "1000", "1200"].contains(&last_folder) {
                    resize_images_ffmpeg(&path.to_string_lossy());
                };
                
            } else if let Some(extension) = path.extension() {
                if let Some(extension_str) = extension.to_str() {
                    // Check if the file is an image
                    if extension_str.eq_ignore_ascii_case("jpg")
                        || extension_str.eq_ignore_ascii_case("jpeg")
                        || extension_str.eq_ignore_ascii_case("png")
                        || extension_str.eq_ignore_ascii_case("gif")
                    {
                        
                        
                        let file_name = path.file_stem().expect("Failed to get file stem");
                        let file_name_str = file_name.to_string_lossy();
                        let file_name_cleaned = file_name_str
                            .trim_matches(|c: char| !c.is_alphanumeric())
                            .to_string();


                        match fs::create_dir(format!("{}/default", directory)) {
                            Ok(_) => {

                            }
                            Err(_) => {
                                // eprintln!("Error creating directory: {}", error);    
                            }
                        }

                        let of = format!(
                            "{}/default/{}.{}",
                            directory, file_name_cleaned, extension_str.to_lowercase()
                        );
                        let cleaned_file_name = of.clone();

                        
                        match fs::copy(&path, of) {
                            Ok(_) => {

                            }
                            Err(_) => {
                                // eprintln!("Error creating directory: {}", error);    
                            }
                        }

                        
                        let pattern = r".*-\d{2,4}$";
                        let re = Regex::new(pattern).unwrap();

                        if !re.is_match(&file_name_cleaned) {
                            println!("Resizing (ffmpeg): {:?}", cleaned_file_name);
                            
                            for width in [20, 200, 400, 600, 800, 1000, 1200].iter() {
                                let w = width.to_string();
                                let path_string = path.to_str().unwrap();
                                let (file_path, _) = path_string.rsplit_once('/').unwrap();
                                

                                match fs::create_dir(file_path.to_owned() + "/" + &w) {
                                    Ok(_) => {

                                    }
                                    Err(_) => {
                                        // eprintln!("Error creating directory: {}", error);    
                                    }
                                }
                                
                                let output_file = format!(
                                    "{}/{}/{}.{}",
                                    directory, width, file_name_cleaned, extension_str.to_lowercase()
                                );

                                let p: &Path = Path::new(&output_file);
                                let path_str = p.to_string_lossy().to_string();
                                // println!("image location: {:?}", path_str);

                                if !Path::new(&output_file).exists() {
                                    resize_image_ffmpeg(&path.to_string_lossy(), &path_str, *width);
                                }
                            }
                        }
                        


                    }
                }
            }
        }
    }
}

// Function to autorotate an image using exiftran
fn auto_rotate_image(filename: &str) -> Result<(), std::io::Error> {
    // Execute jpegtran command to auto-rotate the image in-place
    Command::new("exiftran")
        .arg("-ai")
        .arg(filename)
        .output()?;
        
    Ok(())
}

// Function to autorotate images using exiftran
fn autorotate_images_exiftran(directory: &str) {
    let entries = fs::read_dir(directory).expect("Failed to read directory");

    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_dir() {
                // Recursive call to handle nested directories
                autorotate_images_exiftran(&path.to_string_lossy())
            } else if let Some(extension) = path.extension() {
                if let Some(extension_str) = extension.to_str() {
                    // Check if the file is an image
                    if extension_str.eq_ignore_ascii_case("jpg")
                        || extension_str.eq_ignore_ascii_case("jpeg")
                    {
                        println!("Autorotating (exiftran): {:?}", path);
                        match auto_rotate_image(&path.to_string_lossy()) {
                            Ok(()) => {
                                // println!("Image rotated {}", &path.to_string_lossy());
                                // println!("Image rotated successfully!");
                            }
                            Err(error) => {
                                eprintln!("Error rotating image: {}", error);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn main() {
    // Read command-line arguments
    let args: Vec<String> = env::args().collect();

    // Check the number of arguments
    if args.len() != 2 {
        eprintln!("Usage: cargo run -- <directory_path>");
        return;
    }

    let directory = &args[1];

    
    // Autorotate images using exiftran in the specified directory  
    // autorotate_images_exiftran(directory);

    // Resize images using ffmpeg in the specified directory
    resize_images_ffmpeg(directory);

}
