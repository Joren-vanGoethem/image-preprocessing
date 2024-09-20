use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};

pub trait ImageBufferConversions {
    fn to_image_buffer(&self) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>>;
}

impl ImageBufferConversions for DynamicImage {
    fn to_image_buffer(&self) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        let (w,h) = self.dimensions();
        let rgba_image = self.to_rgba8();
        let image_bytes = rgba_image.as_raw();
        ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(w, h, image_bytes.to_vec())
    }
}