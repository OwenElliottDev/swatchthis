use crate::color::{Lab, Rgb};
use crate::sample_step;

const MAX_ITERATIONS: u32 = 50;
const CONVERGENCE_THRESHOLD: f32 = 0.5;

/// The colour space used for clustering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KmeansColorSpace {
    /// Cluster using Euclidean distance in RGB space.
    Rgb,
    /// Cluster using Euclidean distance in CIELAB space.
    Lab,
    /// Cluster using CIEDE2000 distance in CIELAB space.
    LabCIEDE2000,
}

/// The centroid initialisation method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitMethod {
    /// Select initial centroids uniformly at random.
    Random,
    /// Use the k-means++ algorithm for smarter initial centroid placement.
    KMeansPlusPlus,
}

/// Extracts `k` dominant colours from the given pixels.
///
/// Returns a vec of `(colour, population)` pairs. Large inputs are
/// automatically subsampled by striding; population counts reflect the
/// sampled set.
///
/// # Example
///
/// ```
/// use swatchthis::color::Rgb;
/// use swatchthis::kmeans::{extract_colors_kmeans, KmeansColorSpace, InitMethod};
///
/// let pixels = vec![Rgb::new(255, 0, 0); 50];
/// let result = extract_colors_kmeans(&pixels, 1, KmeansColorSpace::Rgb, InitMethod::Random, 42);
///
/// assert_eq!(result.len(), 1);
/// assert_eq!(result[0].0, Rgb::new(255, 0, 0));
/// assert_eq!(result[0].1, 50);
/// ```
pub fn extract_colors_kmeans(
    pixels: &[Rgb],
    k: usize,
    color_space: KmeansColorSpace,
    init: InitMethod,
    seed: u64,
) -> Vec<(Rgb, u32)> {
    if pixels.is_empty() || k == 0 {
        return Vec::new();
    }

    let step = sample_step(pixels.len());
    let sampled: Vec<Rgb> = pixels.iter().step_by(step).copied().collect();
    let k = k.min(sampled.len());

    match color_space {
        KmeansColorSpace::Rgb => cluster_rgb(&sampled, k, init, seed),
        KmeansColorSpace::Lab => cluster_lab(&sampled, k, init, seed, Lab::distance_squared),
        KmeansColorSpace::LabCIEDE2000 => {
            cluster_lab(&sampled, k, init, seed, Lab::distance_ciede2000)
        }
    }
}

fn cluster_rgb(pixels: &[Rgb], k: usize, init: InitMethod, seed: u64) -> Vec<(Rgb, u32)> {
    let mut rng = SimpleRng::new(seed);

    let mut centroids = match init {
        InitMethod::Random => init_random_rgb(pixels, k, &mut rng),
        InitMethod::KMeansPlusPlus => init_plusplus_rgb(pixels, k, &mut rng),
    };

    let mut assignments = vec![0usize; pixels.len()];

    for _ in 0..MAX_ITERATIONS {
        for (i, px) in pixels.iter().enumerate() {
            let mut best = 0;
            let mut best_dist = u32::MAX;
            for (j, c) in centroids.iter().enumerate() {
                let d = px.distance_squared(*c);
                if d < best_dist {
                    best_dist = d;
                    best = j;
                }
            }
            assignments[i] = best;
        }

        let mut sums = vec![(0u64, 0u64, 0u64, 0u64); k];
        for (i, px) in pixels.iter().enumerate() {
            let c = assignments[i];
            sums[c].0 += px.r as u64;
            sums[c].1 += px.g as u64;
            sums[c].2 += px.b as u64;
            sums[c].3 += 1;
        }

        let mut max_shift: u32 = 0;
        for (j, (sr, sg, sb, count)) in sums.iter().enumerate() {
            if *count == 0 {
                continue;
            }
            let new = Rgb::new((sr / count) as u8, (sg / count) as u8, (sb / count) as u8);
            max_shift = max_shift.max(centroids[j].distance_squared(new));
            centroids[j] = new;
        }

        if (max_shift as f32) < CONVERGENCE_THRESHOLD {
            break;
        }
    }

    let mut populations = vec![0u32; k];
    for &a in &assignments {
        populations[a] += 1;
    }
    centroids.into_iter().zip(populations).collect()
}

