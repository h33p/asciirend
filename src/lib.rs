//! # asciirend
//!
//! ```text
//!     .  .  .  ....   . . .   .  ..   . ...
//!  .   ..    ...   .  . .  .   .. .    ..  .
//!  .. .#+===+========+=+=============+=. ..
//!    ..  %#######===============++==++.... .
//! .   ..  ######%#...........=+==++== ..... .
//!  .    ...#######.asciirend.==++=+.  ..
//! ..     .   %%###...........=++++..     .
//!   .      .. #%########+++=+=+== ..     .  .
//!   ..   . . ...########==+=+==...  . . . .
//! . .  ..     .. ..####%+=++=. .... .  ...
//!      . .... ..... .###++=.  .      .. .. .
//! ```
//!
//! ## Generic ascii renderer
//!
//! `asciirend` is a `no_std` compatible 3D rendering core. This crate renders objects in several
//! stages:
//!
//! - Primitive shading (similar to vertex shading, albeit works on whole primitives).
//! - Fragment shading.
//! - Additional text pass.
//!
//! Fragments are shaded in full sRGB float color space, which then is quantized to characters with a
//! dithering algorithm. This allows the image to have clearer color transitions, as opposed to direct
//! character shading. This also means the rendering backend is agnostic to the available color space
//! (so long as it's not HDR!), which is important for a generic `no_std` compatible renderer.
//!
//! Entrypoint to rendering is the [`Renderer`] struct.
//!
//! With `std` feature, you may also incorporate `crossterm` crate for out-of-the-box colorful 3D
//! rendering.
//!
//! ## Example
//!
//! Please see [`src/bin/example.rs`](src/bin/example.rs) for usage sample.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::{sync::Arc, vec::Vec};
use core::cmp::Ordering;

use nalgebra as na;

pub mod dithering;
use dithering::Dithering;
pub mod color;
use color::{PixelText, QuantizePixel};
pub mod extra;

pub type Transform = na::Transform3<f32>;
pub type Vector2 = na::Vector2<f32>;
pub type Vector3 = na::Vector3<f32>;
pub type Vector4 = na::Vector4<f32>;
pub type Matrix4 = na::Matrix4<f32>;
pub type Matrix<const R: usize, const C: usize> =
    na::Matrix<f32, na::Const<R>, na::Const<C>, na::ArrayStorage<f32, R, C>>;

fn clip_to_ndc(clip_space: Vector4) -> Vector3 {
    Vector3::new(
        clip_space.x / clip_space.w,
        clip_space.y / clip_space.w,
        clip_space.z / clip_space.w,
    )
}

fn ndc_to_screen(mut p: Vector3, w: usize, h: usize) -> Vector3 {
    let half_w = (w - 1) as f32 / 2.0;
    let half_h = (h - 1) as f32 / 2.0;
    p.x = (p.x + 1.0) * half_w;
    p.y = (1.0 - p.y) * half_h;
    p.z = (p.z + 1.0) / 2.0;
    p
}

fn edge_function(a: Vector2, b: Vector2, c: Vector2) -> f32 {
    (c.x - a.x) * (b.y - a.y) - (c.y - a.y) * (b.x - a.x)
}

#[allow(unused)]
fn float_sort(a: f32, b: f32) -> Ordering {
    if a > b {
        Ordering::Greater
    } else if a < b {
        Ordering::Less
    } else {
        Ordering::Equal
    }
}

/// Computes bounding box
///
/// # Panics
///
/// If empty list is passed
fn bounding_box<const R: usize, const C: usize>(
    v: &[Matrix<R, C>],
) -> (Matrix<R, C>, Matrix<R, C>) {
    let mut min = v[0];
    let mut max = v[0];

    for v in v {
        for (v, (min, max)) in v.iter().zip(min.iter_mut().zip(max.iter_mut())) {
            *min = v.min(*min);
            *max = v.max(*max);
        }
    }

    (min, max)
}

/// Returns the assumed aspect ratio of terminal characters.
///
/// Currently returns (1, 2), indicating that characters are twice as tall, compared to their
/// width.
pub fn term_char_aspect() -> (usize, usize) {
    (1, 2)
}

