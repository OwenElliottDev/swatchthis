use crate::color::Rgb;
use crate::preprocessors::{DEFAULT_MAX_DIM, downsample, finalize_superpixels};

const SLIC_ITERATIONS: usize = 10;

struct SlicCluster {
    r: f32,
    g: f32,
    b: f32,
    x: f32,
    y: f32,
    count: u32,
}

/// Reduces an image to superpixel average colours using the SLIC algorithm.
///
/// Returns one `Rgb` per superpixel. The result can be passed directly to any
/// palette extraction function (kmeans, octree, median_cut).
///
/// - `num_superpixels`: target number of superpixels (typically 100–400)
/// - `compactness`: controls colour vs spatial weight (typical: 10–40)
pub fn slic_preprocess(
    pixels: &[Rgb],
    width: usize,
    height: usize,
    num_superpixels: usize,
    compactness: f32,
) -> Vec<Rgb> {
    if pixels.is_empty() || width == 0 || height == 0 {
        return Vec::new();
    }

    let num_superpixels = num_superpixels.max(1);

    let (small_pixels, sw, sh) = downsample(pixels, width, height, DEFAULT_MAX_DIM);
    let n = sw * sh;
    let k = num_superpixels.min(n);

    let s = ((n as f64 / k as f64).sqrt()) as usize;
    let s = s.max(1);

    let mut centers = init_centers(&small_pixels, sw, sh, s);

    for c in &mut centers {
        let (nx, ny) = lowest_gradient(&small_pixels, sw, sh, c.x as usize, c.y as usize);
        let px = small_pixels[ny * sw + nx];
        c.x = nx as f32;
        c.y = ny as f32;
        c.r = px.r as f32;
        c.g = px.g as f32;
        c.b = px.b as f32;
    }

    let mut labels = vec![u32::MAX; n];
    let mut distances = vec![f32::MAX; n];
    let spatial_weight = compactness / s as f32;
    let spatial_weight_sq = spatial_weight * spatial_weight;

    for _ in 0..SLIC_ITERATIONS {
        distances.fill(f32::MAX);

        for (ci, center) in centers.iter().enumerate() {
            let cx = center.x as isize;
            let cy = center.y as isize;
            let search = s as isize;

            let y_start = (cy - search).max(0) as usize;
            let y_end = ((cy + search) as usize + 1).min(sh);
            let x_start = (cx - search).max(0) as usize;
            let x_end = ((cx + search) as usize + 1).min(sw);

            for y in y_start..y_end {
                for x in x_start..x_end {
                    let idx = y * sw + x;
                    let px = small_pixels[idx];

                    let dr = px.r as f32 - center.r;
                    let dg = px.g as f32 - center.g;
                    let db = px.b as f32 - center.b;
                    let d_rgb_sq = dr * dr + dg * dg + db * db;

                    let dx = x as f32 - center.x;
                    let dy = y as f32 - center.y;
                    let d_spatial_sq = dx * dx + dy * dy;

                    let dist = d_rgb_sq + spatial_weight_sq * d_spatial_sq;

                    if dist < distances[idx] {
                        distances[idx] = dist;
                        labels[idx] = ci as u32;
                    }
                }
            }
        }

        for c in &mut centers {
            c.r = 0.0;
            c.g = 0.0;
            c.b = 0.0;
            c.x = 0.0;
            c.y = 0.0;
            c.count = 0;
        }

        for y in 0..sh {
            for x in 0..sw {
                let idx = y * sw + x;
                let label = labels[idx];
                if label == u32::MAX {
                    continue;
                }
                let c = &mut centers[label as usize];
                let px = small_pixels[idx];
                c.r += px.r as f32;
                c.g += px.g as f32;
                c.b += px.b as f32;
                c.x += x as f32;
                c.y += y as f32;
                c.count += 1;
            }
        }

        for c in &mut centers {
            if c.count > 0 {
                let inv = 1.0 / c.count as f32;
                c.r *= inv;
                c.g *= inv;
                c.b *= inv;
                c.x *= inv;
                c.y *= inv;
            }
        }
    }

    for label in labels.iter_mut() {
        if *label == u32::MAX {
            *label = 0;
        }
    }

    let min_size = (s * s) / 4;
    enforce_connectivity(&mut labels, sw, sh, min_size);

    finalize_superpixels(pixels, &labels, sw, sh, width, height)
}

fn init_centers(pixels: &[Rgb], width: usize, height: usize, s: usize) -> Vec<SlicCluster> {
    let mut centers = Vec::new();
    let half_s = s / 2;

    let mut y = half_s;
    while y < height {
        let mut x = half_s;
        while x < width {
            let px = pixels[y * width + x];
            centers.push(SlicCluster {
                r: px.r as f32,
                g: px.g as f32,
                b: px.b as f32,
                x: x as f32,
                y: y as f32,
                count: 0,
            });
            x += s;
        }
        y += s;
    }

    if centers.is_empty() {
        let cx = width / 2;
        let cy = height / 2;
        let px = pixels[cy * width + cx];
        centers.push(SlicCluster {
            r: px.r as f32,
            g: px.g as f32,
            b: px.b as f32,
            x: cx as f32,
            y: cy as f32,
            count: 0,
        });
    }

    centers
}

