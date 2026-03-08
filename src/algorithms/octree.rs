use crate::color::{ColorChannels, Lab, Rgb};
use crate::sample_step;

pub enum OctreeColorSpace {
    Rgb,
    Lab,
}

/// Tree depth for the octree quantiser. Valid range is 1–8, matching the
/// 8 bits per colour channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OctreeDepth {
    D1 = 1,
    D2 = 2,
    D3 = 3,
    D4 = 4,
    D5 = 5,
    D6 = 6,
    D7 = 7,
    D8 = 8,
}

impl OctreeDepth {
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => Self::D1,
            2 => Self::D2,
            3 => Self::D3,
            4 => Self::D4,
            5 => Self::D5,
            6 => Self::D6,
            7 => Self::D7,
            _ => Self::D8,
        }
    }

    fn as_usize(self) -> usize {
        self as usize
    }
}

pub fn extract_colors_octree(
    pixels: &[Rgb],
    k: usize,
    color_space: OctreeColorSpace,
    max_depth: OctreeDepth,
) -> Vec<(Rgb, u32)> {
    if pixels.is_empty() || k == 0 {
        return Vec::new();
    }

    let depth = max_depth.as_usize();
    match color_space {
        OctreeColorSpace::Rgb => octree_rgb(pixels, k, depth),
        OctreeColorSpace::Lab => octree_lab(pixels, k, depth),
    }
}

#[derive(Default)]
struct OctreeNode {
    children: [Option<usize>; 8],
    pixel_count: u32,
    channel_sum: [i64; 3],
    is_leaf: bool,
}

struct Octree {
    nodes: Vec<OctreeNode>,
    reducible: [Vec<usize>; 8],
    leaf_count: usize,
}

impl Octree {
    fn new(max_depth: usize) -> Self {
        let root = OctreeNode {
            is_leaf: max_depth == 0,
            ..Default::default()
        };
        let leaf_count = if root.is_leaf { 1 } else { 0 };
        Self {
            nodes: vec![root],
            reducible: Default::default(),
            leaf_count,
        }
    }

    fn alloc(&mut self, depth: usize, max_depth: usize) -> usize {
        let id = self.nodes.len();
        let is_leaf = depth >= max_depth;
        self.nodes.push(OctreeNode {
            is_leaf,
            ..Default::default()
        });
        if is_leaf {
            self.leaf_count += 1;
        } else {
            self.reducible[depth].push(id);
        }
        id
    }

    fn insert<C: ColorChannels + Copy>(&mut self, color: C, max_depth: usize, max_colors: usize) {
        let mut node_id = 0; // root
        for depth in 0..max_depth {
            let idx = get_color_index(color, depth);

            let child_id = match self.nodes[node_id].children[idx] {
                Some(id) => id,
                None => {
                    let id = self.alloc(depth + 1, max_depth);
                    self.nodes[node_id].children[idx] = Some(id);
                    id
                }
            };

            node_id = child_id;

            if self.nodes[node_id].is_leaf {
                break;
            }
        }

        let (c0, c1, c2) = color.channels();
        let node = &mut self.nodes[node_id];
        node.pixel_count += 1;
        node.channel_sum[0] += c0 as i64;
        node.channel_sum[1] += c1 as i64;
        node.channel_sum[2] += c2 as i64;

        while self.leaf_count > max_colors {
            if !self.reduce() {
                break;
            }
        }
    }

    fn reduce(&mut self) -> bool {
        let Some(depth) = (0..8).rev().find(|&d| !self.reducible[d].is_empty()) else {
            return false;
        };

        let node_id = self.reducible[depth].pop().unwrap();

        let child_ids: Vec<usize> = self.nodes[node_id]
            .children
            .iter()
            .filter_map(|&c| c)
            .collect();

        for child_id in child_ids {
            let child = &self.nodes[child_id];
            let pc = child.pixel_count;
            let cs = child.channel_sum;
            let was_leaf = child.is_leaf;

            let node = &mut self.nodes[node_id];
            node.pixel_count += pc;
            node.channel_sum[0] += cs[0];
            node.channel_sum[1] += cs[1];
            node.channel_sum[2] += cs[2];

            if was_leaf {
                self.leaf_count -= 1;
            }
        }

        self.nodes[node_id].children = [None; 8];
        self.nodes[node_id].is_leaf = true;
        self.leaf_count += 1;
        true
    }

    fn collect_rgb_palette(&self) -> Vec<(Rgb, u32)> {
        let mut palette = Vec::new();
        self.walk_rgb(0, &mut palette);
        palette
    }