/// Describes background parameters.
///
/// These parameters need to be passed at the start of each frame, and are used to clear the
/// background with specified color.
pub struct Background {
    pub color: Vector3,
}

impl Default for Background {
    fn default() -> Self {
        Self {
            color: Vector3::new(0.05, 0.23, 0.4),
        }
    }
}

/// Immediate mode renderer.
///
/// This object allows one to render graphics into arbitrary vectors, holding any [`QuantizePixel`]
/// types. The typical flow of each frame is as follows:
///
/// 1. Clear the framebuffer with [`Renderer::clear_screen`].
/// 2. First pass object rendering with [`Renderer::render`].
/// 3. Second pass text rendering with [`Renderer::text_pass`].
///
/// The end result may look something like this:
///
/// ```text
///    ...   .   .   ..    ...   .  . .  .   .. .    ..  .    .
///  ..    .  . .. ... . . ...  ==. . .  .  .     . . ..   ..  .
///   .      ..   ..   ==+===================..  ...... . .   ..
///  .      .  .   ..##=++==========+=+====+=...   ..... . ..  .
///       ..    .    .####===.........+=====.. ..  ..        ...
/// .  .  .......     .###%%#.Example.=====......     .      . .
/// . .  . .     .      ####%.........=+== .   ..     .  .    .
///     .  . ..  ..   . .##########+=====.. ...  . . . .  .  . .
/// .  ..  . . . .  ..     ########==+==.=+ .... .  ...    . .
/// .    . .  .     . .... ..######===. .  +==    .. .. .     ..
///   ..  ..    ....    .  ..  ###=== .    .  +++.    .. ..   ..
///    .    ... .... .  .. .    .#= .          ..++=... ...  . .
/// .   .    .  .. ..  .. .. ....  .       ....    .=++..  ... .
///  . ...      ...  . ...      ...  . ...      ...  . ++.
/// ```
#[derive(Default, Debug)]
pub struct Renderer {
    vertex_state: VertexState,
    fragment_state: RasterState,
}

#[derive(Debug, Clone, Copy)]
struct PrimitiveId {
    mat_idx: usize,
    obj_idx: usize,
    pri_idx: usize,
}

#[derive(Default, Debug)]
struct VertexState {
    /// (Primitive, (mat_idx, obj_idx, pri_idx))
    ///
    /// pri_idx will be passed to material at mat_idx, in order to shade fragments using correct
    /// material.
    primitives: Vec<(Primitive, PrimitiveId)>,
    /// Centers of objects in screen space coordinates
    ///
    /// This is not necessary per se, but it is used in text rendering to have stable centering of
    /// text.
    obj_clip_center: Vec<Vector4>,
}

fn plane_intersect(inside: Vector4, outside: Vector4, dim: usize, clip: f32) -> Vector4 {
    let t = (clip - inside[dim]) / (outside[dim] - inside[dim]);
    let ret = inside + t * (outside - inside);
    ret
}

impl VertexState {
    pub fn reset(&mut self) {
        self.primitives.clear();
        self.obj_clip_center.clear();
    }

    pub fn clip_and_push_primitive(&mut self, primitive: Primitive, id: PrimitiveId) {
        match primitive {
            Primitive::Triangle(t) => self.clip_and_push_triangle(t, id),
            Primitive::Line(l) => self.clip_and_push_line(l, id),
        }
    }

    pub fn clip_and_push_line(&mut self, mut line: Line, id: PrimitiveId) {
        // TODO: clip the line on all axis, and both ends of the coord space.
        // Currently the lines may not render if none of the points is within the screen.
        for dim in 2..3 {
            let Line { start, end } = line;
            let clip = [start, end].map(|v| v[dim] / libm::fabsf(v.w) < 0.0);
            let clip_cnt = clip.iter().filter(|v| **v).count();

            line = if clip_cnt == 0 {
                // No vertices clipped - push as is
                Line { start, end }
            } else if clip_cnt == 1 {
                // 1 vertice clipped - find intersection point and push
                if clip[0] {
                    Line {
                        start: plane_intersect(end, start, dim, 0.0),
                        end,
                    }
                } else {
                    Line {
                        start,
                        end: plane_intersect(start, end, dim, 0.0),
                    }
                }
            } else {
                // All vertices clipped - don't push anything
                return;
            };
        }

        self.primitives.push((Primitive::Line(line), id));
    }