fn gradient(pixels: &[Rgb], width: usize, height: usize, x: usize, y: usize) -> f32 {
    let get = |px: usize, py: usize| &pixels[py * width + px];

    let (lx, rx) = if x > 0 && x + 1 < width {
        (get(x - 1, y), get(x + 1, y))
    } else if x + 1 < width {
        (get(x, y), get(x + 1, y))
    } else if x > 0 {
        (get(x - 1, y), get(x, y))
    } else {
        return 0.0;
    };

    let (ty, by) = if y > 0 && y + 1 < height {
        (get(x, y - 1), get(x, y + 1))
    } else if y + 1 < height {
        (get(x, y), get(x, y + 1))
    } else if y > 0 {
        (get(x, y - 1), get(x, y))
    } else {
        return 0.0;
    };

    let dr = rx.r as f32 - lx.r as f32;
    let dg = rx.g as f32 - lx.g as f32;
    let db = rx.b as f32 - lx.b as f32;
    let horiz = dr * dr + dg * dg + db * db;

    let dr = by.r as f32 - ty.r as f32;
    let dg = by.g as f32 - ty.g as f32;
    let db = by.b as f32 - ty.b as f32;
    let vert = dr * dr + dg * dg + db * db;

    horiz + vert
}

fn lowest_gradient(
    pixels: &[Rgb],
    width: usize,
    height: usize,
    cx: usize,
    cy: usize,
) -> (usize, usize) {
    let mut best_x = cx;
    let mut best_y = cy;
    let mut best_grad = f32::MAX;

    let y_start = if cy > 0 { cy - 1 } else { 0 };
    let y_end = (cy + 2).min(height);
    let x_start = if cx > 0 { cx - 1 } else { 0 };
    let x_end = (cx + 2).min(width);

    for y in y_start..y_end {
        for x in x_start..x_end {
            let g = gradient(pixels, width, height, x, y);
            if g < best_grad {
                best_grad = g;
                best_x = x;
                best_y = y;
            }
        }
    }

    (best_x, best_y)
}

fn enforce_connectivity(labels: &mut [u32], width: usize, height: usize, min_size: usize) {
    let n = width * height;
    let mut new_labels = vec![u32::MAX; n];
    let mut label_counter = 0u32;

    let dx: [isize; 4] = [1, -1, 0, 0];
    let dy: [isize; 4] = [0, 0, 1, -1];

    let mut queue = Vec::new();

    for i in 0..n {
        if new_labels[i] != u32::MAX {
            continue;
        }

        queue.clear();
        queue.push(i);
        new_labels[i] = label_counter;
        let original_label = labels[i];
        let mut head = 0;

        while head < queue.len() {
            let cur = queue[head];
            head += 1;

            let cx = cur % width;
            let cy = cur / width;

            for d in 0..4 {
                let nx = cx as isize + dx[d];
                let ny = cy as isize + dy[d];

                if nx < 0 || ny < 0 || nx >= width as isize || ny >= height as isize {
                    continue;
                }

                let ni = ny as usize * width + nx as usize;
                if new_labels[ni] != u32::MAX {
                    continue;
                }

                if labels[ni] == original_label {
                    new_labels[ni] = label_counter;
                    queue.push(ni);
                }
            }
        }

        if queue.len() < min_size {
            let component_y = (queue[0] / width) as f32;
            let component_x = (queue[0] % width) as f32;

            let mut best_label = None;
            let mut best_dist = f32::MAX;

            for &idx in &queue {
                let px = idx % width;
                let py = idx / width;
                for d in 0..4 {
                    let nx = px as isize + dx[d];
                    let ny = py as isize + dy[d];
                    if nx < 0 || ny < 0 || nx >= width as isize || ny >= height as isize {
                        continue;
                    }
                    let ni = ny as usize * width + nx as usize;
                    if new_labels[ni] != u32::MAX && new_labels[ni] != label_counter {
                        let ddx = component_x - (nx as f32);
                        let ddy = component_y - (ny as f32);
                        let dist = ddx * ddx + ddy * ddy;
                        if dist < best_dist {
                            best_dist = dist;
                            best_label = Some(new_labels[ni]);
                        }
                    }
                }
            }

            if let Some(adj_label) = best_label {
                for &idx in &queue {
                    new_labels[idx] = adj_label;
                }
                continue;
            }
        }

        label_counter += 1;
    }

    labels.copy_from_slice(&new_labels);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_image() {
        let color = Rgb::new(128, 64, 32);
        let pixels = vec![color; 100];
        let result = slic_preprocess(&pixels, 10, 10, 4, 10.0);
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

        let result = slic_preprocess(&pixels, width, height, 4, 10.0);
        assert!(!result.is_empty());

        let has_red = result.iter().any(|c| c.r > 200 && c.b < 50);
        let has_blue = result.iter().any(|c| c.b > 200 && c.r < 50);
        assert!(has_red, "Expected red superpixels, got: {:?}", result);
        assert!(has_blue, "Expected blue superpixels, got: {:?}", result);
    }

    #[test]
    fn tiny_image_1x1() {
        let pixels = vec![Rgb::new(42, 42, 42)];
        let result = slic_preprocess(&pixels, 1, 1, 1, 10.0);
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
        let result = slic_preprocess(&pixels, 2, 2, 4, 10.0);
        assert!(!result.is_empty());
    }

    #[test]
    fn empty_image() {
        let result = slic_preprocess(&[], 0, 0, 4, 10.0);
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

        let preprocessed = slic_preprocess(&pixels, width, height, 8, 10.0);
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
