//! Color related types and functions.

use crate::{Dithering, Vector3};
use colorsys::{Hsl, Rgb};
use nalgebra as na;

// Split between dark and light variants, because their hue is the same. This way we can compute
// neighbors purely based on hue, and then switch between dark and light variants (except black,
// which also has white, and will include grays too).
const COL_16: [Vector3; 16] = [
    // Different grayscale variants

    // Black
    Vector3::new(0.0, 0.0, 0.0),
    // DarkGray
    Vector3::new(0.5, 0.5, 0.5),
    // Gray
    Vector3::new(0.75, 0.75, 0.75),
    // White
    Vector3::new(1.0, 1.0, 1.0),
    // Base hues

    // DarkRed
    Vector3::new(0.5, 0.0, 0.0),
    // DarkYellow
    Vector3::new(0.5, 0.5, 0.0),
    // DarkGreen
    Vector3::new(0.0, 0.5, 0.0),
    // DarkCyan
    Vector3::new(0.0, 0.5, 0.5),
    // DarkBlue
    Vector3::new(0.0, 0.0, 0.5),
    // DarkMagenta
    Vector3::new(0.5, 0.0, 0.5),
    // Light variants

    // Red
    Vector3::new(1.0, 0.0, 0.0),
    // Yellow
    Vector3::new(1.0, 1.0, 0.0),
    // Green
    Vector3::new(0.0, 1.0, 0.0),
    // Cyan
    Vector3::new(0.0, 1.0, 1.0),
    // Blue
    Vector3::new(0.0, 0.0, 1.0),
    // Magenta
    Vector3::new(1.0, 0.0, 1.0),
];

/// Optimized artistic 16 color representation.
///
/// Internal color conversion attempts to quantize HSV color space into the 16 RGB colors, instead
/// of simply matching the nearest RGB value. This results in much smoother color transitions,
/// especially when dithering is employed.
///
/// However, more arbitrary decisions were taken blending between grayscale and color values. This
/// may lead to rather unexpected results, at times.
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum Col16 {
    // Grayscale
    Black = 0,
    DarkGray,
    Gray,
    White,
    // Dark hues - 60 degree steps
    DarkRed,
    DarkYellow,
    DarkGreen,
    DarkCyan,
    DarkBlue,
    DarkMagenta,
    // Light hues - 60 degree steps
    Red,
    Yellow,
    Green,
    Cyan,
    Blue,
    Magenta,
}

impl Col16 {
    pub fn as_vec(&self) -> Vector3 {
        COL_16[*self as u8 as usize]
    }

    fn from_idx(idx: usize) -> Self {
        assert!(idx < 16);
        unsafe { core::mem::transmute(idx as u8) }
    }
}

/// General pixel quantization trait.
///
/// Color quantization is the process of converting a color from high dynamic range into output
/// dynamic range. The process is inherently lossless, which may lead to color banding, however,
/// with dithering we may make the limited color space perceptually not significant.
///
/// Objects implementing `QuantizePixel` may be used as target elements of the renderer. Unlike
/// traditional renderers, ascii renderers may choose to also use different characters in the
/// quantization process. Therefore, we are more flexible to accomodate for that.
pub trait QuantizePixel: Clone {
    type Params;

    /// Quantizes a pixel.
    ///
    /// Takes a input RGB float vector, and quantizes it with provided dithering algorithm.
    fn quantize_color(
        params: &Self::Params,
        inp: Vector3,
        dithering: &impl Dithering,
        x: usize,
        y: usize,
    ) -> Self;
}

impl QuantizePixel for u8 {
    type Params = ();

    fn quantize_color(
        _: &Self::Params,
        inp: Vector3,
        dithering: &impl Dithering,
        x: usize,
        y: usize,
    ) -> u8 {
        // TODO: do HSV or even LAB based grayscale conversion
        //let v = inp.dot(&na::vector![1.0, 1.0, 1.0]).min(1.0);
        let v = inp.dot(&na::vector![0.21, 0.72, 0.07]);
        to_palette(v, dithering, x, y)
    }
}