    /// Performs near plane clipping and pushes the triangle on stack.
    ///
    /// This may lead into an additional triangle being created, but it will share both mat_idx and
    /// pri_idx with the original one.
    pub fn clip_and_push_triangle(&mut self, Triangle { a, b, c }: Triangle, id: PrimitiveId) {
        let dim = 2;

        let clip = [a, b, c].map(|v| v[dim] / libm::fabsf(v.w) < 0.0);
        let clip_cnt = clip.iter().filter(|v| **v).count();

        if clip_cnt == 2 {
            // 2 verts clipped, we just bring all vertices to be within bounds
            let unclipped_idx = clip.iter().enumerate().find(|(_, v)| !**v).unwrap().0;
            let mut cnt = 0;
            let verts = [a, b, c];
            let [a, b, c] = verts.map(|v| {
                let i = cnt;
                cnt += 1;
                if i == unclipped_idx {
                    v
                } else {
                    plane_intersect(verts[unclipped_idx], v, dim, 0.0)
                }
            });
            self.primitives
                .push((Primitive::Triangle(Triangle { a, b, c }), id));
        } else if clip_cnt == 1 {
            // 1 vert clipped, we get 2 intersection points, and create 2 triangles out of them
            let clipped_idx = clip.iter().enumerate().find(|(_, v)| **v).unwrap().0;
            let verts = [a, b, c];

            let (i1, i2) = if clipped_idx == 0 {
                (1, 2)
            } else {
                (0, 1 + (clipped_idx - 1) % 2)
            };

            let c1 = plane_intersect(verts[i1], verts[clipped_idx], dim, 0.0);
            let c2 = plane_intersect(verts[i2], verts[clipped_idx], dim, 0.0);

            {
                let mut verts1 = verts;
                verts1[clipped_idx] = c1;
                let [a, b, c] = verts1;
                self.primitives
                    .push((Primitive::Triangle(Triangle { a, b, c }), id));
            }

            {
                let mut verts2 = verts;
                verts2[i1] = c1;
                verts2[clipped_idx] = c2;
                let [a, b, c] = verts2;
                self.primitives
                    .push((Primitive::Triangle(Triangle { a, b, c }), id));
            }
        } else if clip_cnt == 0 {
            self.primitives
                .push((Primitive::Triangle(Triangle { a, b, c }), id));
        }
    }
}

/// Defines a material and its shading.
///
/// Types that implement this are usually stateful, because instances of `Material` are the ones
/// responsible for storing per-primitive data, used for fragment shading.
pub trait Material {
    /// Indicates the start of new frame.
    ///
    /// On new frame, all primitives are discarded, therefore, the material should clear any stored
    /// data upon this call.
    fn new_frame(&mut self);

    /// Transforms and registers a primitive.
    ///
    /// This function takes a primitive (line/triangle), performs computation, and returns an ID,
    /// associated with it. The ID will then later be used to call [`Material::fragment_shade`]
    /// with.
    ///
    /// This structure allows materials to store arbitrary data for fragment shading purposes.
    fn primitive_shade(
        &mut self,
        primitive: Primitive,
        proj: Matrix4,
        model: Matrix4,
    ) -> (usize, Primitive);

    /// Shade a primitive at specified position.
    ///
    /// Material shall assume that provided position lies within the primitive.
    ///
    /// TODO for later: provide mechanisms for interpolating per-point data.
    fn fragment_shade(&self, primitive: usize, pos: Vector2, depth: f32) -> Option<Vector3>;
}

#[derive(Default, Debug)]
struct RasterOutput {
    obj_bb: Vec<Option<(usize, usize, usize, usize)>>,
}

#[derive(Default, Debug)]
struct RasterState {
    w: usize,
    h: usize,
    /// Used for depth testing
    depth: Vec<f32>,
    /// Used as a stencil for text drawing
    objs: Vec<usize>,
    /// Used for additional passes (like text rendering)
    output: RasterOutput,
}

impl RasterState {
    pub fn ndc_to_screen(&self, p: Vector3) -> Vector3 {
        ndc_to_screen(p, self.w, self.h)
    }

