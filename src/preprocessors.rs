pub mod seeds;
pub mod slic;

use crate::color::Rgb;

const DEFAULT_MAX_DIM: usize = 512;

/// Downsample an image so the longest dimension is at most `max_dim`.
/// Returns (new_pixels, new_width, new_height).
pub(crate) fn downsample(
    pixels: &[Rgb],
    width: usize,
    height: usize,
    max_dim: usize,
) -> (Vec<Rgb>, usize, usize) {
    let longest = width.max(height);
    if longest <= max_dim {
        return (pixels.to_vec(), width, height);
    }

    let scale = max_dim as f64 / longest as f64;
    let new_w = ((width as f64 * scale).round() as usize).max(1);
    let new_h = ((height as f64 * scale).round() as usize).max(1);

    let mut out = Vec::with_capacity(new_w * new_h);
    for y in 0..new_h {
        let src_y = ((y as f64 / new_h as f64) * height as f64).min((height - 1) as f64) as usize;
        for x in 0..new_w {
            let src_x = ((x as f64 / new_w as f64) * width as f64).min((width - 1) as f64) as usize;
            out.push(pixels[src_y * width + src_x]);
        }
    }

    (out, new_w, new_h)
}

/// Average the original full-resolution RGB pixels per superpixel label.
/// Empty labels are filtered out.
pub(crate) fn compute_superpixel_averages(
    pixels: &[Rgb],
    labels: &[u32],
    num_labels: usize,
) -> Vec<Rgb> {
    let mut sums = vec![(0u64, 0u64, 0u64, 0u64); num_labels];

    for (px, &label) in pixels.iter().zip(labels.iter()) {
        let entry = &mut sums[label as usize];
        entry.0 += px.r as u64;
        entry.1 += px.g as u64;
        entry.2 += px.b as u64;
        entry.3 += 1;
    }

    sums.into_iter()
        .filter(|&(_, _, _, count)| count > 0)
        .map(|(r, g, b, count)| Rgb::new((r / count) as u8, (g / count) as u8, (b / count) as u8))
        .collect()
}

/// Upscale labels to full resolution and compute superpixel averages.
pub(crate) fn finalize_superpixels(
    pixels: &[Rgb],
    labels: &[u32],
    small_w: usize,
    small_h: usize,
    full_w: usize,
    full_h: usize,
) -> Vec<Rgb> {
    let full_labels = upscale_labels(labels, small_w, small_h, full_w, full_h);
    let num_labels = *full_labels.iter().max().unwrap_or(&0) as usize + 1;
    compute_superpixel_averages(pixels, &full_labels, num_labels)
}

/// Convert a slice of `Rgb` pixels to RGBA bytes (alpha = 255).
#[allow(dead_code)]
pub(crate) fn rgb_vec_to_rgba(pixels: &[Rgb]) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixels.len() * 4);
    for px in pixels {
        out.push(px.r);
        out.push(px.g);
        out.push(px.b);
        out.push(255);
    }
    out
}

/// Upscale a label map from (small_w, small_h) to (full_w, full_h) using nearest-neighbor.
pub(crate) fn upscale_labels(
    labels: &[u32],
    small_w: usize,
    small_h: usize,
    full_w: usize,
    full_h: usize,
) -> Vec<u32> {
    if small_w == full_w && small_h == full_h {
        return labels.to_vec();
    }

    let mut out = vec![0u32; full_w * full_h];
    for y in 0..full_h {
        let src_y =
            ((y as f64 / full_h as f64) * small_h as f64).min((small_h - 1) as f64) as usize;
        for x in 0..full_w {
            let src_x =
                ((x as f64 / full_w as f64) * small_w as f64).min((small_w - 1) as f64) as usize;
            out[y * full_w + x] = labels[src_y * small_w + src_x];
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downsample_noop_when_small() {
        let pixels = vec![Rgb::new(255, 0, 0); 4];
        let (out, w, h) = downsample(&pixels, 2, 2, 512);
        assert_eq!(w, 2);
        assert_eq!(h, 2);
        assert_eq!(out.len(), 4);
    }

    #[test]
    fn downsample_shrinks_large_image() {
        let pixels = vec![Rgb::new(0, 0, 0); 1024 * 768];
        let (out, w, h) = downsample(&pixels, 1024, 768, 512);
        assert!(w <= 512);
        assert!(h <= 512);
        assert_eq!(out.len(), w * h);
    }

    #[test]
    fn compute_averages_basic() {
        let pixels = vec![
            Rgb::new(100, 0, 0),
            Rgb::new(200, 0, 0),
            Rgb::new(0, 100, 0),
            Rgb::new(0, 200, 0),
        ];
        let labels = vec![0, 0, 1, 1];
        let avgs = compute_superpixel_averages(&pixels, &labels, 2);
        assert_eq!(avgs.len(), 2);
        assert_eq!(avgs[0], Rgb::new(150, 0, 0));
        assert_eq!(avgs[1], Rgb::new(0, 150, 0));
    }

    #[test]
    fn compute_averages_skips_empty_labels() {
        let pixels = vec![Rgb::new(100, 100, 100); 4];
        let labels = vec![0, 0, 2, 2];
        let avgs = compute_superpixel_averages(&pixels, &labels, 3);
        assert_eq!(avgs.len(), 2);
    }

    #[test]
    fn upscale_labels_noop() {
        let labels = vec![0, 1, 2, 3];
        let out = upscale_labels(&labels, 2, 2, 2, 2);
        assert_eq!(out, labels);
    }

    #[test]
    fn upscale_labels_doubles() {
        let labels = vec![0, 1, 2, 3];
        let out = upscale_labels(&labels, 2, 2, 4, 4);
        assert_eq!(out.len(), 16);
        assert_eq!(out[0], 0);
        assert_eq!(out[1], 0);
    }
}
