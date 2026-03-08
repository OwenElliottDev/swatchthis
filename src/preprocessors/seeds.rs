use crate::color::Rgb;
use crate::preprocessors::{DEFAULT_MAX_DIM, downsample, finalize_superpixels};

/// Reduces an image to superpixel average colours using the SEEDS algorithm.
///
/// Returns one `Rgb` per superpixel. The result can be passed directly to any
/// palette extraction function (kmeans, octree, median_cut).
///
/// - `num_superpixels`: target number of superpixels (typically 100–400)
/// - `num_levels`: hierarchical refinement levels (typical: 3–5)
/// - `histogram_bins`: bins per colour channel for histograms (typical: 5)
pub fn seeds_preprocess(
    pixels: &[Rgb],
    width: usize,
    height: usize,
    num_superpixels: usize,
    num_levels: usize,
    histogram_bins: usize,
) -> Vec<Rgb> {
    if pixels.is_empty() || width == 0 || height == 0 {
        return Vec::new();
    }

    let num_superpixels = num_superpixels.max(1);
    let num_levels = num_levels.max(1);
    let histogram_bins = histogram_bins.max(1);

    let (small_pixels, sw, sh) = downsample(pixels, width, height, DEFAULT_MAX_DIM);
    let n = sw * sh;
    let k = num_superpixels.min(n);

    let block_area = (n as f64 / k as f64).sqrt();
    let blocks_x = ((sw as f64 / block_area).round() as usize).max(1);
    let blocks_y = ((sh as f64 / block_area).round() as usize).max(1);
    let block_w = sw.div_ceil(blocks_x);
    let block_h = sh.div_ceil(blocks_y);

    let mut labels = vec![0u32; n];
    let mut num_labels = 0u32;
    for y in 0..sh {
        let by = (y / block_h).min(blocks_y - 1);
        for x in 0..sw {
            let bx = (x / block_w).min(blocks_x - 1);
            let label = (by * blocks_x + bx) as u32;
            labels[y * sw + x] = label;
            if label >= num_labels {
                num_labels = label + 1;
            }
        }
    }

    let mut histograms =
        build_histograms(&small_pixels, &labels, num_labels as usize, histogram_bins);
    let total_bins = histogram_bins * histogram_bins * histogram_bins;
    let mut sub_hist = vec![0u32; total_bins];

    for level in 0..num_levels {
        let divisor = 1usize << (num_levels - 1 - level);
        let sub_w = (block_w / divisor).max(1);
        let sub_h = (block_h / divisor).max(1);

        let sub_blocks_x = sw.div_ceil(sub_w);
        let sub_blocks_y = sh.div_ceil(sub_h);

        for sby in 0..sub_blocks_y {
            for sbx in 0..sub_blocks_x {
                let x0 = sbx * sub_w;
                let y0 = sby * sub_h;
                let x1 = (x0 + sub_w).min(sw);
                let y1 = (y0 + sub_h).min(sh);

                let current_label = center_label(&labels, sw, x0, y0, x1, y1);

                let neighbor_label =
                    find_neighbor_label(&labels, sw, sh, x0, y0, x1, y1, current_label);

                if let Some(neighbor) = neighbor_label {
                    fill_sub_histogram(
                        &small_pixels,
                        sw,
                        x0,
                        y0,
                        x1,
                        y1,
                        histogram_bins,
                        &mut sub_hist,
                    );

                    let score_current =
                        histogram_intersection(&sub_hist, &histograms[current_label as usize]);
                    let score_neighbor =
                        histogram_intersection(&sub_hist, &histograms[neighbor as usize]);

                    if score_neighbor > score_current + 0.1 {
                        update_histograms(
                            &mut histograms,
                            &sub_hist,
                            current_label as usize,
                            neighbor as usize,
                        );
                        relabel_block(&mut labels, sw, x0, y0, x1, y1, neighbor);
                    }
                }
            }
        }
    }

    pixel_level_refinement(
        &mut labels,
        &small_pixels,
        &mut histograms,
        sw,
        sh,
        histogram_bins,
    );

    finalize_superpixels(pixels, &labels, sw, sh, width, height)
}

