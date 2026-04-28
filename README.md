# swatchthis

A Rust library for extracting dominant colour swatches from images. Works in native Rust and WebAssembly.

#### [[demo](https://owenelliott.dev/swatchthis)]

## Features

- **Three palette algorithms**: k-means (with k-means++ init), octree quantisation, and median cut
- **RGB and CIELAB** colour space clustering
- **CIEDE2000 distance** for CIELAB, slower but good perceptual accuracy
- **Superpixel preprocessors**: SLIC and SEEDS, for reducing an image to region-averaged colours before palette extraction
- **WebAssembly support** via `wasm-bindgen` (behind the `wasm` feature flag)
- **No runtime dependencies** for native builds (deterministic PRNG, no `rand` crate)

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
swatchthis = "0.2"
```

### Extract swatches with k-means

```rust
use swatchthis::{generate_swatches_kmeans, pixels_from_rgba};
use swatchthis::algorithms::kmeans::{KmeansColorSpace, InitMethod};

let rgba_data: &[u8] = &[/* RGBA bytes from an image */];
let pixels = pixels_from_rgba(rgba_data);

let swatches = generate_swatches_kmeans(
    &pixels,
    6,                          // number of swatches
    KmeansColorSpace::Lab,      // cluster in CIELAB space
    InitMethod::KMeansPlusPlus, // k-means++ init
    42,                         // seed for deterministic results
);

for swatch in &swatches {
    println!("{} — {} pixels", swatch.hex(), swatch.population);
}
```

### Octree quantisation

```rust
use swatchthis::{generate_swatches_octree, pixels_from_rgba};
use swatchthis::algorithms::octree::{OctreeColorSpace, OctreeDepth};

let pixels = pixels_from_rgba(rgba_data);
let swatches = generate_swatches_octree(
    &pixels,
    6,
    OctreeColorSpace::Rgb,
    OctreeDepth::D6, // tree depth, 1–8
);
```

### Median cut

```rust
use swatchthis::{generate_swatches_median_cut, pixels_from_rgba};

let pixels = pixels_from_rgba(rgba_data);
let swatches = generate_swatches_median_cut(&pixels, 6);
```

### Superpixel preprocessing

For photographic images, reducing pixels to superpixel averages first can produce
cleaner palettes. The output of either preprocessor can be passed straight to any
of the palette functions above.

```rust
use swatchthis::preprocessors::{slic, seeds};
use swatchthis::{generate_swatches_kmeans, pixels_from_rgba};
use swatchthis::algorithms::kmeans::{KmeansColorSpace, InitMethod};

let pixels = pixels_from_rgba(rgba_data);

// SLIC: num_superpixels, compactness (10–40 typical)
let regions = slic::slic_preprocess(&pixels, width, height, 200, 20.0);

// or SEEDS: num_superpixels, num_levels (3–5), histogram_bins (~5)
// let regions = seeds::seeds_preprocess(&pixels, width, height, 200, 4, 5);

let swatches = generate_swatches_kmeans(
    &regions, 6, KmeansColorSpace::Lab, InitMethod::KMeansPlusPlus, 42,
);
```

### Work with colours directly

```rust
use swatchthis::color::Rgb;

let red = Rgb::new(255, 0, 0);
let lab = red.to_lab();
let back = lab.to_rgb();
assert_eq!(red, back);

println!("{}", red.to_hex()); // #ff0000
```

## WebAssembly

### CDN (jsdelivr)

Use the pre-built WASM files directly from jsdelivr — pin to a specific version:

```javascript
import init, {
    generateSwatches,
    generateSwatchesOctree,
    generateSwatchesMedianCut,
    slicPreprocess,
    seedsPreprocess,
    complementaryColor,
} from 'https://cdn.jsdelivr.net/gh/OwenElliottDev/swatchthis@wasm-0.2.0/swatchthis.js';

await init();

const canvas = document.querySelector('canvas');
const ctx = canvas.getContext('2d');
const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);

// k-means: (rgba, count, colorSpace, initMethod, seed)
//   colorSpace: "rgb" | "lab" | "lab-ciede2000"
//   initMethod: "kmeans++" | "random"
const json = generateSwatches(imageData.data, 6, "lab", "kmeans++", 42n);
const swatches = JSON.parse(json);
// [{ hex: "#ff0000", r: 255, g: 0, b: 0, population: 1234 }, ...]

// octree: (rgba, count, colorSpace, maxDepth)
const octreeJson = generateSwatchesOctree(imageData.data, 6, "rgb", 6);

// median cut: (rgba, count)
const medianJson = generateSwatchesMedianCut(imageData.data, 6);

// SLIC preprocess: returns RGBA bytes of region-averaged pixels
const slicRgba = slicPreprocess(imageData.data, canvas.width, canvas.height, 200, 20.0);

// SEEDS preprocess
const seedsRgba = seedsPreprocess(imageData.data, canvas.width, canvas.height, 200, 4, 5);

// Complementary colour: returns [r, g, b]
const [r, g, b] = complementaryColor(255, 0, 0);
```

### Build from source

```sh
wasm-pack build --target web --features wasm
```

A demo app is included in `demos/web_image/`. Build it with:

```sh
bash demos/web_image/build.sh
cd demos/web_image && python3 -m http.server 8080
```

## Licence

MIT