fn init_random_rgb(pixels: &[Rgb], k: usize, rng: &mut SimpleRng) -> Vec<Rgb> {
    let mut centroids = Vec::with_capacity(k);
    let mut indices: Vec<usize> = (0..pixels.len()).collect();
    for i in 0..k {
        let j = i + rng.next_usize() % (indices.len() - i);
        indices.swap(i, j);
        centroids.push(pixels[indices[i]]);
    }
    centroids
}

fn init_plusplus_rgb(pixels: &[Rgb], k: usize, rng: &mut SimpleRng) -> Vec<Rgb> {
    let mut centroids = Vec::with_capacity(k);
    centroids.push(pixels[rng.next_usize() % pixels.len()]);

    let mut dists = vec![u32::MAX; pixels.len()];

    for _ in 1..k {
        let last = *centroids.last().unwrap();
        for (i, px) in pixels.iter().enumerate() {
            dists[i] = dists[i].min(px.distance_squared(last));
        }

        let total: u64 = dists.iter().map(|&d| d as u64).sum();
        if total == 0 {
            break;
        }
        let threshold = rng.next_u64() % total;
        let mut cumulative = 0u64;
        let mut chosen = 0;
        for (i, &d) in dists.iter().enumerate() {
            cumulative += d as u64;
            if cumulative > threshold {
                chosen = i;
                break;
            }
        }
        centroids.push(pixels[chosen]);
    }

    centroids
}

type LabDistFn = fn(Lab, Lab) -> f32;

fn cluster_lab(
    pixels: &[Rgb],
    k: usize,
    init: InitMethod,
    seed: u64,
    dist: LabDistFn,
) -> Vec<(Rgb, u32)> {
    let lab_pixels: Vec<Lab> = pixels.iter().map(|p| p.to_lab()).collect();
    let mut rng = SimpleRng::new(seed);

    let mut centroids = match init {
        InitMethod::Random => init_random_lab(&lab_pixels, k, &mut rng),
        InitMethod::KMeansPlusPlus => init_plusplus_lab(&lab_pixels, k, &mut rng, dist),
    };

    let mut assignments = vec![0usize; lab_pixels.len()];

    for _ in 0..MAX_ITERATIONS {
        for (i, px) in lab_pixels.iter().enumerate() {
            let mut best = 0;
            let mut best_dist = f32::MAX;
            for (j, c) in centroids.iter().enumerate() {
                let d = dist(*px, *c);
                if d < best_dist {
                    best_dist = d;
                    best = j;
                }
            }
            assignments[i] = best;
        }

        let mut sums = vec![(0f64, 0f64, 0f64, 0u64); k];
        for (i, px) in lab_pixels.iter().enumerate() {
            let c = assignments[i];
            sums[c].0 += px.l as f64;
            sums[c].1 += px.a as f64;
            sums[c].2 += px.b as f64;
            sums[c].3 += 1;
        }

        let mut max_shift: f32 = 0.0;
        for (j, (sl, sa, sb, count)) in sums.iter().enumerate() {
            if *count == 0 {
                continue;
            }
            let n = *count as f64;
            let new = Lab::new((sl / n) as f32, (sa / n) as f32, (sb / n) as f32);
            max_shift = max_shift.max(dist(centroids[j], new));
            centroids[j] = new;
        }

        if max_shift < CONVERGENCE_THRESHOLD {
            break;
        }
    }

    let mut populations = vec![0u32; k];
    for &a in &assignments {
        populations[a] += 1;
    }
    centroids
        .into_iter()
        .map(|c| c.to_rgb())
        .zip(populations)
        .collect()
}

fn init_random_lab(pixels: &[Lab], k: usize, rng: &mut SimpleRng) -> Vec<Lab> {
    let mut centroids = Vec::with_capacity(k);
    let mut indices: Vec<usize> = (0..pixels.len()).collect();
    for i in 0..k {
        let j = i + rng.next_usize() % (indices.len() - i);
        indices.swap(i, j);
        centroids.push(pixels[indices[i]]);
    }
    centroids
}