    fn walk_rgb(&self, node_id: usize, out: &mut Vec<(Rgb, u32)>) {
        let node = &self.nodes[node_id];
        if node.is_leaf {
            if node.pixel_count > 0 {
                let count = node.pixel_count as i64;
                let r = (node.channel_sum[0] / count) as u8;
                let g = (node.channel_sum[1] / count) as u8;
                let b = (node.channel_sum[2] / count) as u8;
                out.push((Rgb { r, g, b }, node.pixel_count));
            }
        } else {
            for &child in node.children.iter().flatten() {
                self.walk_rgb(child, out);
            }
        }
    }

    fn collect_lab_palette(&self) -> Vec<(Rgb, u32)> {
        let mut palette = Vec::new();
        self.walk_lab(0, &mut palette);
        palette
    }

    fn walk_lab(&self, node_id: usize, out: &mut Vec<(Rgb, u32)>) {
        let node = &self.nodes[node_id];
        if node.is_leaf {
            if node.pixel_count > 0 {
                let count = node.pixel_count as f32;
                let lab = Lab {
                    l: node.channel_sum[0] as f32 / count,
                    a: node.channel_sum[1] as f32 / count,
                    b: node.channel_sum[2] as f32 / count,
                };
                out.push((lab.to_rgb(), node.pixel_count));
            }
        } else {
            for &child in node.children.iter().flatten() {
                self.walk_lab(child, out);
            }
        }
    }
}

fn octree_rgb(pixels: &[Rgb], k: usize, max_depth: usize) -> Vec<(Rgb, u32)> {
    let step = sample_step(pixels.len());
    let mut tree = Octree::new(max_depth);
    for &p in pixels.iter().step_by(step) {
        tree.insert(p, max_depth, k);
    }
    let mut palette = tree.collect_rgb_palette();
    palette.sort_by(|a, b| b.1.cmp(&a.1));
    palette.truncate(k);
    palette
}

fn octree_lab(pixels: &[Rgb], k: usize, max_depth: usize) -> Vec<(Rgb, u32)> {
    let step = sample_step(pixels.len());
    let mut tree = Octree::new(max_depth);
    for &p in pixels.iter().step_by(step) {
        let lab = p.to_lab();
        tree.insert(lab, max_depth, k);
    }
    let mut palette = tree.collect_lab_palette();
    palette.sort_by(|a, b| b.1.cmp(&a.1));
    palette.truncate(k);
    palette
}

