use crate::color::Rgb;

/// A colour swatch extracted from an image.
///
/// Contains the dominant colour and the number of pixels assigned to it
/// during clustering.
///
/// # Example
///
/// ```
/// use swatchthis::color::Rgb;
/// use swatchthis::swatch::Swatch;
///
/// let swatch = Swatch::new(Rgb::new(255, 0, 0), 120);
/// assert_eq!(swatch.hex(), "#ff0000");
/// assert_eq!(swatch.population, 120);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Swatch {
    pub color: Rgb,
    pub population: u32,
}

impl Swatch {
    pub fn new(color: Rgb, population: u32) -> Self {
        Self { color, population }
    }

    /// Returns the colour as a lowercase hex string (e.g. `"#ff0000"`).
    pub fn hex(&self) -> String {
        self.color.to_hex()
    }
}
