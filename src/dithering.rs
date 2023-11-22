//! Defines dithering trait and implementations.

/// Simple xor shuffled dither.
///
/// A 256 element list is generated using xorshift PRNG, somewhat uniformly distributing dither
/// shifts in the range of `[-0.5; 0.5]`.
///
/// During the dither call, index of the list is taken using `((x + 1) * (y + 1) * (z + 1)) % 256` calculation.
/// This results in a non-repeating dither pattern, which may be undesired in some artistic cases.
///
/// Optionally, user may enable frame counter, which will randomize dithering from frame to frame.
#[derive(Default)]
pub struct XorShufDither {
    count_frames: bool,
    frame_cnt: usize,
}

impl XorShufDither {
    pub fn set_count_frames(&mut self, count_frames: bool) {
        self.count_frames = count_frames;
    }
}

impl XorShufDither {
    const MASK: ([f32; 256], [u8; 256]) = {
        let mut state = (0, 1, 1, 1);

        const fn xorshift((mut a, mut x, mut y, mut z): (u8, u8, u8, u8)) -> (u8, u8, u8, u8) {
            let t = x ^ (x << 4);
            x = y;
            y = z;
            z = a;
            a = z ^ t ^ (z >> 1) ^ (t << 1);
            (a, x, y, z)
        }

        // We need to
        state = xorshift(state);
        state = xorshift(state);
        state = xorshift(state);

        let mut arr = [0.0; 256];
        let mut arr2 = [0; 256];

        let mut i = 0;

        while i < 256 {
            state = xorshift(state);
            arr2[i] = state.0;
            arr[i] = (state.0 as f32 / 256.0) - 0.5;
            i += 1;
        }

        (arr, arr2)
    };
}

impl Dithering for XorShufDither {
    fn new_frame(&mut self, _: usize, _h: usize) {
        self.frame_cnt += 1;
    }

    fn dither(&self, interp: f32, x: usize, y: usize, z: usize) -> f32 {
        let idx =
            ((x + 1) * (y + 1) * (z + 1) * if self.count_frames { self.frame_cnt } else { 1 })
                % 256;
        interp + Self::MASK.0[idx]
    }
}

/// Color dithering.
///
/// In limited color outputs (such as ascii rendered screens), direct nearest color conversion may
/// lead to excessive color banding. Diterhing aims to hide the limits of the output space by
/// injecting noise that would nudge value rounding to one way or the other.
///
/// Practically, dithering aims to make rounding probabilistic, rather than uniform among pixels.
/// Doing so leads to smaller overall error and perceptually better looking image.
pub trait Dithering {
    /// Prepares start of a new frame for the ditherer.
    ///
    /// This may be used to modify state in preparation.
    fn new_frame(&mut self, w: usize, h: usize);
    /// Takes in a 0-1 value and applies dithering to it.
    ///
    /// `interp` value indicates the weight between 2 integer values. The `Dithering` object may
    /// apply an offset in the `-0.5..=0.5` range to nudge the integer cast one way or the other.
    fn dither(&self, interp: f32, x: usize, y: usize, z: usize) -> f32;
}

impl Dithering for () {
    fn new_frame(&mut self, _: usize, _h: usize) {}

    fn dither(&self, interp: f32, _: usize, _: usize, _: usize) -> f32 {
        interp
    }
}
