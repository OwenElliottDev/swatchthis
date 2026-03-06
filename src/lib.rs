//! Colour swatch extraction from images using k-means clustering.
//!
//! Supports clustering in both RGB and CIELAB colour spaces, with random or
//! k-means++ initialisation. Designed to work in native Rust and WebAssembly.
//!
//! # Example
//!
//! ```
//! use swatchthis::{generate_swatches, pixels_from_rgba};
//! use swatchthis::kmeans::{ColorSpace, InitMethod};
//!
//! // Simulate a small image: 4 red pixels and 4 blue pixels (RGBA)
//! let rgba: Vec<u8> = [255, 0, 0, 255].repeat(4)
//!     .into_iter()
//!     .chain([0, 0, 255, 255].repeat(4))
//!     .collect();
//!
//! let pixels = pixels_from_rgba(&rgba);
//! let swatches = generate_swatches(&pixels, 2, ColorSpace::Rgb, InitMethod::KMeansPlusPlus, 42);
//!
//! assert_eq!(swatches.len(), 2);
//! for s in &swatches {
//!     println!("{} ({}px)", s.hex(), s.population);
//! }
//! ```

pub mod color;
pub mod kmeans;
pub mod swatch;

use color::Rgb;
use kmeans::{ColorSpace, InitMethod};
use swatch::Swatch;

/// Extracts dominant colour swatches from a slice of pixels.
///
/// Returns swatches sorted by population (most common first). Large images are
/// automatically subsampled for performance; population counts reflect the
/// sampled distribution.
///
/// # Example
///
/// ```
/// use swatchthis::generate_swatches;
/// use swatchthis::color::Rgb;
/// use swatchthis::kmeans::{ColorSpace, InitMethod};
///
/// let pixels = vec![Rgb::new(255, 0, 0); 100];
/// let swatches = generate_swatches(&pixels, 1, ColorSpace::Rgb, InitMethod::Random, 1);
///
/// assert_eq!(swatches[0].color, Rgb::new(255, 0, 0));
/// ```
pub fn generate_swatches(
    pixels: &[Rgb],
    count: usize,
    color_space: ColorSpace,
    init: InitMethod,
    seed: u64,
) -> Vec<Swatch> {
    let mut swatches: Vec<Swatch> = kmeans::extract_colors(pixels, count, color_space, init, seed)
        .into_iter()
        .map(|(color, pop)| Swatch::new(color, pop))
        .collect();

    swatches.sort_by(|a, b| b.population.cmp(&a.population));
    swatches
}

/// Parses raw RGBA bytes into [`Rgb`] pixels. Alpha is discarded.
///
/// This is useful for converting `ImageData` from an HTML canvas or any other
/// source that provides pixels as contiguous RGBA bytes.
///
/// # Example
///
/// ```
/// use swatchthis::pixels_from_rgba;
/// use swatchthis::color::Rgb;
///
/// let rgba = [255, 128, 0, 255, 0, 0, 0, 255];
/// let pixels = pixels_from_rgba(&rgba);
///
/// assert_eq!(pixels, vec![Rgb::new(255, 128, 0), Rgb::new(0, 0, 0)]);
/// ```
pub fn pixels_from_rgba(data: &[u8]) -> Vec<Rgb> {
    data.chunks_exact(4)
        .map(|chunk| Rgb::new(chunk[0], chunk[1], chunk[2]))
        .collect()
}

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = generateSwatches)]
pub fn generate_swatches_wasm(
    rgba_data: &[u8],
    count: usize,
    color_space: &str,
    init_method: &str,
    seed: u64,
) -> String {
    let pixels = pixels_from_rgba(rgba_data);

    let cs = match color_space {
        "lab" => ColorSpace::Lab,
        "lab-ciede2000" => ColorSpace::LabCIEDE2000,
        _ => ColorSpace::Rgb,
    };
    let init = match init_method {
        "random" => InitMethod::Random,
        _ => InitMethod::KMeansPlusPlus,
    };

    let swatches = generate_swatches(&pixels, count, cs, init, seed);

    let entries: Vec<String> = swatches
        .iter()
        .map(|s| {
            format!(
                r#"{{"hex":"{}","r":{},"g":{},"b":{},"population":{}}}"#,
                s.hex(),
                s.color.r,
                s.color.g,
                s.color.b,
                s.population
            )
        })
        .collect();
    format!("[{}]", entries.join(","))
}
