use crate::{
    color::{ColorConvParams, PixelDarken, PixelText, QuantizePixel, TermColor, TermColorMode},
    dithering::Dithering,
    extra::{
        create_transform,
        global_state::{self as gs, Scene, StandardMaterial},
        ortho_proj,
    },
    *,
};
#[cfg(feature = "pyo3")]
use pyo3::prelude::*;
#[cfg(all(not(target_os = "wasi"), feature = "wasm-bindgen"))]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

#[cfg_attr(all(not(target_os = "wasi"), feature = "wasm-bindgen"), wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyfunction)]
#[no_mangle]
pub extern "C" fn new_scene() -> usize {
    gs::new_scene(Default::default)
}

#[cfg_attr(all(not(target_os = "wasi"), feature = "wasm-bindgen"), wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyfunction)]
#[no_mangle]
pub extern "C" fn remove_scene(scene: usize) {
    gs::remove_scene(scene)
}

#[cfg_attr(all(not(target_os = "wasi"), feature = "wasm-bindgen"), wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyfunction)]
#[no_mangle]
pub fn scene_to_json(scene: usize) -> String {
    gs::with_scene(scene, |scene| serde_json::to_string(scene).unwrap()).unwrap()
}

#[repr(C)]
pub struct StringTuple {
    ptr: *mut u8,
    len: usize,
}

impl Default for StringTuple {
    fn default() -> Self {
        Self {
            ptr: core::ptr::null_mut(),
            len: 0,
        }
    }
}

#[no_mangle]
pub extern "C" fn scene_to_json_tuple(scene: usize) -> *mut StringTuple {
    let json = gs::with_scene(scene, |scene| serde_json::to_string(scene).unwrap()).unwrap();
    let mut json = json.into_bytes();
    let ret = StringTuple {
        ptr: json.as_mut_ptr(),
        len: json.len(),
    };
    std::mem::forget(json);
    Box::leak(Box::new(ret))
}

#[cfg_attr(all(not(target_os = "wasi"), feature = "wasm-bindgen"), wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyfunction)]
#[no_mangle]
pub fn scene_from_json(scene: &str) -> usize {
    let Some(scene): Option<Scene> = serde_json::from_str(scene).ok() else {
        return !0usize;
    };
    gs::new_scene(move || scene)
}

#[no_mangle]
pub unsafe extern "C" fn alloc_string(len: usize) -> *mut u8 {
    let mut buffer = Vec::with_capacity(len);
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer); // Prevent Rust from cleaning up the buffer
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn dealloc_string(ptr: *mut u8, len: usize) {
    let _ = Vec::from_raw_parts(ptr, len, 0);
}

#[no_mangle]
pub unsafe extern "C" fn dealloc_strtup(ptr: *mut StringTuple) {
    let _ = Box::from_raw(ptr);
}

#[cfg_attr(all(not(target_os = "wasi"), feature = "wasm-bindgen"), wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyfunction)]
#[no_mangle]
pub extern "C" fn vec3(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3 { x, y, z }
}

#[cfg_attr(all(not(target_os = "wasi"), feature = "wasm-bindgen"), wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyfunction)]
#[no_mangle]
pub extern "C" fn color_conv(colors: TermColorMode) -> ColorConvParams {
    ColorConvParams { colors }
}

#[cfg_attr(all(not(target_os = "wasi"), feature = "wasm-bindgen"), wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyfunction)]
#[no_mangle]
pub extern "C" fn set_obj_transform(scene: usize, obj: usize, pos: Vec3, rot: Vec3, scale: Vec3) {
    gs::with_scene(scene, |scene| {
        scene.objects[obj].transform = create_transform(
            pos.into(),
            na::UnitQuaternion::from_euler_angles(rot.x, rot.y, rot.z),
            scale.into(),
        );
    });
}

#[cfg_attr(all(not(target_os = "wasi"), feature = "wasm-bindgen"), wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyfunction)]
#[no_mangle]
pub fn add_cube(
    scene: usize,
    material: StandardMaterial,
    size: Vec3,
    text: Option<String>,
) -> Option<usize> {
    gs::with_scene(scene, |scene| {
        let id = scene.objects.len();

        scene.objects.push(Object {
            material: material as usize,
            transform: Default::default(),
            text: text.map(|v| v.into()),
            ty: ObjType::Cube { size: size.into() },
        });

        id
    })
}

