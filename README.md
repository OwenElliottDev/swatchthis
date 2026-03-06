# swatchthis

A Rust library for extracting dominant colour swatches from images using k-means clustering. Works in native Rust and WebAssembly.

## Features

- **K-means and k-means++** centroid initialisation
- **RGB and CIELAB** colour space clustering
- **CIEDE2000 Distance** for CIELAB, slower but good perceptual accuracy
- **WebAssembly support** via `wasm-bindgen` (behind the `wasm` feature flag)
- **No runtime dependencies** for native builds (deterministic PRNG, no `rand` crate)

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
swatchthis = "0.1"
```

### Extract swatches from raw RGBA pixel data

```rust
use swatchthis::{generate_swatches, pixels_from_rgba};
use swatchthis::kmeans::{ColorSpace, InitMethod};

let rgba_data: &[u8] = &[/* RGBA bytes from an image */];
let pixels = pixels_from_rgba(rgba_data);

let swatches = generate_swatches(
    &pixels,
    6,                          // number of swatches
    ColorSpace::Lab,            // cluster in CIELAB space
    InitMethod::KMeansPlusPlus, // k-means++ init
    42,                         // seed for deterministic results
);

for swatch in &swatches {
    println!("{} — {} pixels", swatch.hex(), swatch.population);
}
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

### Lower-level clustering

```rust
use swatchthis::color::Rgb;
use swatchthis::kmeans::{extract_colors, ColorSpace, InitMethod};

let pixels: Vec<Rgb> = vec![/* ... */];
let results = extract_colors(&pixels, 4, ColorSpace::Rgb, InitMethod::Random, 1);

for (color, population) in &results {
    println!("{} ({}px)", color.to_hex(), population);
}
```

## WebAssembly

### Pre-built from GitHub Releases

Download the WASM files from the [latest release](https://github.com/OwenElliottDev/swatchthis/releases) and serve them alongside your app:

```javascript
import init, { generateSwatches } from './swatchthis.js';

await init();

const canvas = document.querySelector('canvas');
const ctx = canvas.getContext('2d');
const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);

const json = generateSwatches(imageData.data, 6, "lab", "kmeans++", 42n);
const swatches = JSON.parse(json);
// [{ hex: "#ff0000", r: 255, g: 0, b: 0, population: 1234 }, ...]
```

The release includes `swatchthis.js`, `swatchthis.d.ts`, and `swatchthis_bg.wasm`. All three files must be served from the same path.

### Build from source

```sh
wasm-pack build --target web --features wasm
```

A demo app is included in `demos/web/`. Build it with:

```sh
bash demos/web/build.sh
cd demos/web && python3 -m http.server 8080
```

## Licence

MIT