    fn clear_screen<T: QuantizePixel>(
        &mut self,
        bg: &Background,
        conv_params: &T::Params,
        dithering: &mut impl Dithering,
        buf: &mut Vec<T>,
        w: usize,
        h: usize,
    ) {
        self.w = w;
        self.h = h;

        let len = w * h;
        buf.clear();
        dithering.new_frame(w, h);

        for y in 0..h {
            for x in 0..w {
                buf.push(T::quantize_color(conv_params, bg.color, dithering, x, y));
            }
        }

        self.depth.clear();
        self.depth.resize(len, 1f32);
        self.objs.resize(len, !0usize);
    }

    fn rasterize<T: QuantizePixel>(
        &mut self,
        vs: &VertexState,
        mats: &mut [&mut dyn Material],
        max_obj: usize,
        conv_params: &T::Params,
        dithering: &mut impl Dithering,
        buf: &mut Vec<T>,
    ) {
        let len = self.w * self.h;
        assert_eq!(buf.len(), len);

        self.output.obj_bb.clear();
        self.output.obj_bb.resize(max_obj, None);

        for (
            p,
            PrimitiveId {
                mat_idx,
                obj_idx,
                pri_idx,
            },
        ) in vs.primitives.iter()
        {
            let mat = &mut mats[*mat_idx];

            let mut shade_pixel = |x, y, depth| {
                assert!(x < self.w);
                assert!(y < self.h);
                let bidx = y * self.w + x;

                // Update bounding box before depth checking, and before making sure the fragment
                // was emitted. This is so that holes can be represented within the bounds.
                if let Some(bb) = self.output.obj_bb.get_mut(*obj_idx) {
                    if let Some((min_x, min_y, max_x, max_y)) = bb.as_mut() {
                        *min_x = core::cmp::min(*min_x, x);
                        *min_y = core::cmp::min(*min_y, y);
                        *max_x = core::cmp::max(*max_x, x);
                        *max_y = core::cmp::max(*max_y, y);
                    } else {
                        *bb = Some((x, y, x, y));
                    }
                }

                if depth >= 0.0 && self.depth[bidx] >= depth {
                    if let Some(color) = mat.fragment_shade(
                        *pri_idx,
                        Vector2::new((x as f32) / self.w as f32, (y as f32) / self.h as f32),
                        depth,
                    ) {
                        self.depth[bidx] = depth;
                        self.objs[bidx] = *obj_idx;
                        buf[bidx] = T::quantize_color(conv_params, color, dithering, x, y);
                    }
                }
            };

            match p {
                Primitive::Triangle(t) => {
                    let t = [t.a, t.b, t.c]
                        .map(clip_to_ndc)
                        .map(|v| ndc_to_screen(v, self.w, self.h));

                    let (bbmin, bbmax) = bounding_box(&t);

                    let [a, b, c] = t;

                    let area = edge_function(a.xy(), b.xy(), c.xy());

                    if area <= 0.0 {
                        continue;
                    }

                    for y in
                        (bbmin.y.max(0.) as usize)..(libm::ceilf(bbmax.y.min(self.h as _)) as usize)
                    {
                        for x in (bbmin.x.max(0.) as usize)
                            ..(libm::ceilf(bbmax.x.min(self.w as _)) as usize)
                        {
                            let p = Vector2::new(x as f32, y as f32);

                            let wa = edge_function(b.xy(), c.xy(), p) / area;
                            let wb = edge_function(c.xy(), a.xy(), p) / area;
                            let wc = edge_function(a.xy(), b.xy(), p) / area;

                            if wa >= 0.0 && wb >= 0.0 && wc >= 0.0 {
                                let depth = wa * a.z + wb * b.z + wc * c.z;
                                shade_pixel(x, y, depth);
                            }
                        }
                    }
                }
                Primitive::Line(l) => {
                    let l = [l.start, l.end]
                        .map(clip_to_ndc)
                        .map(|v| ndc_to_screen(v, self.w, self.h));

                    let [a, b] = l;

                    fn plot_line_low(
                        w: usize,
                        h: usize,
                        mut x0: usize,
                        y0: usize,
                        x1: usize,
                        y1: usize,
                        mut plot: impl FnMut(usize, usize),
                    ) {
                        let dx = x1 as isize - x0 as isize;
                        let mut dy = y1 as isize - y0 as isize;
                        let mut yi = 1;

                        if dy < 0 {
                            yi = -1;
                            dy = -dy;
                        }

                        let mut d = (2 * dy) - dx;
                        let mut y = y0;

                        while x0 <= x1 && x0 < w && y < h {
                            plot(x0, y);
                            if d > 0 {
                                y = y.saturating_add_signed(yi);
                                d += 2 * (dy - dx);
                            } else {
                                d += 2 * dy;
                            }
                            x0 += 1;
                        }
                    }

                    fn plot_line_high(
                        w: usize,
                        h: usize,
                        x0: usize,
                        mut y0: usize,
                        x1: usize,
                        y1: usize,
                        mut plot: impl FnMut(usize, usize),
                    ) {
                        let mut dx = x1 as isize - x0 as isize;
                        let dy = y1 as isize - y0 as isize;
                        let mut xi = 1;

                        if dx < 0 {
                            xi = -1;
                            dx = -dx;
                        }

                        let mut d = (2 * dx) - dy;
                        let mut x = x0;

                        while y0 <= y1 && y0 < h && x < w {
                            plot(x, y0);
                            if d > 0 {
                                x = x.saturating_add_signed(xi);
                                d += 2 * (dx - dy);
                            } else {
                                d += 2 * dx;
                            }
                            y0 += 1;
                        }
                    }

                    fn plot_line(
                        w: usize,
                        h: usize,
                        x0: usize,
                        y0: usize,
                        x1: usize,
                        y1: usize,
                        plot: impl FnMut(usize, usize),
                    ) {
                        if y1.abs_diff(y0) < x1.abs_diff(x0) {
                            if x0 > x1 {
                                plot_line_low(w, h, x1, y1, x0, y0, plot);
                            } else {
                                plot_line_low(w, h, x0, y0, x1, y1, plot);
                            }
                        } else {
                            if y0 > y1 {
                                plot_line_high(w, h, x1, y1, x0, y0, plot);
                            } else {
                                plot_line_high(w, h, x0, y0, x1, y1, plot);
                            }
                        }
                    }

                    plot_line(
                        self.w,
                        self.h,
                        libm::roundf(a.x) as usize,
                        libm::roundf(a.y) as usize,
                        libm::roundf(b.x) as usize,
                        libm::roundf(b.y) as usize,
                        |x, y| {
                            // Compute how close we are to both segments (in NDC 2D);
                            let da = (a.xy() - Vector2::new(x as f32, y as f32)).magnitude();
                            let db = (b.xy() - Vector2::new(x as f32, y as f32)).magnitude();
                            let total = da + db;
                            let lerp = da / total;
                            let depth = a.z + (b.z - a.z) * lerp;
                            shade_pixel(x, y, depth);
                        },
                    );
                }
            }
        }
    }
}