#[cfg_attr(all(not(target_os = "wasi"), feature = "wasm-bindgen"), wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyfunction)]
#[no_mangle]
pub fn add_line(
    scene: usize,
    material: StandardMaterial,
    start: Vec3,
    end: Vec3,
    text: Option<String>,
) -> Option<usize> {
    gs::with_scene(scene, |scene| {
        let id = scene.objects.len();

        scene.objects.push(Object {
            material: material as usize,
            transform: Default::default(),
            text: text.map(|v| v.into()),
            ty: ObjType::Primitive(Primitive::Line(Line {
                start: start.into(),
                end: end.into(),
            })),
        });

        id
    })
}

#[cfg_attr(all(not(target_os = "wasi"), feature = "wasm-bindgen"), wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyfunction)]
#[no_mangle]
pub fn set_text(scene: usize, object: usize, text: Option<String>) {
    gs::with_scene(scene, |scene| {
        scene.objects[object].text = text.map(|v| v.into());
    });
}

#[cfg_attr(all(not(target_os = "wasi"), feature = "wasm-bindgen"), wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyfunction)]
#[no_mangle]
pub extern "C" fn set_line_points(scene: usize, obj: usize, start: Vec3, end: Vec3) {
    gs::with_scene(scene, |scene| {
        let ObjType::Primitive(Primitive::Line(line)) = &mut scene.objects[obj].ty else {
            return;
        };
        line.start = start.into();
        line.end = end.into();
    });
}

/// Renders a scene into RgbPixel slice.
#[cfg_attr(all(not(target_os = "wasi"), feature = "wasm-bindgen"), wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyfunction)]
#[no_mangle]
pub fn render(scene: usize, color_conv: &ColorConvParams, w: usize, h: usize) -> Vec<RgbPixel> {
    let mut out: Vec<RgbPixel> = vec![];
    gs::render(scene, color_conv, &mut out, w, h);
    out
}

