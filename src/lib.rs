//! Colour swatch extraction from images using k-means clustering.
//!
//! Supports clustering in both RGB and CIELAB colour spaces, with random or
//! k-means++ initialisation. Designed to work in native Rust and WebAssembly.
//!
//! # Example
//!
//! ```
//! use swatchthis::{generate_swatches_kmeans, pixels_from_rgba};
//! use swatchthis::algorithms::kmeans::{KmeansColorSpace, InitMethod};
//!
//! // Simulate a small image: 4 red pixels and 4 blue pixels (RGBA)
//! let rgba: Vec<u8> = [255, 0, 0, 255].repeat(4)
//!     .into_iter()
//!     .chain([0, 0, 255, 255].repeat(4))
//!     .collect();
//!
//! let pixels = pixels_from_rgba(&rgba);
//! let swatches = generate_swatches_kmeans(&pixels, 2, KmeansColorSpace::Rgb, InitMethod::KMeansPlusPlus, 42);
//!
//! assert_eq!(swatches.len(), 2);
//! for s in &swatches {
//!     println!("{} ({}px)", s.hex(), s.population);
//! }
//! ```

pub mod algorithms;
pub mod color;
pub mod preprocessors;
pub mod swatch;

use algorithms::kmeans;
use algorithms::median_cut;
use algorithms::octree;
use color::Rgb;
use kmeans::{InitMethod, KmeansColorSpace};
use octree::{OctreeColorSpace, OctreeDepth};
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
/// use swatchthis::generate_swatches_kmeans;
/// use swatchthis::color::Rgb;
/// use swatchthis::algorithms::kmeans::{KmeansColorSpace, InitMethod};
///
/// let pixels = vec![Rgb::new(255, 0, 0); 100];
/// let swatches = generate_swatches_kmeans(&pixels, 1, KmeansColorSpace::Rgb, InitMethod::Random, 1);
///
/// assert_eq!(swatches[0].color, Rgb::new(255, 0, 0));
/// ```
pub fn generate_swatches_kmeans(
    pixels: &[Rgb],
    count: usize,
    color_space: KmeansColorSpace,
    init: InitMethod,
    seed: u64,
) -> Vec<Swatch> {
    collect_sorted_swatches(kmeans::extract_colors_kmeans(
        pixels,
        count,
        color_space,
        init,
        seed,
    ))
}

pub fn generate_swatches_octree(
    pixels: &[Rgb],
    count: usize,
    color_space: OctreeColorSpace,
    max_depth: OctreeDepth,
) -> Vec<Swatch> {
    collect_sorted_swatches(octree::extract_colors_octree(
        pixels,
        count,
        color_space,
        max_depth,
    ))
}

pub fn generate_swatches_median_cut(pixels: &[Rgb], count: usize) -> Vec<Swatch> {
    collect_sorted_swatches(median_cut::extract_colors_median_cut(pixels, count))
}

fn collect_sorted_swatches(raw: Vec<(Rgb, u32)>) -> Vec<Swatch> {
    let mut swatches: Vec<Swatch> = raw.into_iter().map(|(c, p)| Swatch::new(c, p)).collect();
    swatches.sort_by_key(|b| std::cmp::Reverse(b.population));
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

const MAX_SAMPLE: usize = 20_000;

fn sample_step(len: usize) -> usize {
    if len > MAX_SAMPLE {
        len / MAX_SAMPLE
    } else {
        1
    }
}

#[cfg(feature = "wasm")]
use swatch::swatches_to_json;
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = generateSwatches)]
pub fn generate_swatches_kmeans_wasm(
    rgba_data: &[u8],
    count: usize,
    color_space: &str,
    init_method: &str,
    seed: u64,
) -> String {
    let pixels = pixels_from_rgba(rgba_data);

    let cs = match color_space {
        "lab" => KmeansColorSpace::Lab,
        "lab-ciede2000" => KmeansColorSpace::LabCIEDE2000,
        _ => KmeansColorSpace::Rgb,
    };
    let init = match init_method {
        "random" => InitMethod::Random,
        _ => InitMethod::KMeansPlusPlus,
    };

    let swatches = generate_swatches_kmeans(&pixels, count, cs, init, seed);
    swatches_to_json(&swatches)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = generateSwatchesOctree)]
pub fn generate_swatches_octree_wasm(
    rgba_data: &[u8],
    count: usize,
    color_space: &str,
    max_depth: u32,
) -> String {
    let pixels = pixels_from_rgba(rgba_data);

    let cs = match color_space {
        "lab" => OctreeColorSpace::Lab,
        _ => OctreeColorSpace::Rgb,
    };

    let swatches = generate_swatches_octree(&pixels, count, cs, OctreeDepth::from_u32(max_depth));
    swatches_to_json(&swatches)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = generateSwatchesMedianCut)]
pub fn generate_swatches_median_cut_wasm(rgba_data: &[u8], count: usize) -> String {
    let pixels = pixels_from_rgba(rgba_data);
    let swatches = generate_swatches_median_cut(&pixels, count);
    swatches_to_json(&swatches)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = complementaryColor)]
pub fn complementary_color_wasm(r: u8, g: u8, b: u8) -> Vec<u8> {
    let comp = Rgb::new(r, g, b).to_hsl().complement().to_rgb();
    vec![comp.r, comp.g, comp.b]
}

#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = slicPreprocess)]
pub fn slic_preprocess_wasm(
    rgba_data: &[u8],
    width: usize,
    height: usize,
    num_superpixels: usize,
    compactness: f32,
) -> Vec<u8> {
    let pixels = pixels_from_rgba(rgba_data);
    let result =
        preprocessors::slic::slic_preprocess(&pixels, width, height, num_superpixels, compactness);
    preprocessors::rgb_vec_to_rgba(&result)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = seedsPreprocess)]
pub fn seeds_preprocess_wasm(
    rgba_data: &[u8],
    width: usize,
    height: usize,
    num_superpixels: usize,
    num_levels: usize,
    histogram_bins: usize,
) -> Vec<u8> {
    let pixels = pixels_from_rgba(rgba_data);
    let result = preprocessors::seeds::seeds_preprocess(
        &pixels,
        width,
        height,
        num_superpixels,
        num_levels,
        histogram_bins,
    );
    preprocessors::rgb_vec_to_rgba(&result)
}