pub trait PixelDarken {
    /// Used around text to give more contrast around the text.
    fn darken(&mut self);
}

impl PixelDarken for u8 {
    fn darken(&mut self) {
        if *self != PALETTE[0] {
            *self = PALETTE[1]
        }
    }
}

pub trait PixelText: PixelDarken {
    /// Embed a character point at the given position.
    fn embed(&mut self, c: char);
}

impl PixelText for u8 {
    fn embed(&mut self, c: char) {
        if c.is_ascii() {
            *self = c as u8;
        } else {
            *self = b'?';
        }
    }
}

const PALETTE: [u8; 10] = *b" .:-=+*%#@";

fn dithered_range(
    val: f32,
    max_val: usize,
    dithering: &impl Dithering,
    x: usize,
    y: usize,
) -> usize {
    let val = val.min(1.0).max(0.0) * max_val as f32;
    let floored = libm::floorf(val);
    let interp = val - floored;
    let dithered = dithering.dither(interp, x, y, 0);
    let val = dithered + floored;
    let idx = libm::roundf(val) as usize;
    if idx >= max_val {
        max_val
    } else {
        idx
    }
}

fn to_palette(val: f32, dithering: &impl Dithering, x: usize, y: usize) -> u8 {
    PALETTE[dithered_range(val, PALETTE.len() - 1, dithering, x, y)]
}

/// Converts to hsv (not hsl!)
fn to_hsv(rgb: Rgb) -> Hsl {
    let mut hsl = Hsl::from(&rgb);

    let mx = rgb.red().max(rgb.green()).max(rgb.blue()) / 2.55;
    let mn = rgb.red().min(rgb.green()).min(rgb.blue()) / 2.55;

    hsl.set_lightness(mx.min(100.0));
    hsl.set_saturation(
        if mx == 0.0 {
            0.0
        } else {
            (mx - mn) / mx * 100.0
        }
        .min(100.0),
    );

    hsl
}

impl QuantizePixel for Col16 {
    type Params = ();