#[no_mangle]
pub extern "C" fn render_raw(
    scene: usize,
    colors: TermColorMode,
    w: usize,
    h: usize,
) -> *mut RgbPixel {
    let mut out = render(scene, &ColorConvParams { colors }, w, h);
    assert_eq!(out.len(), w * h);
    let ptr = out.as_mut_ptr();
    core::mem::forget(out);
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn free_raw_pixels(pixels: *mut RgbPixel, w: usize, h: usize) {
    let _: Vec<RgbPixel> = Vec::from_raw_parts(pixels, w * h, w * h);
}

#[cfg_attr(all(not(target_os = "wasi"), feature = "wasm-bindgen"), wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyfunction)]
#[no_mangle]
pub extern "C" fn update_camera(scene: usize) {
    gs::with_scene(scene, |scene| {
        scene.camera_controller.update(&scene.ctx);
        scene.camera.transform = scene.camera_controller.transform();
    });
}

#[cfg_attr(all(not(target_os = "wasi"), feature = "wasm-bindgen"), wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyclass)]
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum ProjectionMode {
    Perspective = 0,
    Orthographic = 1,
}

#[cfg_attr(all(not(target_os = "wasi"), feature = "wasm-bindgen"), wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyfunction)]
#[no_mangle]
pub extern "C" fn new_frame(
    scene: usize,
    proj_mode: ProjectionMode,
    w: usize,
    h: usize,
    aspect_ratio: f32,
    fov: f32,
    near: f32,
    far: f32,
) {
    gs::with_scene(scene, |scene| {
        match proj_mode {
            ProjectionMode::Perspective => {
                scene.camera.proj =
                    na::Perspective3::new(aspect_ratio, fov.to_radians(), near, far)
                        .to_projective();
            }
            ProjectionMode::Orthographic => {
                let fov = fov * scene.camera_controller.dist;

                scene.camera.proj = ortho_proj(
                    -fov * aspect_ratio,
                    fov * aspect_ratio,
                    -fov,
                    fov,
                    near,
                    far,
                )
                .to_projective();
            }
        }
        scene.camera_controller.fov_y = fov;
        scene.ctx.new_frame(0, 0, w as u16, h as u16);
    });
}

#[cfg_attr(all(not(target_os = "wasi"), feature = "wasm-bindgen"), wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyfunction)]
#[no_mangle]
pub extern "C" fn event_focus(scene: usize, focused: bool) {
    gs::with_scene(scene, |scene| {
        scene.ctx.focused = focused;
    });
}

#[cfg_attr(all(not(target_os = "wasi"), feature = "wasm-bindgen"), wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyfunction)]
#[no_mangle]
pub extern "C" fn event_mouse_pos(scene: usize, x: f32, y: f32) {
    gs::with_scene(scene, |scene| {
        scene.ctx.input.pointer.interact_pos = Some(Vector2::new(x, y));
    });
}

#[cfg_attr(all(not(target_os = "wasi"), feature = "wasm-bindgen"), wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyfunction)]
#[no_mangle]
pub extern "C" fn event_mouse_pos_clear(scene: usize) {
    gs::with_scene(scene, |scene| {
        scene.ctx.input.pointer.interact_pos = None;
    });
}

#[cfg_attr(all(not(target_os = "wasi"), feature = "wasm-bindgen"), wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyfunction)]
#[no_mangle]
pub extern "C" fn event_mouse_button_state(scene: usize, down: bool, primary: bool) {
    gs::with_scene(scene, |scene| {
        let pointer = &mut scene.ctx.input.pointer;
        if primary {
            pointer.primary_down = down;
        } else {
            pointer.secondary_down = down;
        }
    });
}

#[cfg_attr(all(not(target_os = "wasi"), feature = "wasm-bindgen"), wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyfunction)]
#[no_mangle]
pub extern "C" fn event_scroll(scene: usize, x: f32, y: f32) {
    gs::with_scene(scene, |scene| {
        scene.ctx.input.scroll_delta = Vector2::new(x, y);
    });
}

#[cfg_attr(all(not(target_os = "wasi"), feature = "wasm-bindgen"), wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyfunction)]
#[no_mangle]
pub extern "C" fn set_bg_color(scene: usize, col: Vec3) {
    gs::with_scene(scene, |scene| {
        scene.bg.color = col.into();
    });
}

#[cfg_attr(all(not(target_os = "wasi"), feature = "wasm-bindgen"), wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyfunction)]
#[no_mangle]
pub extern "C" fn set_dither_count_frames(scene: usize, count_frames: bool) {
    gs::with_scene(scene, |scene| {
        scene.dithering.set_count_frames(count_frames);
    });
}

#[cfg_attr(all(not(target_os = "wasi"), feature = "wasm-bindgen"), wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyclass)]
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl From<Vec3> for Vector3 {
    fn from(v: Vec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

impl From<Vec3> for Vector4 {
    fn from(v: Vec3) -> Self {
        Self::new(v.x, v.y, v.z, 1.0)
    }
}

impl From<Vector3> for Vec3 {
    fn from(v: Vector3) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
        }
    }
}

#[cfg_attr(all(not(target_os = "wasi"), feature = "wasm-bindgen"), wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyclass)]
#[derive(Clone, Copy)]
#[repr(C)]
pub struct RgbPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub c: u8,
}

impl RgbPixel {
    pub fn to_u32(self) -> u32 {
        let Self { r, g, b, c } = self;
        u32::from_le_bytes([r, g, b, c])
    }

    pub fn from_u32(val: u32) -> Self {
        let [r, g, b, c] = val.to_le_bytes();
        Self { r, g, b, c }
    }
}

impl QuantizePixel for RgbPixel {
    type Params = ColorConvParams;

    fn quantize_color(
        params: &Self::Params,
        inp: Vector3,
        dithering: &impl Dithering,
        x: usize,
        y: usize,
    ) -> Self {
        let [r, g, b] = TermColor::quantize_color(params, inp, dithering, x, y).as_rgb();
        Self {
            r,
            g,
            b,
            c: u8::quantize_color(&(), inp, dithering, x, y),
        }
    }
}

impl PixelDarken for RgbPixel {
    fn darken(&mut self) {
        self.c.darken();

        let Self { r, g, b, .. } = *self;

        let mut col = TermColor::from_rgb([r, g, b]);
        col.darken();
        let [r, g, b] = col.as_rgb();

        *self = Self { r, g, b, ..*self }
    }
}

impl PixelText for RgbPixel {
    fn embed(&mut self, c: char) {
        self.c.embed(c)
    }
}

// TODO: finish this
/*#[cfg(feature = "pyo3")]
#[pymodule]
fn asciirend(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(new_scene, m)?)?;
    Ok(())
}*/
