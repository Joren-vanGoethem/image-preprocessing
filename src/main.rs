mod image_buffer_conversions;
mod exif_rotation;

use regex::Regex;
use std::{env, fs, io};
use std::ffi::OsStr;
use std::fs::File;
use std::io::BufRead;
use std::path::Path;
use walkdir::{WalkDir};
use zip::write::SimpleFileOptions;
use zip::CompressionMethod::Stored;
use zip::ZipWriter;
use exif;
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};
use image::imageops::{resize, FilterType};
use crate::exif_rotation::{fix_rotation};
use crate::image_buffer_conversions::{ImageBufferConversions};

const QUALITY_PRESETS: [u16; 7] = [20, 200, 400, 600, 800, 1000, 1200];

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

fn resize_image(image: &DynamicImage, width: u32, height: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    resize(image, width, height, FilterType::CatmullRom)
}

fn is_supported_extension(extension_str: &str) -> bool {
    // TODO: check if exists in string array, cleaner solution
    extension_str.eq_ignore_ascii_case("jpg")
        || extension_str.eq_ignore_ascii_case("jpeg")
        || extension_str.eq_ignore_ascii_case("png")
        || extension_str.eq_ignore_ascii_case("gif")
        || extension_str.eq_ignore_ascii_case("webp")
        || extension_str.eq_ignore_ascii_case("avif")
}

fn get_image_paths(dir: &str) -> Vec<String> {
    let mut file_paths = vec![];
    let entries: fs::ReadDir = fs::read_dir(dir).expect("Failed to read directory");

    // Store the Vec<String>
    let quality_presets_string: Vec<String> = QUALITY_PRESETS
        .iter()
        .map(|&q| q.to_string())
        .collect();

    // Create Vec<&str> by referencing the strings in Vec<String>
    let quality_presets_slices: Vec<&str> = quality_presets_string
        .iter()
        .map(|s| s.as_str())
        .collect();

    for entry in entries {
        if let Ok(entry) = entry {
            // println!("entry: {:?}", entry);
            let path = entry.path();
            if path.is_dir() {
                match path.to_str() {
                    None => {}
                    Some(path_string) => {
                        let (_, last_folder) = path_string.rsplit_once(std::path::MAIN_SEPARATOR).unwrap();
                        // println!("last_folder: {:?}", last_folder);

                        if !quality_presets_slices
                            .contains(&last_folder)
                        {
                            let mut sub_paths = get_image_paths(&path.to_string_lossy());
                            file_paths.append(&mut sub_paths);
                        };
                    }
                }
            } else if let Some(extension_str) = path.extension().and_then(OsStr::to_str) {
                if is_supported_extension(extension_str)
                {
                    // TODO: are these checks actually useful? we read files from filesystem, i expect them to be valid file names
                    file_paths.push(path.to_str().unwrap().to_string());
                }
            }
        }
    }

    file_paths
}

fn create_dir(dir: &String) {
    if !Path::new(dir).is_dir() {
        fs::create_dir_all(dir).expect(&format!("creating new dir {} failed", dir));
    }
}

fn save_image(image: ImageBuffer<Rgba<u8>, Vec<u8>>, original_file_path: String, output_directory: &str) {
    // TODO: cleanup this mess
    let system_path = Path::new(&original_file_path);
    let extension = system_path.extension().and_then(OsStr::to_str);
    let file_name = system_path.file_stem().and_then(OsStr::to_str).unwrap();
    let directory = original_file_path.rsplit_once(file_name).unwrap().0.split_once(std::path::MAIN_SEPARATOR).unwrap().1;

    println!("file_name: {:?}", file_name);
    println!("extension: {:?}", extension);
    println!("directory: {:?}", directory);

    let dyn_img = DynamicImage::ImageRgba8(image);

    if let Some(ext) = extension {
        let mut result  = Ok(());

        let file_output_path = &format!("{}{}{}{}", output_directory, std::path::MAIN_SEPARATOR, directory, std::path::MAIN_SEPARATOR);
        create_dir(file_output_path);

        match ext.to_ascii_lowercase().as_str() {
            "jpg" | "jpeg" => {
                result = dyn_img.to_rgb8().save(format!("{}{}.{}", file_output_path, file_name, ext));
            },
            "png" | "webp" | "avif" => {
                result = dyn_img.to_rgba8().save(format!("{}{}.{}", file_output_path, file_name, ext));
            },
            _ => eprintln!("extension {} not supported", ext)
        }

        if result.is_err() {
            eprintln!("{:?}", result)
        }
    }
}

fn pre_process_originals(image_paths: Vec<String>, output_directory: &str) {
    for image_path in image_paths {
        let rotated_image_option = fix_rotation(
            format!("{}", image_path).as_str()
        );

        // TODO: enable again after debugging
        match rotated_image_option {
            Some(image) => save_image(image, image_path, output_directory),
            None => eprintln!("rotating image failed, skipping")
        }
    }
}

fn main() {
    // Read command-line arguments
    let args: Vec<String> = env::args().collect();

    // Check the number of arguments
    if args.len() != 2 {
        eprintln!("Usage: cargo run -- <directory>");
        return;
    }

    let input_directory = &args[1];
    let output_directory = "output";
    let dest_file = "images.zip";

    let original_images = get_image_paths(input_directory);

    println!("{:?}", original_images);

    pre_process_originals(original_images, output_directory);

    // let images = gather_image_paths(directory);
    // process_files(images);

    match zip_directory(input_directory, dest_file) {
        Ok(_) => println!("Zipped directory successfully."),
        Err(e) => eprintln!("Error zipping directory: {}", e),
    }
}