fn get_color_index<C: ColorChannels>(color: C, level: usize) -> usize {
    let (c0, c1, c2) = color.channels();
    let mask = 0b10000000 >> level;
    let mut index = 0;
    if c0 as i32 & mask != 0 {
        index |= 0b100;
    }
    if c1 as i32 & mask != 0 {
        index |= 0b010;
    }
    if c2 as i32 & mask != 0 {
        index |= 0b001;
    }
    index
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
        assert!(extract_colors_octree(&[], 5, OctreeColorSpace::Rgb, OctreeDepth::D6).is_empty());
    }

    #[test]
    fn zero_k() {
        let pixels = vec![Rgb::new(255, 0, 0); 10];
        assert!(
            extract_colors_octree(&pixels, 0, OctreeColorSpace::Rgb, OctreeDepth::D6).is_empty()
        );
    }

    #[test]
    fn single_color_rgb() {
        let pixels = make_pixels(&[(42, 42, 42)], 50);
        let result = extract_colors_octree(&pixels, 1, OctreeColorSpace::Rgb, OctreeDepth::D6);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, Rgb::new(42, 42, 42));
        assert_eq!(result[0].1, 50);
    }

    #[test]
    fn single_color_lab() {
        let pixels = make_pixels(&[(42, 42, 42)], 50);
        let result = extract_colors_octree(&pixels, 1, OctreeColorSpace::Lab, OctreeDepth::D6);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, 50);
        let c = result[0].0;
        assert!(
            Rgb::new(42, 42, 42).distance_squared(c) < 10,
            "expected ~(42,42,42), got ({},{},{})",
            c.r,
            c.g,
            c.b,
        );
    }

    #[test]
    fn extracts_obvious_clusters_rgb() {
        let pixels = make_pixels(&[(255, 0, 0), (0, 255, 0), (0, 0, 255)], 100);
        let result = extract_colors_octree(&pixels, 3, OctreeColorSpace::Rgb, OctreeDepth::D6);
        assert_eq!(result.len(), 3);
        for expected in [
            Rgb::new(255, 0, 0),
            Rgb::new(0, 255, 0),
            Rgb::new(0, 0, 255),
        ] {
            assert!(
                result
                    .iter()
                    .any(|(c, _)| c.distance_squared(expected) < 10),
                "missing expected colour {expected:?}",
            );
        }
    }

    #[test]
    fn extracts_obvious_clusters_lab() {
        let pixels = make_pixels(&[(255, 0, 0), (0, 255, 0), (0, 0, 255)], 100);
        let result = extract_colors_octree(&pixels, 3, OctreeColorSpace::Lab, OctreeDepth::D6);
        assert_eq!(result.len(), 3);
        for expected in [
            Rgb::new(255, 0, 0),
            Rgb::new(0, 255, 0),
            Rgb::new(0, 0, 255),
        ] {
            assert!(
                result
                    .iter()
                    .any(|(c, _)| c.distance_squared(expected) < 400),
                "missing expected colour {expected:?} in {result:?}",
            );
        }
    }

    #[test]
    fn respects_k_limit() {
        let pixels = make_pixels(
            &[
                (255, 0, 0),
                (0, 255, 0),
                (0, 0, 255),
                (255, 255, 0),
                (255, 0, 255),
                (0, 255, 255),
                (128, 128, 128),
                (0, 0, 0),
            ],
            100,
        );
        let result = extract_colors_octree(&pixels, 4, OctreeColorSpace::Rgb, OctreeDepth::D6);
        assert!(
            result.len() <= 4,
            "got {} colors, expected <= 4",
            result.len()
        );
    }

    #[test]
    fn k_larger_than_distinct_colors() {
        let pixels = make_pixels(&[(10, 20, 30), (40, 50, 60)], 50);
        let result = extract_colors_octree(&pixels, 100, OctreeColorSpace::Rgb, OctreeDepth::D6);
        assert!(result.len() <= 2);
        assert_eq!(total_population(&result), 100);
    }

    #[test]
    fn population_sums_to_pixel_count() {
        let pixels = make_pixels(&[(200, 50, 50), (50, 200, 50), (50, 50, 200)], 80);
        let result = extract_colors_octree(&pixels, 3, OctreeColorSpace::Rgb, OctreeDepth::D6);
        assert_eq!(total_population(&result), 240);
    }

    #[test]
    fn sorted_by_population_descending() {
        let mut pixels = make_pixels(&[(255, 0, 0)], 200);
        pixels.extend(make_pixels(&[(0, 255, 0)], 100));
        pixels.extend(make_pixels(&[(0, 0, 255)], 50));
        let result = extract_colors_octree(&pixels, 3, OctreeColorSpace::Rgb, OctreeDepth::D6);
        for w in result.windows(2) {
            assert!(w[0].1 >= w[1].1, "palette not sorted by population");
        }
    }

    #[test]
    fn k_fewer_than_octree_branches() {
        let pixels = make_pixels(
            &[
                (255, 0, 0),
                (0, 255, 0),
                (0, 0, 255),
                (255, 255, 0),
                (255, 0, 255),
                (0, 255, 255),
                (128, 128, 128),
                (255, 255, 255),
            ],
            100,
        );
        let result = extract_colors_octree(&pixels, 2, OctreeColorSpace::Rgb, OctreeDepth::D6);
        assert!(result.len() <= 2);
    }

    #[test]
    fn k_one() {
        let pixels = make_pixels(&[(100, 150, 200), (200, 100, 50)], 50);
        let result = extract_colors_octree(&pixels, 1, OctreeColorSpace::Rgb, OctreeDepth::D6);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn different_depths() {
        let pixels = make_pixels(&[(255, 0, 0), (0, 255, 0)], 50);
        for depth in [
            OctreeDepth::D1,
            OctreeDepth::D2,
            OctreeDepth::D3,
            OctreeDepth::D4,
            OctreeDepth::D5,
            OctreeDepth::D6,
            OctreeDepth::D7,
            OctreeDepth::D8,
        ] {
            let result = extract_colors_octree(&pixels, 2, OctreeColorSpace::Rgb, depth);
            assert!(!result.is_empty(), "empty result at depth {depth:?}");
            assert_eq!(total_population(&result), 100);
        }
    }

    #[test]
    fn lab_preserves_population() {
        let pixels = make_pixels(&[(200, 50, 50), (50, 200, 50)], 60);
        let result = extract_colors_octree(&pixels, 2, OctreeColorSpace::Lab, OctreeDepth::D6);
        assert_eq!(total_population(&result), 120);
    }

    #[test]
    fn many_similar_colors() {
        let pixels: Vec<Rgb> = (0..50)
            .map(|i| Rgb::new(100 + i, 100 + i, 100 + i))
            .collect();
        let result = extract_colors_octree(&pixels, 3, OctreeColorSpace::Rgb, OctreeDepth::D6);
        assert!(result.len() <= 3);
        assert_eq!(total_population(&result), 50);
    }
}
