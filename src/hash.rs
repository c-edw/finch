use image::{DynamicImage, FilterType};

use std::f64;

const RESIZE_DIMENSION: u32 = 8;
const RESIZE_LENGTH: u32 = 64;

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

pub trait Hash {
    fn average_hash(&self) -> ImageHash;
    fn marr_hash(&self) -> ImageHash;
}

impl Hash for DynamicImage {
    /// Calculate the perceptual hash of an image using the Average Hash algorithm.
    fn average_hash(&self) -> ImageHash {
        let resize = self.resize_exact(RESIZE_DIMENSION, RESIZE_DIMENSION, FilterType::Nearest)
            .grayscale();

        let raw = resize.raw_pixels();

        assert_eq!(raw.len(), RESIZE_LENGTH as usize);

        // Calculate the average value.
        let average = raw.iter().map(|&n| n as usize).sum::<usize>() / RESIZE_LENGTH as usize;

        // Calculate a 64-bit hash based on whether each value is greater than the average value.
        let hash = raw.iter()
            .map(|&n| (n as usize > average) as u64)
            .enumerate()
            .fold(0, |acc, (i, n)| acc | (n << i));

        ImageHash::new(hash)
    }

    /// Calculate the perceptual hash of an image using the Marr Wavelet Hash algorithm.
    fn marr_hash(&self) -> ImageHash {
        let resize = self.resize_exact(RESIZE_DIMENSION, RESIZE_DIMENSION, FilterType::Nearest)
            .grayscale();

        let raw = resize.raw_pixels();

        assert_eq!(raw.len(), RESIZE_LENGTH as usize);

        // Run the raw pixel buffer through the wavelet kernel.
        let new = raw.iter().enumerate().map(marr_kernel).collect::<Vec<_>>();

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

fn marr_kernel((i, n): (usize, &u8)) -> f64 {
    let sigma_pow = f64::from(RESIZE_LENGTH);

    let (x, y) = (i % RESIZE_DIMENSION as usize, i / RESIZE_DIMENSION as usize);
    let (xpow, ypow) = (x.pow(2), y.pow(2));

    let mult_one = 1f64 / (f64::consts::PI * sigma_pow);
    let mult_two = (xpow + ypow) as f64 / sigma_pow;
    let mult_three = -((xpow + ypow) as f64 / (2f64 * sigma_pow));

    ((mult_one * (1f64 - (mult_two / 2f64))) * f64::consts::E.powf(mult_three)) * f64::from(*n)
}