/// Describes camera point of view.
///
/// The camera is ought to use right handed coordinate system, where `Z` is up, and `Y` is forward.
/// Feel free to use [`extra::create_transform`] function to construct the transformation. While
/// continuous control of the camera can be set up with
/// [`extra::camera_controller::CameraController`].
pub struct Camera {
    pub transform: Transform,
    pub proj: na::Projective3<f32>,
}

impl Camera {
    pub fn new(proj: na::Projective3<f32>) -> Self {
        Self {
            transform: Transform::default(),
            proj,
        }
    }
}

/// Properties for a renderable object.
pub struct Object {
    /// Tranformation matrix.
    ///
    /// Create one using [`extra::create_transform`] or do it yourself.
    pub transform: Transform,
    /// Material ID.
    ///
    /// This will correspond to the slice index of materials that are passed to the renderer.
    pub material: usize,
    /// Object type.
    pub ty: ObjType,
    /// Optional text to be drawn.
    ///
    /// If [`Renderer::text_pass`] is called, this text will be placed in the middle of the object
    /// (based on transform world origin), and the text will only be drawn within the pixels of the
    /// object, i.e. it will not overlap any other objects.
    pub text: Option<Arc<str>>,
}

/// Describes an object shape.
pub enum ObjType {
    Cube { size: Vector3 },
    Primitive(Primitive),
}

