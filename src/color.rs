/// An 8-bit RGB colour.
///
/// # Example
///
/// ```
/// use swatchthis::color::Rgb;
///
/// let red = Rgb::new(255, 0, 0);
/// assert_eq!(red.to_hex(), "#ff0000");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// A colour in the CIELAB colour space.
///
/// L is lightness (0–100), a and b are the colour-opponent dimensions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lab {
    pub l: f32,
    pub a: f32,
    pub b: f32,
}

impl Rgb {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Returns the colour as a lowercase hex string (e.g. `"#ff8000"`).
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Converts to CIELAB via the XYZ colour space (D65 illuminant).
    ///
    /// # Example
    ///
    /// ```
    /// use swatchthis::color::Rgb;
    ///
    /// let lab = Rgb::new(255, 255, 255).to_lab();
    /// assert!((lab.l - 100.0).abs() < 0.1);
    /// ```
    pub fn to_lab(self) -> Lab {
        let r = linearize(self.r);
        let g = linearize(self.g);
        let b = linearize(self.b);

        let x = r * 0.4124564 + g * 0.3575761 + b * 0.1804375;
        let y = r * 0.2126729 + g * 0.7151522 + b * 0.0721750;
        let z = r * 0.0193339 + g * 0.1191920 + b * 0.9503041;

        let xn = 0.95047;
        let yn = 1.0;
        let zn = 1.08883;

        let fx = lab_f(x / xn);
        let fy = lab_f(y / yn);
        let fz = lab_f(z / zn);

        Lab {
            l: 116.0 * fy - 16.0,
            a: 500.0 * (fx - fy),
            b: 200.0 * (fy - fz),
        }
    }

    /// Returns the squared Euclidean distance to another RGB colour.
    pub fn distance_squared(self, other: Rgb) -> u32 {
        let dr = self.r as i32 - other.r as i32;
        let dg = self.g as i32 - other.g as i32;
        let db = self.b as i32 - other.b as i32;
        (dr * dr + dg * dg + db * db) as u32
    }
}

impl Lab {
    pub fn new(l: f32, a: f32, b: f32) -> Self {
        Self { l, a, b }
    }

    /// Converts to sRGB via the XYZ colour space (D65 illuminant).
    ///
    /// # Example
    ///
    /// ```
    /// use swatchthis::color::{Rgb, Lab};
    ///
    /// let rgb = Rgb::new(128, 64, 32);
    /// let roundtrip = rgb.to_lab().to_rgb();
    /// assert_eq!(rgb, roundtrip);
    /// ```
    pub fn to_rgb(self) -> Rgb {
        let xn = 0.95047_f32;
        let yn = 1.0_f32;
        let zn = 1.08883_f32;

        let fy = (self.l + 16.0) / 116.0;
        let fx = self.a / 500.0 + fy;
        let fz = fy - self.b / 200.0;

        let x = xn * lab_f_inv(fx);
        let y = yn * lab_f_inv(fy);
        let z = zn * lab_f_inv(fz);

        let r = x * 3.2404542 + y * -1.5371385 + z * -0.4985314;
        let g = x * -0.9692660 + y * 1.8760108 + z * 0.0415560;
        let b = x * 0.0556434 + y * -0.2040259 + z * 1.0572252;

        Rgb {
            r: delinearize(r),
            g: delinearize(g),
            b: delinearize(b),
        }
    }

    /// Returns the squared Euclidean distance to another Lab colour.
    pub fn distance_squared(self, other: Lab) -> f32 {
        let dl = self.l - other.l;
        let da = self.a - other.a;
        let db = self.b - other.b;
        dl * dl + da * da + db * db
    }

