use image::{ImageBuffer, Rgba};
use image::imageops::{flip_horizontal, flip_vertical, rotate180, rotate270, rotate90};

// vertical flips could also be user for number 4, but this keeps the logic simple, rotate if needed, then flip if needed
#[repr(u32)]
#[derive(PartialEq, Debug)]
pub enum ExifRotation {
    Upright = 1,
    FlippedHorizontal = 2,
    Rotated180 = 3,
    FlippedVertical = 4,
    Rotated90CWFlippedHorizontal = 5,
    Rotated90CCW = 6,
    Rotated90CCWFlippedHorizontal = 7,
    Rotated90CW = 8,
}

impl TryFrom<u32> for ExifRotation {
    type Error = &'static str;  // Simpler error type

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if (1..=8).contains(&value) {
            Ok(unsafe { std::mem::transmute(value) })
        } else {
            Err("only values 1 to 8 are supported")
        }
    }
}

impl ExifRotation {
    pub fn apply(&self, image_buffer: ImageBuffer<Rgba<u8>, Vec<u8>>) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        match self {
            ExifRotation::Upright => {image_buffer}
            ExifRotation::FlippedHorizontal => {flip_horizontal(&image_buffer)}
            ExifRotation::Rotated180 => {rotate180(&image_buffer)}
            ExifRotation::FlippedVertical => {flip_vertical(&image_buffer)}
            ExifRotation::Rotated90CWFlippedHorizontal => {rotate270(&flip_horizontal(&image_buffer))}
            ExifRotation::Rotated90CCW => {rotate90(&image_buffer)}
            ExifRotation::Rotated90CCWFlippedHorizontal => {rotate90(&flip_horizontal(&image_buffer))}
            ExifRotation::Rotated90CW => {rotate270(&image_buffer)}
        }
    }
}