impl ObjType {
    fn gen(
        &self,
        proj: Matrix4,
        model: Matrix4,
        state: &mut VertexState,
        material: &mut dyn Material,
        obj_idx: usize,
        mat_idx: usize,
    ) {
        //let mat = proj * model;
        match self {
            Self::Cube { size, .. } => {
                const CUBE_VERTICES: [Vector4; 8] = [
                    Vector4::new(-0.5, -0.5, -0.5, 1.0),
                    Vector4::new(0.5, -0.5, -0.5, 1.0),
                    Vector4::new(0.5, 0.5, -0.5, 1.0),
                    Vector4::new(-0.5, 0.5, -0.5, 1.0),
                    Vector4::new(-0.5, -0.5, 0.5, 1.0),
                    Vector4::new(0.5, -0.5, 0.5, 1.0),
                    Vector4::new(0.5, 0.5, 0.5, 1.0),
                    Vector4::new(-0.5, 0.5, 0.5, 1.0),
                ];

                const CUBE_INDICES: [[usize; 3]; 12] = [
                    [0, 2, 1],
                    [0, 3, 2],
                    [1, 2, 6],
                    [6, 5, 1],
                    [4, 5, 6],
                    [6, 7, 4],
                    [2, 3, 6],
                    [6, 3, 7],
                    [0, 7, 3],
                    [0, 4, 7],
                    [0, 1, 5],
                    [0, 5, 4],
                ];

                for [a, b, c] in CUBE_INDICES.map(|v| {
                    v.map(|v| {
                        //println!("{:?} - {:?}", CUBE_VERTICES[v], CUBE_VERTICES[v].component_mul(&Vector4::new(size.x, size.y, size.z, 1.0)));
                        //println!("{:?} - {:?}", mat * CUBE_VERTICES[v], mat * CUBE_VERTICES[v].component_mul(&Vector4::new(size.x, size.y, size.z, 1.0)));
                        CUBE_VERTICES[v].component_mul(&Vector4::new(size.x, size.y, size.z, 1.0))
                    })
                }) {
                    let triangle = Triangle { a, b, c };
                    let (pri_idx, primitive) =
                        material.primitive_shade(Primitive::Triangle(triangle), proj, model);
                    state.clip_and_push_primitive(
                        primitive,
                        PrimitiveId {
                            mat_idx,
                            obj_idx,
                            pri_idx,
                        },
                    );
                }
            }
            Self::Primitive(primitive) => {
                let (pri_idx, primitive) = material.primitive_shade(*primitive, proj, model);
                state.clip_and_push_primitive(
                    primitive,
                    PrimitiveId {
                        mat_idx,
                        obj_idx,
                        pri_idx,
                    },
                );
            }
        }
    }
}

/// General rendering primitives.
#[derive(Debug, Clone, Copy)]
pub enum Primitive {
    Triangle(Triangle),
    Line(Line),
}

/// A triangle.
///
/// Described rather oddly, in homogeneous coordinates, but oh well. Deal with it. Or if you don't
/// know how, just keep `w` component set to `1.0` and it should all be good.
#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    pub a: Vector4,
    pub b: Vector4,
    pub c: Vector4,
}

/// A line.
///
/// Described rather oddly, in homogeneous coordinates, but oh well. Deal with it. Or if you don't
/// know how, just keep `w` component set to `1.0` and it should all be good.
#[derive(Debug, Clone, Copy, Default)]
pub struct Line {
    pub start: Vector4,
    pub end: Vector4,
}

impl Renderer {
    /// Clears the screen with specified background.
    ///
    /// This function must be called before every [`Renderer::render`] call, because it sets
    /// internal buffers up with specified dimensions.
    ///
    /// The `buf` should be then passed to subsequent renderer calls, without having its dimensions
    /// modified beforehand.
    pub fn clear_screen<T: QuantizePixel>(
        &mut self,
        bg: &Background,
        conv_params: &T::Params,
        dithering: &mut impl Dithering,
        buf: &mut Vec<T>,
        w: usize,
        h: usize,
    ) {
        self.fragment_state
            .clear_screen(bg, conv_params, dithering, buf, w, h)
    }