fn build_histograms(
    pixels: &[Rgb],
    labels: &[u32],
    num_labels: usize,
    bins: usize,
) -> Vec<Vec<u32>> {
    let total_bins = bins * bins * bins;
    let mut histograms = vec![vec![0u32; total_bins]; num_labels];

    for (px, &label) in pixels.iter().zip(labels.iter()) {
        let bin = rgb_to_bin(px, bins);
        histograms[label as usize][bin] += 1;
    }

    histograms
}

fn rgb_to_bin(px: &Rgb, bins: usize) -> usize {
    let rb = (px.r as usize * bins / 256).min(bins - 1);
    let gb = (px.g as usize * bins / 256).min(bins - 1);
    let bb = (px.b as usize * bins / 256).min(bins - 1);
    rb * bins * bins + gb * bins + bb
}

fn histogram_intersection(a: &[u32], b: &[u32]) -> f32 {
    let mut a_sum = 0u64;
    let mut b_sum = 0u64;
    for (&av, &bv) in a.iter().zip(b.iter()) {
        a_sum += av as u64;
        b_sum += bv as u64;
    }

    if a_sum == 0 || b_sum == 0 {
        return 0.0;
    }

    // min(a/sum_a, b/sum_b) = min(a*sum_b, b*sum_a) / (sum_a * sum_b)
    let denom = a_sum as f64 * b_sum as f64;
    let mut numer = 0u128;
    for (&av, &bv) in a.iter().zip(b.iter()) {
        let scaled_a = av as u64 * b_sum;
        let scaled_b = bv as u64 * a_sum;
        numer += scaled_a.min(scaled_b) as u128;
    }

    (numer as f64 / denom) as f32
}

#[allow(clippy::too_many_arguments)]
fn fill_sub_histogram(
    pixels: &[Rgb],
    width: usize,
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    bins: usize,
    hist: &mut [u32],
) {
    hist.fill(0);
    for y in y0..y1 {
        for x in x0..x1 {
            let px = &pixels[y * width + x];
            let bin = rgb_to_bin(px, bins);
            hist[bin] += 1;
        }
    }
}

fn center_label(labels: &[u32], width: usize, x0: usize, y0: usize, x1: usize, y1: usize) -> u32 {
    let cy = (y0 + y1) / 2;
    let cx = (x0 + x1) / 2;
    labels[cy * width + cx]
}

#[allow(clippy::too_many_arguments)]
fn find_neighbor_label(
    labels: &[u32],
    width: usize,
    height: usize,
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    current_label: u32,
) -> Option<u32> {
    if y0 > 0 {
        for x in x0..x1 {
            let l = labels[(y0 - 1) * width + x];
            if l != current_label {
                return Some(l);
            }
        }
    }
    if y1 < height {
        for x in x0..x1 {
            let l = labels[y1 * width + x];
            if l != current_label {
                return Some(l);
            }
        }
    }
    if x0 > 0 {
        for y in y0..y1 {
            let l = labels[y * width + (x0 - 1)];
            if l != current_label {
                return Some(l);
            }
        }
    }
    if x1 < width {
        for y in y0..y1 {
            let l = labels[y * width + x1];
            if l != current_label {
                return Some(l);
            }
        }
    }
    None
}

fn relabel_block(
    labels: &mut [u32],
    width: usize,
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    new_label: u32,
) {
    for y in y0..y1 {
        for x in x0..x1 {
            labels[y * width + x] = new_label;
        }
    }
}

fn update_histograms(
    histograms: &mut [Vec<u32>],
    sub_hist: &[u32],
    old_label: usize,
    new_label: usize,
) {
    for (i, &count) in sub_hist.iter().enumerate() {
        histograms[old_label][i] = histograms[old_label][i].saturating_sub(count);
        histograms[new_label][i] += count;
    }
}