fn init_plusplus_lab(pixels: &[Lab], k: usize, rng: &mut SimpleRng, dist: LabDistFn) -> Vec<Lab> {
    let mut centroids = Vec::with_capacity(k);
    centroids.push(pixels[rng.next_usize() % pixels.len()]);

    let mut dists = vec![f32::MAX; pixels.len()];

    for _ in 1..k {
        let last = *centroids.last().unwrap();
        for (i, px) in pixels.iter().enumerate() {
            dists[i] = dists[i].min(dist(*px, last));
        }

        let total: f64 = dists.iter().map(|&d| d as f64).sum();
        if total == 0.0 {
            break;
        }
        let threshold = (rng.next_u64() as f64 / u64::MAX as f64) * total;
        let mut cumulative = 0f64;
        let mut chosen = 0;
        for (i, &d) in dists.iter().enumerate() {
            cumulative += d as f64;
            if cumulative > threshold {
                chosen = i;
                break;
            }
        }
        centroids.push(pixels[chosen]);
    }

    centroids
}

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    fn next_usize(&mut self) -> usize {
        self.next_u64() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pixels(colors: &[(u8, u8, u8)], count_each: usize) -> Vec<Rgb> {
        colors
            .iter()
            .flat_map(|&(r, g, b)| std::iter::repeat_n(Rgb::new(r, g, b), count_each))
            .collect()
    }

    #[test]
    fn extracts_obvious_clusters_rgb() {
        let pixels = make_pixels(&[(255, 0, 0), (0, 255, 0), (0, 0, 255)], 100);
        let swatches = extract_colors_kmeans(
            &pixels,
            3,
            KmeansColorSpace::Rgb,
            InitMethod::KMeansPlusPlus,
            42,
        );
        assert_eq!(swatches.len(), 3);
        for expected in [
            Rgb::new(255, 0, 0),
            Rgb::new(0, 255, 0),
            Rgb::new(0, 0, 255),
        ] {
            assert!(
                swatches
                    .iter()
                    .any(|(s, _)| s.distance_squared(expected) < 10),
                "missing expected colour {expected:?}"
            );
        }
    }

    #[test]
    fn extracts_obvious_clusters_lab() {
        let pixels = make_pixels(&[(255, 0, 0), (0, 255, 0), (0, 0, 255)], 100);
        let swatches = extract_colors_kmeans(
            &pixels,
            3,
            KmeansColorSpace::Lab,
            InitMethod::KMeansPlusPlus,
            42,
        );
        assert_eq!(swatches.len(), 3);
        for expected in [
            Rgb::new(255, 0, 0),
            Rgb::new(0, 255, 0),
            Rgb::new(0, 0, 255),
        ] {
            assert!(
                swatches
                    .iter()
                    .any(|(s, _)| s.distance_squared(expected) < 100),
                "missing expected colour {expected:?} in {swatches:?}"
            );
        }
    }

    #[test]
    fn random_init_works() {
        let pixels = make_pixels(&[(200, 50, 50), (50, 200, 50)], 50);
        let swatches =
            extract_colors_kmeans(&pixels, 2, KmeansColorSpace::Rgb, InitMethod::Random, 7);
        assert_eq!(swatches.len(), 2);
    }

    #[test]
    fn single_colour() {
        let pixels = make_pixels(&[(42, 42, 42)], 10);
        let swatches = extract_colors_kmeans(
            &pixels,
            1,
            KmeansColorSpace::Rgb,
            InitMethod::KMeansPlusPlus,
            1,
        );
        assert_eq!(swatches.len(), 1);
        assert_eq!(swatches[0].0, Rgb::new(42, 42, 42));
    }

    #[test]
    fn empty_input() {
        assert!(
            extract_colors_kmeans(&[], 5, KmeansColorSpace::Rgb, InitMethod::Random, 0).is_empty()
        );
    }

    #[test]
    fn k_larger_than_pixels() {
        let pixels = vec![Rgb::new(10, 20, 30), Rgb::new(40, 50, 60)];
        let swatches =
            extract_colors_kmeans(&pixels, 100, KmeansColorSpace::Rgb, InitMethod::Random, 1);
        assert_eq!(swatches.len(), 2);
    }
}
