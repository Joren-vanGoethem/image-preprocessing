mod image_buffer_conversions;
mod exif_rotation;

use regex::Regex;
use std::{env, fs, io};
use std::fs::File;
use std::path::Path;
use std::process::{Command};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod::Stored;
use zip::ZipWriter;
use exif;
use exif::{Error, Exif};
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};
use crate::exif_rotation::ExifRotation;
use crate::image_buffer_conversions::ImageBufferConversions;

// [20, 200, 400, 600, 800, 1000, 1200] // to speedup testing

fn zip_directory(src_dir: &str, dest_file: &str) -> io::Result<()> {
    let path = Path::new(src_dir);
    let file = File::create(&Path::new(dest_file))?;
    let walkdir = WalkDir::new(path);
    let it = walkdir.into_iter();

    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
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
                match path.to_str() {
                    None => {}
                    Some(path_string) => {
                        let (_, last_folder) = path_string.rsplit_once(std::path::MAIN_SEPARATOR).unwrap();

                        if !["default", "1200"]
                            .contains(&last_folder)
                        {
                            let mut sub_paths = gather_image_paths(&path.to_string_lossy());
                            file_paths.append(&mut sub_paths);
                        };
                    }
                }
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
                            fix_rotation(
                                format!(
                                    "{}/{}.{}",
                                    directory,
                                    file_name_cleaned,
                                    extension_str.to_lowercase()
                                ).as_str()
                            );

                            for width in [1200].iter() {
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

fn read_image_to_buffer(filename: &str) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    let dyn_img = image::open(filename).unwrap();
    dyn_img.to_image_buffer()
}

fn read_exif_data_from_file(filename: &str) -> Result<Exif, Error> {
    let file = File::open(filename)?;
    let mut bufreader = std::io::BufReader::new(&file);
    let exif_reader = exif::Reader::new();
    exif_reader.read_from_container(&mut bufreader)
}

fn fix_rotation(filename: &str) {
    let exif_result = read_exif_data_from_file(filename);

    match exif_result {
        Ok(exif_data) => {
            let rotation = ExifRotation::read_rotation_from_exif(exif_data);
            if rotation == ExifRotation::Upright {
                return;
            }
            let mut image_buffer_option = read_image_to_buffer(&filename);
            match image_buffer_option {
                None => {eprintln!("Reading image to buffer failed")}
                Some(image_buffer) => {
                    let rotated_image = rotation.apply(image_buffer);

                    let rgb_image = DynamicImage::ImageRgba8(rotated_image).to_rgb8(); // conversion needed because jpg doesn't have a channel
                    rgb_image.save(format!("{}rotated{:?}.jpg", filename, rotation)).expect("unable to save rotated image");
                }
            }
        },
        Err(err) => eprintln!("Reading exif data failed: {err}")
    }

}


fn process_files(files: Vec<(String, String, u32)>) {
    files.iter().for_each(|(input, output, int_value)| {

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
    let dest_file = "images.zip";

    let images = gather_image_paths(directory);
    process_files(images);

    match zip_directory(directory, dest_file) {
        Ok(_) => println!("Zipped directory successfully."),
        Err(e) => eprintln!("Error zipping directory: {}", e),
    }
}
