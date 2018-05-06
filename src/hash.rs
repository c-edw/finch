use image::{DynamicImage, FilterType};

const RESIZE_DIMENSION: u32 = 8;

pub struct ImageHash {
    bits: u64,
}

impl ImageHash {
    fn new(bits: u64) -> Self {
        ImageHash { bits }
    }

    pub fn hamming(&self, other: &ImageHash) -> u32 {
        (self.bits ^ other.bits).count_ones()
    }

    pub fn similarity(&self, other: &ImageHash) -> f32 {
        1f32 - (self.hamming(other) as f32 / 64f32)
    }
}

pub fn average_hash(image: &DynamicImage) -> ImageHash {
    let resize = image
        .resize_exact(RESIZE_DIMENSION, RESIZE_DIMENSION, FilterType::Nearest)
        .grayscale();

    let raw = resize.raw_pixels();

    assert_eq!(raw.len(), 64);

    let average = raw.iter().map(|&n| n as usize).sum::<usize>() / 64;

    let hash = raw.iter()
        .map(|&n| (n as usize > average) as u64)
        .enumerate()
        .fold(0, |acc, (i, n)| acc | (n << i));

    ImageHash::new(hash)
}