    /// CIEDE2000 colour difference. Returns a non-negative value where 0
    /// means identical colours. The final sqrt is omitted since this is used
    /// for comparison only (sqrt is monotonic).
    pub fn distance_ciede2000(self, other: Lab) -> f32 {
        use std::f32::consts::{PI, TAU};
        const POW25_7: f32 = 6_103_515_625.0; // 25^7
        const DEG2RAD: f32 = PI / 180.0;
        const RAD30: f32 = 30.0 * DEG2RAD;
        const RAD6: f32 = 6.0 * DEG2RAD;
        const RAD63: f32 = 63.0 * DEG2RAD;
        const RAD275: f32 = 275.0 * DEG2RAD;
        const RAD25: f32 = 25.0 * DEG2RAD;

        let c1 = (self.a * self.a + self.b * self.b).sqrt();
        let c2 = (other.a * other.a + other.b * other.b).sqrt();
        let c_avg = (c1 + c2) * 0.5;
        let c_avg7 = c_avg * c_avg * c_avg * c_avg * c_avg * c_avg * c_avg;
        let g = 0.5 * (1.0 - (c_avg7 / (c_avg7 + POW25_7)).sqrt());

        let a1p = self.a * (1.0 + g);
        let a2p = other.a * (1.0 + g);

        let c1p = (a1p * a1p + self.b * self.b).sqrt();
        let c2p = (a2p * a2p + other.b * other.b).sqrt();

        let h1p = self.b.atan2(a1p).rem_euclid(TAU);
        let h2p = other.b.atan2(a2p).rem_euclid(TAU);

        let d_lp = other.l - self.l;
        let d_cp = c2p - c1p;

        let c1c2 = c1p * c2p;
        let dhp = if c1c2 == 0.0 {
            0.0
        } else if (h2p - h1p).abs() <= PI {
            h2p - h1p
        } else if h2p - h1p > PI {
            h2p - h1p - TAU
        } else {
            h2p - h1p + TAU
        };

        let d_hp = 2.0 * c1c2.sqrt() * (dhp * 0.5).sin();

        let lp_avg = (self.l + other.l) * 0.5;
        let cp_avg = (c1p + c2p) * 0.5;

        let hp_avg = if c1c2 == 0.0 {
            h1p + h2p
        } else if (h1p - h2p).abs() <= PI {
            (h1p + h2p) * 0.5
        } else if h1p + h2p < TAU {
            (h1p + h2p + TAU) * 0.5
        } else {
            (h1p + h2p - TAU) * 0.5
        };

        let t = 1.0 - 0.17 * (hp_avg - RAD30).cos()
            + 0.24 * (2.0 * hp_avg).cos()
            + 0.32 * (3.0 * hp_avg + RAD6).cos()
            - 0.20 * (4.0 * hp_avg - RAD63).cos();

        let lp50 = lp_avg - 50.0;
        let lp50_sq = lp50 * lp50;
        let sl = 1.0 + 0.015 * lp50_sq / (20.0 + lp50_sq).sqrt();
        let sc = 1.0 + 0.045 * cp_avg;
        let sh = 1.0 + 0.015 * cp_avg * t;

        let cp_avg7 = cp_avg * cp_avg * cp_avg * cp_avg * cp_avg * cp_avg * cp_avg;
        let rc = 2.0 * (cp_avg7 / (cp_avg7 + POW25_7)).sqrt();
        let hp_term = (hp_avg - RAD275) / RAD25;
        let rt = -(2.0 * 30.0 * DEG2RAD * (-(hp_term * hp_term)).exp()).sin() * rc;

        let dl_sl = d_lp / sl;
        let dc_sc = d_cp / sc;
        let dh_sh = d_hp / sh;
        dl_sl * dl_sl + dc_sc * dc_sc + dh_sh * dh_sh + rt * dc_sc * dh_sh
    }
}

fn linearize(c: u8) -> f32 {
    let c = c as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn delinearize(c: f32) -> u8 {
    let c = if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (c.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn lab_f(t: f32) -> f32 {
    let delta: f32 = 6.0 / 29.0;
    if t > delta * delta * delta {
        t.cbrt()
    } else {
        t / (3.0 * delta * delta) + 4.0 / 29.0
    }
}

fn lab_f_inv(t: f32) -> f32 {
    let delta: f32 = 6.0 / 29.0;
    if t > delta {
        t * t * t
    } else {
        3.0 * delta * delta * (t - 4.0 / 29.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_to_hex() {
        assert_eq!(Rgb::new(255, 128, 0).to_hex(), "#ff8000");
        assert_eq!(Rgb::new(0, 0, 0).to_hex(), "#000000");
    }

    #[test]
    fn rgb_to_lab_roundtrip() {
        let colors = [
            Rgb::new(255, 0, 0),
            Rgb::new(0, 255, 0),
            Rgb::new(0, 0, 255),
            Rgb::new(128, 128, 128),
            Rgb::new(0, 0, 0),
            Rgb::new(255, 255, 255),
        ];
        for rgb in colors {
            let lab = rgb.to_lab();
            let back = lab.to_rgb();
            assert_eq!(rgb, back, "roundtrip failed for {rgb:?} (lab={lab:?})");
        }
    }

    #[test]
    fn black_lab_values() {
        let lab = Rgb::new(0, 0, 0).to_lab();
        assert!((lab.l).abs() < 0.01);
    }

    #[test]
    fn white_lab_values() {
        let lab = Rgb::new(255, 255, 255).to_lab();
        assert!((lab.l - 100.0).abs() < 0.1);
    }
}