    fn quantize_color(
        _: &Self::Params,
        inp: Vector3,
        dithering: &impl Dithering,
        x: usize,
        y: usize,
    ) -> Col16 {
        fn nearest_colors(
            inp: Hsl,
            dither_value: f32,
            dither_value2: f32,
        ) -> ((Col16, f32), (Col16, f32)) {
            // Based on saturation, check how we are going to blend colors
            // Because we can only blend between 2 colors, we apply dithering to transition between
            // grayscale and color blending.
            let (a, b) = if inp.saturation() / 100.0 < dither_value as f64 * 0.33 {
                // Only grayscale here, based on value. Simple.
                let val = inp.lightness() as f32 / 400.0;
                let floor = libm::floorf(val);
                let ceil = libm::ceilf(val);
                let idx1 = floor as usize;
                let idx2 = ceil as usize;
                (
                    (
                        Col16::from_idx(core::cmp::min(idx1, 3)),
                        libm::fabsf(val - floor),
                    ),
                    (
                        Col16::from_idx(core::cmp::min(idx2, 3)),
                        libm::fabsf(val - ceil),
                    ),
                )
            } else {
                let q_hue = inp.hue() / 60.0;
                // Ratio in -1.0 to 1.0 range
                let hue_rat = (q_hue - libm::floor(q_hue)) * 2.0 - 1.0;

                // The color square has value range 50-100
                // We use that and hue to determine the quadrant we are in
                let value_rat = inp.lightness() / 100.0 * 4.0 - 3.0;
                let phase = libm::atan2f(hue_rat as f32, value_rat as f32);

                use core::f32::consts::PI;

                const LIGHT_TRANSFORM: usize = Col16::Red as usize - Col16::DarkRed as usize;

                // We need to dither against this too, because we are blending between 2 colors out
                // of 4 possible ones.
                let phase = (phase + PI - PI * (dither_value2 - 0.5)) % (2.0 * PI) - PI;

                if phase <= 3.0 * PI / 4.0 && phase >= PI / 4.0 {
                    // Ceiled hue, diff lightness
                    let idx = Col16::DarkRed as usize + (libm::ceil(q_hue) as usize % 6);
                    (
                        (
                            Col16::from_idx(idx),
                            (value_rat.max(-1.0) + 1.0) as f32 / 2.0,
                        ),
                        (
                            Col16::from_idx(idx + LIGHT_TRANSFORM),
                            ((-value_rat).max(-1.0) + 1.0) as f32 / 2.0,
                        ),
                    )
                } else if phase <= PI / 4.0 && phase >= -PI / 4.0 {
                    // Ceiled lightness, diff hues
                    let idx1 = Col16::DarkRed as usize + libm::floor(q_hue) as usize;
                    let idx2 = Col16::DarkRed as usize + (libm::ceil(q_hue) as usize % 6);
                    let (a, b) = (
                        Col16::from_idx(idx1 + LIGHT_TRANSFORM),
                        Col16::from_idx(idx2 + LIGHT_TRANSFORM),
                    );
                    (
                        (a, (hue_rat + 1.0) as f32 * 30.0),
                        (b, (-hue_rat + 1.0) as f32 * 30.0),
                    )
                } else if phase <= -PI / 4.0 && phase >= -3.0 * PI / 4.0 {
                    // Floored hue, diff lightness
                    let idx = Col16::DarkRed as usize + libm::floor(q_hue) as usize;
                    (
                        (
                            Col16::from_idx(idx),
                            (value_rat.max(-1.0) + 1.0) as f32 / 2.0,
                        ),
                        (
                            Col16::from_idx(idx + LIGHT_TRANSFORM),
                            ((-value_rat).max(-1.0) + 1.0) as f32 / 2.0,
                        ),
                    )
                } else {
                    // Floored lightness, diff hues
                    let idx1 = Col16::DarkRed as usize + libm::floor(q_hue) as usize;
                    let idx2 = Col16::DarkRed as usize + (libm::ceil(q_hue) as usize % 6);
                    let (a, b) = (Col16::from_idx(idx1), Col16::from_idx(idx2));
                    (
                        (a, (hue_rat + 1.0) as f32 * 30.0),
                        (b, (-hue_rat + 1.0) as f32 * 30.0),
                    )
                }
            };

            (a, b)
        }

        let target = dithering.dither(0.5, x, y, 0);

        let inp = inp * 256.0;
        let rgb = Rgb::new(inp.x as f64, inp.y as f64, inp.z as f64, None);

        let ((a, a_dist), (b, b_dist)) = nearest_colors(
            to_hsv(rgb),
            dithering.dither(0.5, x, y, 1),
            dithering.dither(0.5, x, y, 2),
        );
        let dist_total = a_dist + b_dist;
        let lerp = a_dist / dist_total;

        if lerp <= target {
            a
        } else {
            b
        }
    }
}

impl<A: QuantizePixel, B: QuantizePixel> QuantizePixel for (A, B) {
    type Params = (A::Params, B::Params);

    fn quantize_color(
        (p_a, p_b): &(A::Params, B::Params),
        inp: Vector3,
        dithering: &impl Dithering,
        x: usize,
        y: usize,
    ) -> (A, B) {
        (
            A::quantize_color(p_a, inp, dithering, x, y),
            B::quantize_color(p_b, inp, dithering, x, y),
        )
    }
}

impl<A: PixelDarken, B: PixelDarken> PixelDarken for (A, B) {
    fn darken(&mut self) {
        self.0.darken();
        self.1.darken();
    }
}

impl<A: PixelDarken, B: PixelText> PixelText for (A, B) {
    fn embed(&mut self, c: char) {
        self.1.embed(c)
    }
}

#[cfg(feature = "crossterm")]
pub struct CrosstermConvParams {
    pub colors: CrosstermColorMode,
}

#[cfg(feature = "crossterm")]
pub enum CrosstermColorMode {
    SingleCol,
    Col16,
    Col256,
    Rgb,
}

