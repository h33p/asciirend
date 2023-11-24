//! Global state for multiple scenes.
//!
//! This module enables opaque management of scenes. This is primarily focused around exposing
//! functionality over wasm. See [`wasm`](crate::wasm) module for more.

use crate::{
    dithering::XorShufDither,
    extra::{camera_controller::CameraController, Ctx},
    material::*,
    *,
};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Scene {
    #[serde(skip, default)]
    pub ctx: Ctx,
    pub camera: Camera,
    pub camera_controller: CameraController,
    pub objects: Vec<Object>,
    pub bg: Background,
    pub dithering: XorShufDither,
}

#[cfg_attr(feature = "wasm-bindgen", wasm_bindgen::prelude::wasm_bindgen)]
#[cfg_attr(feature = "pyo3", pyo3::pyclass)]
#[derive(Clone, Copy)]
#[repr(usize)]
pub enum StandardMaterial {
    Unlit = 0,
    Diffuse = 1,
    UiText = 2,
}

#[derive(Default)]
struct Scenes {
    scenes: Vec<Option<Arc<Mutex<Scene>>>>,
    free_scenes: Vec<usize>,
}

static MATERIALS: OnceLock<Mutex<Vec<Box<dyn Material + Send>>>> = OnceLock::new();
static SCENES: OnceLock<RwLock<Scenes>> = OnceLock::new();
static RENDERER: OnceLock<Mutex<Renderer>> = OnceLock::new();

fn get_materials() -> &'static Mutex<Vec<Box<dyn Material + Send>>> {
    MATERIALS.get_or_init(|| {
        Mutex::new(vec![
            Box::new(Unlit::default()),
            Box::new(Diffuse::default()),
            Box::new(UiText::default()),
        ])
    })
}

fn get_renderer() -> &'static Mutex<Renderer> {
    RENDERER.get_or_init(Default::default)
}

fn get_scenes() -> &'static RwLock<Scenes> {
    SCENES.get_or_init(|| {
        #[cfg(feature = "wasm")]
        crate::extra::bindings::set_panic_hook();
        RwLock::new(Scenes::default())
    })
}

pub fn new_scene(init: impl FnOnce() -> Scene) -> usize {
    let scenes = get_scenes();
    let mut scenes = scenes.write().unwrap();

    let id = if let Some(id) = scenes.free_scenes.pop() {
        assert!(scenes.scenes[id].is_none());
        id
    } else {
        let id = scenes.scenes.len();
        scenes.scenes.push(None);
        id
    };

    scenes.scenes[id] = Some(Mutex::new(init()).into());

    id
}

pub fn remove_scene(scene: usize) {
    let scenes = get_scenes();
    let mut scenes = scenes.write().unwrap();

    let old = scenes.scenes[scene].take();
    assert!(old.is_some());

    scenes.free_scenes.push(scene);
}

pub fn with_scene<T>(scene: usize, f: impl FnOnce(&mut Scene) -> T) -> Option<T> {
    get_scenes()
        .read()
        .unwrap()
        .scenes
        .get(scene)
        .and_then(|v| v.clone())
        .map(|v| f(&mut v.lock().unwrap()))
}

pub fn render<T: QuantizePixel + PixelText>(
    scene: usize,
    conv_params: &T::Params,
    buf: &mut Vec<T>,
    w: usize,
    h: usize,
) {
    let scene = {
        let scenes = get_scenes().read().unwrap();
        scenes.scenes[scene].clone().unwrap()
    };
    let mut renderer = get_renderer().lock().unwrap();
    let scene = &mut *scene.lock().unwrap();

    renderer.clear_screen(&scene.bg, conv_params, &mut scene.dithering, buf, w, h);

    let mut materials = get_materials().lock().unwrap();

    renderer.render(
        &scene.camera,
        conv_params,
        &mut materials[..],
        &scene.objects,
        &mut scene.dithering,
        buf,
    );

    renderer.text_pass(&scene.objects, buf);
}