    /// Draws objects on screen.
    ///
    /// This function takes a list of objects, their materials, and draws them to given buffer.
    /// Note that `buf` must be first cleared using [`Renderer::clear_screen`] function.
    pub fn render<T: QuantizePixel>(
        &mut self,
        camera: &Camera,
        conv_params: &T::Params,
        mats: &mut [&mut dyn Material],
        objects: &[Object],
        dithering: &mut impl Dithering,
        buf: &mut Vec<T>,
    ) {
        let pos = camera.transform.transform_point(&Vector3::default().into());
        let dir = camera
            .transform
            .transform_vector(&na::vector![0.0, 1.0, 0.0]);
        let view = Matrix4::look_at_rh(&pos, &(pos + dir), &na::vector![0.0, 0.0, 1.0]);

        let proj = camera.proj.matrix() * view;

        // First, split into view space triangles and lines, Sort of equivalent of vertex shading
        self.vertex_state.reset();

        for mat in mats.iter_mut() {
            mat.new_frame();
        }

        for (i, obj) in objects.iter().enumerate() {
            self.vertex_state
                .obj_clip_center
                .push(proj * obj.transform.matrix() * na::vector![0.0, 0.0, 0.0, 1.0]);
            obj.ty.gen(
                proj,
                *obj.transform.matrix(),
                &mut self.vertex_state,
                mats[obj.material],
                i,
                obj.material,
            );
        }

        // Then, render into the buffer
        self.fragment_state.rasterize(
            &self.vertex_state,
            mats,
            objects.len(),
            conv_params,
            dithering,
            buf,
        );
    }

    /// Draws text on top of rendered objects.
    ///
    /// This function takes a list of objects (the identical set, to previously passed to
    /// [`Renderer::render`]), and draws auxiliary text, if it was set.
    pub fn text_pass<T: PixelText + QuantizePixel>(
        &mut self,
        objects: &[Object],
        buf: &mut Vec<T>,
    ) {
        for (i, obj) in objects.iter().enumerate() {
            let Some((min_x, min_y, max_x, max_y)) =
                self.fragment_state.output.obj_bb.get(i).and_then(|v| *v)
            else {
                continue;
            };

            if max_y - min_y < 3 {
                continue;
            }

            let Some(text) = obj.text.as_deref() else {
                continue;
            };

            // TODO: create optimal line wrapping that minimizes area
            let w = text.len();

            let max_chars = max_x - min_x - 2;

            let (chars, width) = if max_chars < w {
                (text.chars().take(max_chars), max_chars)
            } else {
                (text.chars().take(w), w)
            };

            // Center point

            let mut mid_x = (max_x + min_x) / 2;
            let mut mid_y = (max_y + min_y) / 2;

            let left_chars = width / 2;
            let right_chars = (width - left_chars).saturating_sub(1);

            // Adjust midpoint with object world space center. Doing so allows us to have more
            // consistent text position.
            if let Some(coords) = self.vertex_state.obj_clip_center.get(i) {
                let ndc = clip_to_ndc(*coords);
                let screen = self.fragment_state.ndc_to_screen(ndc);
                let mx = libm::roundf(screen.x) as usize;
                let my = libm::roundf(screen.y) as usize;
                if max_y != min_y {
                    mid_y = core::cmp::min(core::cmp::max(my, min_y + 1), max_y - 1);
                }

                if max_x != min_x {
                    mid_x = core::cmp::min(
                        core::cmp::max(mx, min_x + left_chars + 1),
                        max_x - right_chars - 1,
                    );
                }
            }

            let mut darken = |x: usize, y: usize| {
                let bidx = y * self.fragment_state.w + x;
                // Do not darken other object pixels
                if self.fragment_state.objs[bidx] == i {
                    buf[bidx].darken();
                }
            };

            // First, darken all pixels around the text
            for y in ((mid_y - 1)..=(mid_y + 1)).step_by(2) {
                for x in (mid_x - left_chars - 1)..=(mid_x + right_chars + 1) {
                    darken(x, y);
                }
            }
            darken(mid_x - left_chars - 1, mid_y);
            darken(mid_x + right_chars + 1, mid_y);

            // Then, embed the character values
            for (o, c) in chars.enumerate() {
                let x = mid_x - left_chars + o;
                let bidx = mid_y * self.fragment_state.w + x;
                if self.fragment_state.objs[bidx] == i {
                    buf[bidx].embed(c);
                }
            }
        }
    }
}