#[cfg(feature = "crossterm")]
const _: () = {
    use crossterm::style::{Color, Colors};

    impl From<Col16> for Color {
        fn from(v: Col16) -> Self {
            use Color::*;

            match v {
                Col16::Black => Black,
                Col16::DarkGray => DarkGrey,
                Col16::Red => Red,
                Col16::DarkRed => DarkRed,
                Col16::Green => Green,
                Col16::DarkGreen => DarkGreen,
                Col16::Yellow => Yellow,
                Col16::DarkYellow => DarkYellow,
                Col16::Blue => Blue,
                Col16::DarkBlue => DarkBlue,
                Col16::Magenta => Magenta,
                Col16::DarkMagenta => DarkMagenta,
                Col16::Cyan => Cyan,
                Col16::DarkCyan => DarkCyan,
                Col16::White => White,
                Col16::Gray => Grey,
            }
        }
    }

    impl QuantizePixel for Colors {
        type Params = CrosstermConvParams;

        fn quantize_color(
            params: &Self::Params,
            inp: Vector3,
            dithering: &impl Dithering,
            x: usize,
            y: usize,
        ) -> Self {
            use CrosstermColorMode::*;

            let col = match params.colors {
                SingleCol => {
                    return Self {
                        foreground: None,
                        background: None,
                    }
                }
                Col16 => Color::from(crate::color::Col16::quantize_color(
                    &(),
                    inp,
                    dithering,
                    x,
                    y,
                )),
                Col256 => {
                    // We are doing something very ugly and inaccurate here, but hey, it's fast!
                    let r =
                        core::cmp::min(dithered_range(inp.x, 8, dithering, x, y) * 32, 255) as u8;
                    let g =
                        core::cmp::min(dithered_range(inp.y, 8, dithering, x, y) * 32, 255) as u8;
                    let b =
                        core::cmp::min(dithered_range(inp.z, 4, dithering, x, y) * 64, 255) as u8;
                    let rgb = colorsys::Rgb::from([r, g, b]);
                    let ansi = colorsys::Ansi256::from(rgb);
                    Color::AnsiValue(ansi.code())
                }
                Rgb => Color::Rgb {
                    r: dithered_range(inp.x, 255, dithering, x, y) as u8,
                    g: dithered_range(inp.y, 255, dithering, x, y) as u8,
                    b: dithered_range(inp.z, 255, dithering, x, y) as u8,
                },
            };

            Self {
                foreground: Some(col),
                background: None,
            }
        }
    }

    impl PixelDarken for Color {
        fn darken(&mut self) {
            match self {
                Self::White => *self = Self::Grey,
                Self::Grey => *self = Self::DarkGrey,
                Self::DarkGrey => *self = Self::Black,
                Self::Red => *self = Self::DarkRed,
                Self::Green => *self = Self::DarkGreen,
                Self::Yellow => *self = Self::DarkYellow,
                Self::Blue => *self = Self::DarkBlue,
                Self::Magenta => *self = Self::DarkMagenta,
                Self::Cyan => *self = Self::DarkCyan,
                Self::DarkRed => *self = Self::Black,
                Self::DarkGreen => *self = Self::Black,
                Self::DarkYellow => *self = Self::Black,
                Self::DarkBlue => *self = Self::Black,
                Self::DarkMagenta => *self = Self::Black,
                Self::DarkCyan => *self = Self::Black,
                Self::AnsiValue(val) => {
                    let ansi = colorsys::Ansi256::new(*val);
                    let mut rgb = colorsys::Rgb::from(ansi);
                    rgb.set_red(rgb.red() / 2.0);
                    rgb.set_green(rgb.green() / 2.0);
                    rgb.set_blue(rgb.blue() / 2.0);
                    let ansi = colorsys::Ansi256::from(rgb);
                    *self = Color::AnsiValue(ansi.code());
                }
                Self::Rgb { r, g, b } => {
                    *r /= 2;
                    *g /= 2;
                    *b /= 2;
                }
                _ => (),
            }
        }
    }

    impl PixelDarken for Colors {
        fn darken(&mut self) {
            self.foreground.as_mut().map(|v| v.darken());
            self.background.as_mut().map(|v| v.darken());
        }
    }
};
