use image::{DynamicImage, FilterType};

use std::f64;

const RESIZE_DIMENSION: u32 = 8;
const RESIZE_LENGTH: u32 = 64;

#[derive(Debug)]
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
        1f32 - (self.hamming(other) as f32 / RESIZE_LENGTH as f32)
    }
}

#[allow(unused)]
pub enum Algorithm {
    Average,
    Marr,
}

pub trait Hash {
    fn hash(&self, Algorithm) -> ImageHash;
}

impl Hash for DynamicImage {
    /// Calculate the perceptual hash of an image using the specified Algorithm.
    fn hash(&self, algorithm: Algorithm) -> ImageHash {
        let resize = self.resize_exact(RESIZE_DIMENSION, RESIZE_DIMENSION, FilterType::Nearest)
            .grayscale();

        let raw = resize.raw_pixels();

        assert_eq!(raw.len(), RESIZE_LENGTH as usize);

        let kernel = match algorithm {
            Algorithm::Average => average_kernel,
            Algorithm::Marr => marr_kernel,
        };

        // Run the raw pixel buffer through the wavelet kernel.
        let new = raw.iter().enumerate().map(kernel).collect::<Vec<_>>();

        // Calculate the average value.
        let average = new.iter().sum::<f64>() / f64::from(RESIZE_LENGTH);

        // Calculate a 64-bit hash based on whether each value is greater than the average value.
        let hash = new.iter()
            .map(|&n| (n > average) as u64)
            .enumerate()
            .fold(0, |acc, (i, n)| acc | (n << i));

        ImageHash::new(hash)
    }
}

fn average_kernel((_, n): (usize, &u8)) -> f64 {
    // The Average Hash algorithm has no kernel.
    f64::from(*n)
}

fn marr_kernel((i, n): (usize, &u8)) -> f64 {
    let sigma_pow = f64::from(RESIZE_LENGTH);

    let (x, y) = (i % RESIZE_DIMENSION as usize, i / RESIZE_DIMENSION as usize);
    let (xpow, ypow) = (x.pow(2), y.pow(2));

    let mult_one = 1f64 / (f64::consts::PI * sigma_pow);
    let mult_two = (xpow + ypow) as f64 / sigma_pow;
    let mult_three = -((xpow + ypow) as f64 / (2f64 * sigma_pow));

    ((mult_one * (1f64 - (mult_two / 2f64))) * f64::consts::E.powf(mult_three)) * f64::from(*n)
}

#[cfg(test)]
mod tests {
    use hash::{Algorithm, Hash, ImageHash};
    use image::{ImageRgba8, Rgba, RgbaImage};

    #[test]
    fn hamming_distance_is_correct() {
        let a = ImageHash {
            bits: 0xFFFFFFFFFFFFFFFF,
        };
        let b = ImageHash {
            bits: 0xFFFFFFFF00000000,
        };

        assert_eq!(a.hamming(&b), 32);
    }

    #[test]
    fn similarity_is_correct() {
        let a = ImageHash {
            bits: 0xFFFFFFFFFFFFFFFF,
        };
        let b = ImageHash {
            bits: 0xFFFFFFFF00000000,
        };

        assert_eq!(a.similarity(&b), 0.5);
    }

    #[test]
    #[allow(deprecated)]
    fn average_hash_is_correct() {
        let mut image = RgbaImage::new(32, 32);

        // Generate a noisy image.
        image
            .enumerate_pixels_mut()
            .enumerate()
            .map(|(i, (_, _, pixel))| *pixel = Rgba([(i % 255) as u8, 255, 255, 255]))
            .for_each(drop);

        assert_eq!(
            ImageRgba8(image).hash(Algorithm::Average).bits,
            0xFF00FF00FF00FF00
        );
    }

    #[test]
    fn marr_hash_is_correct() {
        let mut image = RgbaImage::new(32, 32);

        // Generate a noisy image.
        image
            .enumerate_pixels_mut()
            .enumerate()
            .map(|(i, (_, _, pixel))| *pixel = Rgba([(i % 255) as u8, 255, 255, 255]))
            .for_each(drop);

        assert_eq!(ImageRgba8(image).hash(Algorithm::Marr).bits, 0xF0F3F1F3F3F);
    }
}
