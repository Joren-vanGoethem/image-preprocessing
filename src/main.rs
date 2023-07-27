use rayon::prelude::*;
use regex::Regex;
use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::CompressionMethod::Stored;
use zip::ZipWriter;

fn copy_directory(src: &str, dest: &str) -> io::Result<()> {
    for entry in WalkDir::new(src) {
        let entry = entry?;
        let path = entry.path();
        let relative_path = path.strip_prefix(src).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to strip prefix: {:?}", e),
            )
        })?;

        let dest_path = Path::new(dest).join(relative_path);

        if path.is_dir() {
            fs::create_dir_all(&dest_path)?;
        } else {
            fs::copy(&path, &dest_path)?;
        }
    }
    Ok(())
}

fn zip_directory(src_dir: &str, dest_file: &str) -> io::Result<()> {
    let path = Path::new(src_dir);
    let file = File::create(&Path::new(dest_file))?;
    let walkdir = WalkDir::new(path);
    let it = walkdir.into_iter();

    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(Stored) // 0: no compression
        .unix_permissions(0o755);

    for entry in it {
        let entry = entry?;
        let path = entry.path();
        let name = path.strip_prefix(Path::new(src_dir)).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to strip prefix: {:?}", e),
            )
        })?;

        if path.is_file() {
            zip.start_file(name.to_string_lossy().into_owned(), options)?;
            let mut f = File::open(path)?;
            std::io::copy(&mut f, &mut zip)?;
        }
    }
    zip.finish()?;
    Ok(())
}

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

fn gather_image_paths(directory: &str) -> Vec<(String, String, u32)> {
    let mut file_paths = vec![];
    let entries: fs::ReadDir = fs::read_dir(directory).expect("Failed to read directory");

    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_dir() {
                let path_string = path.to_str().unwrap();
                let (_, last_folder) = path_string.rsplit_once('/').unwrap();

                if !["default", "20", "200", "400", "600", "800", "1000", "1200"]
                    .contains(&last_folder)
                {
                    let mut sub_paths = gather_image_paths(&path.to_string_lossy());
                    file_paths.append(&mut sub_paths);
                };
            } else if let Some(extension) = path.extension() {
                if let Some(extension_str) = extension.to_str() {
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

                        // Create default directory and copy image into it
                        let default_dir = format!("{}/default", directory);
                        let default_file = format!(
                            "{}/{}.{}",
                            default_dir,
                            file_name_cleaned,
                            extension_str.to_lowercase()
                        );

                        if !Path::new(&default_dir).exists() {
                            fs::create_dir_all(&default_dir).expect("Failed to create directory");
                        }
                        if !Path::new(&default_file).exists() {
                            fs::copy(&path, &default_file).expect("Failed to copy file");
                        }

                        let pattern = r".*-\d{2,4}$";
                        let re = Regex::new(pattern).unwrap();

                        if !re.is_match(&file_name_cleaned) {
                            for width in [20, 200, 400, 600, 800, 1000, 1200].iter() {
                                let w = width.to_string();
                                let output_directory = format!("{}/{}", directory, w);
                                let output_file = format!(
                                    "{}/{}/{}.{}",
                                    directory,
                                    w,
                                    file_name_cleaned,
                                    extension_str.to_lowercase()
                                );

                                let p: &Path = Path::new(&output_file);
                                let output_path_str = p.to_string_lossy().to_string();

                                if !Path::new(&output_directory).exists() {
                                    fs::create_dir_all(&output_directory)
                                        .expect("Failed to create directory");
                                }

                                if !Path::new(&output_file).exists() {
                                    file_paths.push((
                                        path.to_str().unwrap().to_string(),
                                        output_path_str,
                                        *width,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    file_paths
}

// Function to autorotate an image using exiftran
fn auto_rotate_image(filename: &str) -> Result<(), std::io::Error> {
    // Execute jpegtran command to auto-rotate the image in-place
    Command::new("exiftran").arg("-ai").arg(filename).output()?;
    Ok(())
}

fn process_files(files: Vec<(String, String, u32)>) {
    files.par_iter().for_each(|(input, output, int_value)| {
        auto_rotate_image(&input);
        // Replace with your actual processing function
        if let Some(parent) = Path::new(output).parent() {
            if !parent.exists() {
                println!("creating directory: {}", &parent.to_string_lossy());
                fs::create_dir_all(parent).expect("Failed to create directory");
            }
        }
        resize_image_ffmpeg(input, output, *int_value);
    });
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

    // Specify the source directory and destination directory
    let src_dir = "/Users/jonasfaber/Library/CloudStorage/GoogleDrive-jonas@hagenfaber.eu/.shortcut-targets-by-id/1W5yfM9fIlw09ul_aDfgAA0Kj0gJsUg1r/STRUCTURE_EXAMPLE";
    let dest_dir = "./images";

    // Call the copy_directory function
    match copy_directory(src_dir, dest_dir) {
        Ok(_) => println!("Directory copied successfully."),
        Err(e) => eprintln!("Error copying directory: {}", e),
    }

    let dest_file = "images.zip";

    // Autorotate images using exiftran in the specified directory
    // autorotate_images_exiftran(directory);

    // Resize images using ffmpeg in the specified directory
    // resize_images_ffmpeg(directory);
    let images = gather_image_paths(directory);
    process_files(images);

    match zip_directory(dest_dir, dest_file) {
        Ok(_) => println!("Zipped directory successfully."),
        Err(e) => eprintln!("Error zipping directory: {}", e),
    }
}
