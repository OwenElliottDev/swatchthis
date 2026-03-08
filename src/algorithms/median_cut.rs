use crate::color::Rgb;
use crate::sample_step;

pub fn extract_colors_median_cut(pixels: &[Rgb], k: usize) -> Vec<(Rgb, u32)> {
    if pixels.is_empty() || k == 0 {
        return Vec::new();
    }

    let step = sample_step(pixels.len());
    let sampled: Vec<Rgb> = pixels.iter().step_by(step).copied().collect();
    let k = k.min(sampled.len());

    let mut boxes = vec![ColorBox::from_pixels(&sampled)];

    while boxes.len() < k {
        let (split_idx, _) = boxes
            .iter()
            .enumerate()
            .filter(|(_, b)| b.count > 1)
            .max_by_key(|(_, b)| b.range())
            .unwrap_or((0, &boxes[0]));

        if boxes[split_idx].count <= 1 {
            break;
        }

        let to_split = boxes.swap_remove(split_idx);
        let (a, b) = to_split.split();
        boxes.push(a);
        boxes.push(b);
    }

    boxes
        .iter()
        .filter(|b| b.count > 0)
        .map(|b| (b.average(), b.count as u32))
        .collect()
}

#[derive(Clone)]
struct ColorBox {
    pixels: Vec<Rgb>,
    count: usize,
}

impl ColorBox {
    fn from_pixels(pixels: &[Rgb]) -> Self {
        Self {
            pixels: pixels.to_vec(),
            count: pixels.len(),
        }
    }

    fn ranges(&self) -> (u8, u8, u8) {
        let (mut r_min, mut g_min, mut b_min) = (u8::MAX, u8::MAX, u8::MAX);
        let (mut r_max, mut g_max, mut b_max) = (0u8, 0u8, 0u8);
        for p in &self.pixels {
            r_min = r_min.min(p.r);
            r_max = r_max.max(p.r);
            g_min = g_min.min(p.g);
            g_max = g_max.max(p.g);
            b_min = b_min.min(p.b);
            b_max = b_max.max(p.b);
        }
        (r_max - r_min, g_max - g_min, b_max - b_min)
    }

    fn range(&self) -> u8 {
        let (r, g, b) = self.ranges();
        r.max(g).max(b)
    }

    fn split(mut self) -> (ColorBox, ColorBox) {
        let (r_range, g_range, b_range) = self.ranges();

        if r_range >= g_range && r_range >= b_range {
            self.pixels.sort_unstable_by_key(|p| p.r);
        } else if g_range >= b_range {
            self.pixels.sort_unstable_by_key(|p| p.g);
        } else {
            self.pixels.sort_unstable_by_key(|p| p.b);
        }

        let mid = self.pixels.len() / 2;
        let right = self.pixels.split_off(mid);
        let left = self.pixels;

        (ColorBox::from_pixels(&left), ColorBox::from_pixels(&right))
    }

    fn average(&self) -> Rgb {
        let (mut r, mut g, mut b) = (0u64, 0u64, 0u64);
        for p in &self.pixels {
            r += p.r as u64;
            g += p.g as u64;
            b += p.b as u64;
        }
        let n = self.count as u64;
        Rgb::new((r / n) as u8, (g / n) as u8, (b / n) as u8)
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

    fn total_population(result: &[(Rgb, u32)]) -> u32 {
        result.iter().map(|(_, p)| p).sum()
    }

    #[test]
    fn empty_input() {
        assert!(extract_colors_median_cut(&[], 5).is_empty());
    }

    #[test]
    fn zero_k() {
        let pixels = vec![Rgb::new(255, 0, 0); 10];
        assert!(extract_colors_median_cut(&pixels, 0).is_empty());
    }

    #[test]
    fn single_color() {
        let pixels = make_pixels(&[(42, 42, 42)], 50);
        let result = extract_colors_median_cut(&pixels, 1);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, Rgb::new(42, 42, 42));
        assert_eq!(result[0].1, 50);
    }

    #[test]
    fn extracts_distinct_colors() {
        let pixels = make_pixels(&[(255, 0, 0), (0, 255, 0), (0, 0, 255)], 100);
        let result = extract_colors_median_cut(&pixels, 3);
        assert_eq!(result.len(), 3);
        for pair in result.windows(2) {
            assert!(
                pair[0].0.distance_squared(pair[1].0) > 0,
                "got duplicate colours in result",
            );
        }
    }

    #[test]
    fn population_sums_to_pixel_count() {
        let pixels = make_pixels(&[(200, 50, 50), (50, 200, 50), (50, 50, 200)], 80);
        let result = extract_colors_median_cut(&pixels, 3);
        assert_eq!(total_population(&result), 240);
    }

    #[test]
    fn k_larger_than_pixels() {
        let pixels = vec![Rgb::new(10, 20, 30), Rgb::new(40, 50, 60)];
        let result = extract_colors_median_cut(&pixels, 100);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn k_one() {
        let pixels = make_pixels(&[(100, 150, 200), (200, 100, 50)], 50);
        let result = extract_colors_median_cut(&pixels, 1);
        assert_eq!(result.len(), 1);
        assert_eq!(total_population(&result), 100);
    }
}