fn pixel_level_refinement(
    labels: &mut [u32],
    pixels: &[Rgb],
    histograms: &mut [Vec<u32>],
    width: usize,
    height: usize,
    bins: usize,
) {
    let dx: [isize; 4] = [1, -1, 0, 0];
    let dy: [isize; 4] = [0, 0, 1, -1];

    let mut totals: Vec<u64> = histograms
        .iter()
        .map(|h| h.iter().map(|&v| v as u64).sum())
        .collect();

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let current = labels[idx];
            let px = &pixels[idx];
            let bin = rgb_to_bin(px, bins);

            let current_total = totals[current as usize];
            if current_total == 0 {
                continue;
            }
            let mut best_score = histograms[current as usize][bin] as f32 / current_total as f32;
            let mut best_neighbor = None;

            for d in 0..4 {
                let nx = x as isize + dx[d];
                let ny = y as isize + dy[d];

                if nx < 0 || ny < 0 || nx >= width as isize || ny >= height as isize {
                    continue;
                }

                let ni = ny as usize * width + nx as usize;
                let neighbor = labels[ni];
                if neighbor == current {
                    continue;
                }

                let neighbor_total = totals[neighbor as usize];
                if neighbor_total == 0 {
                    continue;
                }
                let score = histograms[neighbor as usize][bin] as f32 / neighbor_total as f32;
                if score > best_score {
                    best_score = score;
                    best_neighbor = Some(neighbor);
                }
            }

            if let Some(new_label) = best_neighbor {
                histograms[current as usize][bin] =
                    histograms[current as usize][bin].saturating_sub(1);
                histograms[new_label as usize][bin] += 1;
                totals[current as usize] = totals[current as usize].saturating_sub(1);
                totals[new_label as usize] += 1;
                labels[idx] = new_label;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_image() {
        let color = Rgb::new(128, 64, 32);
        let pixels = vec![color; 100];
        let result = seeds_preprocess(&pixels, 10, 10, 4, 3, 5);
        for c in &result {
            assert_eq!(*c, color);
        }
    }

    #[test]
    fn two_color_split() {
        let width = 20;
        let height = 10;
        let mut pixels = Vec::with_capacity(width * height);
        for _y in 0..height {
            for x in 0..width {
                if x < width / 2 {
                    pixels.push(Rgb::new(255, 0, 0));
                } else {
                    pixels.push(Rgb::new(0, 0, 255));
                }
            }
        }

        let result = seeds_preprocess(&pixels, width, height, 4, 3, 5);
        assert!(!result.is_empty());

        let has_red = result.iter().any(|c| c.r > 200 && c.b < 50);
        let has_blue = result.iter().any(|c| c.b > 200 && c.r < 50);
        assert!(has_red, "Expected red superpixels, got: {:?}", result);
        assert!(has_blue, "Expected blue superpixels, got: {:?}", result);
    }

    #[test]
    fn tiny_image_1x1() {
        let pixels = vec![Rgb::new(42, 42, 42)];
        let result = seeds_preprocess(&pixels, 1, 1, 1, 2, 3);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], Rgb::new(42, 42, 42));
    }

    #[test]
    fn tiny_image_2x2() {
        let pixels = vec![
            Rgb::new(255, 0, 0),
            Rgb::new(0, 255, 0),
            Rgb::new(0, 0, 255),
            Rgb::new(255, 255, 0),
        ];
        let result = seeds_preprocess(&pixels, 2, 2, 4, 2, 3);
        assert!(!result.is_empty());
    }

    #[test]
    fn empty_image() {
        let result = seeds_preprocess(&[], 0, 0, 4, 3, 5);
        assert!(result.is_empty());
    }

    #[test]
    fn integration_with_kmeans() {
        use crate::algorithms::kmeans::{InitMethod, KmeansColorSpace};
        use crate::generate_swatches_kmeans;

        let width = 20;
        let height = 20;
        let mut pixels = Vec::with_capacity(width * height);
        for y in 0..height {
            for _x in 0..width {
                if y < height / 2 {
                    pixels.push(Rgb::new(255, 0, 0));
                } else {
                    pixels.push(Rgb::new(0, 0, 255));
                }
            }
        }

        let preprocessed = seeds_preprocess(&pixels, width, height, 8, 3, 5);
        let swatches = generate_swatches_kmeans(
            &preprocessed,
            2,
            KmeansColorSpace::Rgb,
            InitMethod::KMeansPlusPlus,
            42,
        );

        assert_eq!(swatches.len(), 2);
    }
}
