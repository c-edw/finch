use image::{DynamicImage, FilterType};

const RESIZE_DIMENSION: u32 = 8;

pub struct ImageHash {
    bits: u64,
}

impl ImageHash {
    fn new(bits: u64) -> Self {
        ImageHash { bits }
    }

    /// Get the Hamming distance between another ImageHash
    pub fn hamming(&self, other: &ImageHash) -> u32 {
        (self.bits ^ other.bits).count_ones()
    }

    /// Get the similarity to another ImageHash as a float between 0 and 1, where 0 represents no similarity and 1 represents no difference.
    pub fn similarity(&self, other: &ImageHash) -> f32 {
        1f32 - (self.hamming(other) as f32 / RESIZE_DIMENSION.pow(2) as f32)
    }
}

pub trait Hash {
    fn average_hash(&self) -> ImageHash;
}

impl Hash for DynamicImage {
    /// Calculate the perceptual hash of an image using the Average Hash algorithm.
    fn average_hash(&self) -> ImageHash {
        let resize = self.resize_exact(RESIZE_DIMENSION, RESIZE_DIMENSION, FilterType::Nearest)
            .grayscale();

        let raw = resize.raw_pixels();

        assert_eq!(raw.len(), RESIZE_DIMENSION.pow(2) as usize);

        let average =
            raw.iter().map(|&n| n as usize).sum::<usize>() / RESIZE_DIMENSION.pow(2) as usize;

        let hash = raw.iter()
            .map(|&n| (n as usize > average) as u64)
            .enumerate()
            .fold(0, |acc, (i, n)| acc | (n << i));

        ImageHash::new(hash)
    }
}
