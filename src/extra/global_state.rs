//! Global state for multiple scenes.
//!
//! This module enables opaque management of scenes. This is primarily focused around exposing
//! functionality over wasm. See [`wasm`](crate::wasm) module for more.

use crate::{
    dithering::XorShufDither,
    extra::{camera_controller::CameraController, ortho_proj, Ctx},
    material::*,
    *,
};
use core::cell::RefCell;
#[cfg(feature = "scripting")]
use rhai::{Dynamic, Engine, Scope, AST};
use std::rc::Rc;
use std::sync::Arc;

#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[serde(default)]
pub struct Scene {
    #[serde(skip)]
    pub ctx: Ctx,
    #[serde(skip)]
    pub camera: Camera,
    pub camera_props: CameraProperties,
    pub camera_controller: CameraController,
    pub objects: Vec<Object>,
    pub bg: Background,
    pub dithering: XorShufDither,
    #[cfg(feature = "scripting")]
    pub script: Option<Arc<str>>,
    #[serde(skip)]
    pub frames: usize,
}

impl Scene {
    fn update_camera(&mut self) {
        let proj = match self.camera_props.proj_mode {
            ProjectionMode::Perspective => na::Perspective3::new(
                self.camera_props.aspect_ratio,
                self.camera_props.fov.to_radians(),
                self.camera_props.near,
                self.camera_props.far,
            )
            .to_projective(),
            ProjectionMode::Orthographic => {
                let fov = self.camera_props.fov * self.camera_controller.dist;

                ortho_proj(
                    -fov * self.camera_props.aspect_ratio,
                    fov * self.camera_props.aspect_ratio,
                    -fov,
                    fov,
                    self.camera_props.near,
                    self.camera_props.far,
                )
                .to_projective()
            }
        };
        self.camera_controller.fov_y = self.camera_props.fov;
        self.camera.proj = proj;
    }
}

#[cfg_attr(
    all(not(target_os = "wasi"), feature = "wasm-bindgen"),
    wasm_bindgen::prelude::wasm_bindgen
)]
#[cfg_attr(feature = "pyo3", pyclass)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum ProjectionMode {
    Perspective = 0,
    Orthographic = 1,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CameraProperties {
    pub proj_mode: ProjectionMode,
    #[serde(default)]
    pub aspect_ratio: f32,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for CameraProperties {
    fn default() -> Self {
        Self {
            proj_mode: ProjectionMode::Perspective,
            aspect_ratio: 1.0,
            fov: 60.0,
            near: 0.01,
            far: 100.0,
        }
    }
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

struct SceneAux {
    #[cfg(feature = "scripting")]
    last_script: Option<Arc<str>>,
    #[cfg(feature = "scripting")]
    compiled_script: Option<AST>,
    #[cfg(feature = "scripting")]
    scope: Scope<'static>,
    #[cfg(feature = "scripting")]
    state: Dynamic,
    start: Option<f64>,
}

impl SceneAux {
    fn new(scene: &mut Scene, scenes: &Scenes) -> Self {
        let mut ret = Self {
            #[cfg(feature = "scripting")]
            last_script: None,
            #[cfg(feature = "scripting")]
            compiled_script: None,
            #[cfg(feature = "scripting")]
            scope: Scope::new(),
            #[cfg(feature = "scripting")]
            state: Default::default(),
            start: None,
        };

        ret.update_scripts(scene, scenes);

        ret
    }
}

impl SceneAux {
    pub fn update_scripts(&mut self, scene: &mut Scene, scenes: &Scenes) {
        #[cfg(feature = "scripting")]
        if scene.script.as_ref().map(|v| v.as_ptr())
            != self.last_script.as_ref().map(|v| v.as_ptr())
        {
            self.last_script = scene.script.clone();
            if let Some(script) = self.last_script.as_ref() {
                self.compiled_script = Some(
                    scenes
                        .engine
                        .compile_with_scope(&mut self.scope, &script)
                        .unwrap(),
                );
            } else {
                self.compiled_script = None;
            }
        }
    }
}

thread_local! {
    static MATERIALS: Rc<RefCell<Vec<Box<dyn Material + Send>>>> =
        Rc::new(RefCell::new(vec![
            Box::new(Unlit::default()) as Box<dyn Material + Send>,
            Box::new(Diffuse::default()),
            Box::new(UiText::default()),
        ]))
    ;
    static SCENES: Rc<RefCell<Scenes>> = {
        #[cfg(feature = "wasm")]
        crate::extra::bindings::set_panic_hook();
        Default::default()
    };
    static RENDERER: Rc<RefCell<Renderer>> = Default::default();
}

struct Scenes {
    scenes: Vec<Option<(Rc<RefCell<Scene>>, Rc<RefCell<SceneAux>>)>>,
    free_scenes: Vec<usize>,
    #[cfg(feature = "scripting")]
    engine: Engine,
}

impl Default for Scenes {
    fn default() -> Self {
        Self {
            scenes: vec![],
            free_scenes: vec![],
            #[cfg(feature = "scripting")]
            engine: {
                type UnitQuaternion = na::UnitQuaternion<f32>;
                type Transform3 = na::Transform3<f32>;
                type SceneRef = Rc<RefCell<Scene>>;

                let mut engine = Engine::new();

                engine
                    .set_strict_variables(true)
                    .register_fn("sin", f32::sin)
                    .register_fn("cos", f32::cos)
                    .register_fn("atan2", f32::atan2)
                    .register_type::<SceneRef>()
                    .register_fn(
                        "set_transform",
                        |s: SceneRef, obj: i64, transform: Transform3| {
                            s.borrow_mut().objects[obj as usize].transform = transform;
                        },
                    )
                    .register_fn("get_position", |s: SceneRef, obj: i64| {
                        s.borrow_mut().objects[obj as usize]
                            .transform
                            .transform_point(&Default::default())
                            .coords
                    })
                    .register_fn("set_text", |s: SceneRef, obj: i64, _: ()| {
                        s.borrow_mut().objects[obj as usize].text = None;
                    })
                    .register_fn("set_text", |s: SceneRef, obj: i64, text: &str| {
                        s.borrow_mut().objects[obj as usize].text = Some(text.into());
                    })
                    .register_set("camera_fov", |s: &mut SceneRef, fov: f32| {
                        s.borrow_mut().camera_props.fov = fov;
                    })
                    .register_set("camera_dist", |s: &mut SceneRef, dist: f32| {
                        s.borrow_mut().camera_controller.dist = dist;
                    })
                    .register_set("camera_focus", |s: &mut SceneRef, focus: Vector3| {
                        s.borrow_mut().camera_controller.focus_point = focus.into();
                    })
                    .register_set(
                        "camera_rotation",
                        |s: &mut SceneRef, rotation: UnitQuaternion| {
                            s.borrow_mut().camera_controller.rot = rotation;
                        },
                    )
                    .register_get("frames", |s: &mut SceneRef| s.borrow_mut().frames as i64)
                    .register_type::<Vector4>()
                    .register_fn("vec4", Vector4::new)
                    .register_get("x", |v: &mut Vector4| v.x)
                    .register_get("y", |v: &mut Vector4| v.y)
                    .register_get("z", |v: &mut Vector4| v.z)
                    .register_get("w", |v: &mut Vector4| v.w)
                    .register_set("x", |v: &mut Vector4, x: f32| v.x = x)
                    .register_set("y", |v: &mut Vector4, y: f32| v.y = y)
                    .register_set("z", |v: &mut Vector4, z: f32| v.z = z)
                    .register_set("w", |v: &mut Vector4, w: f32| v.w = w)
                    .register_type::<Vector3>()
                    .register_fn("vec3", Vector3::new)
                    .register_get("x", |v: &mut Vector3| v.x)
                    .register_get("y", |v: &mut Vector3| v.y)
                    .register_get("z", |v: &mut Vector3| v.z)
                    .register_set("x", |v: &mut Vector3, x: f32| v.x = x)
                    .register_set("y", |v: &mut Vector3, y: f32| v.y = y)
                    .register_set("z", |v: &mut Vector3, z: f32| v.z = z)
                    .register_type::<Vector2>()
                    .register_fn("vec2", Vector2::new)
                    .register_get("x", |v: &mut Vector2| v.x)
                    .register_get("y", |v: &mut Vector2| v.y)
                    .register_set("x", |v: &mut Vector2, x: f32| v.x = x)
                    .register_set("y", |v: &mut Vector2, y: f32| v.y = y)
                    .register_type::<Transform3>()
                    .register_type::<UnitQuaternion>()
                    .register_fn(
                        "uq_from_euler",
                        nalgebra::UnitQuaternion::<f32>::from_euler_angles,
                    )
                    .register_fn("create_transform", crate::extra::create_transform);

                engine
            },
        }
    }
}

fn get_materials() -> Rc<RefCell<Vec<Box<dyn Material + Send>>>> {
    MATERIALS.with(Clone::clone)
}

fn get_renderer() -> Rc<RefCell<Renderer>> {
    RENDERER.with(Clone::clone)
}

fn get_scenes() -> Rc<RefCell<Scenes>> {
    SCENES.with(Clone::clone)
}

pub fn new_scene(init: impl FnOnce() -> Scene) -> usize {
    let scenes = get_scenes();
    let mut scenes = scenes.borrow_mut();

    let id = if let Some(id) = scenes.free_scenes.pop() {
        assert!(scenes.scenes[id].is_none());
        id
    } else {
        let id = scenes.scenes.len();
        scenes.scenes.push(None);
        id
    };

    let mut scene = init();
    let aux = SceneAux::new(&mut scene, &scenes);
    scenes.scenes[id] = Some((RefCell::new(scene).into(), RefCell::new(aux).into()));

    id
}

pub fn remove_scene(scene: usize) {
    let scenes = get_scenes();
    let mut scenes = scenes.borrow_mut();

    let old = scenes.scenes[scene].take();
    assert!(old.is_some());

    scenes.free_scenes.push(scene);
}

pub fn with_scene<T>(scene: usize, f: impl FnOnce(&mut Scene) -> T) -> Option<T> {
    get_scenes()
        .borrow()
        .scenes
        .get(scene)
        .and_then(|v| v.clone())
        .map(|v| f(&mut v.0.borrow_mut()))
}

pub fn render<T: QuantizePixel + PixelText>(
    scene: usize,
    conv_params: &T::Params,
    buf: &mut Vec<T>,
    w: usize,
    h: usize,
    elapsed: f64,
) {
    let scenes = get_scenes();
    let scenes = scenes.borrow();
    let scene = scenes.scenes[scene].clone().unwrap();
    let renderer = get_renderer();
    let mut renderer = renderer.borrow_mut();

    #[cfg(feature = "scripting")]
    {
        let aux = &mut *scene.1.borrow_mut();

        let elapsed = if let Some(s) = aux.start {
            elapsed - s
        } else {
            aux.start = Some(elapsed);
            0.0
        };

        aux.update_scripts(&mut *scene.0.borrow_mut(), &scenes);
        if let Some(ast) = aux.compiled_script.as_ref() {
            aux.state = scenes
                .engine
                .call_fn::<Dynamic>(
                    &mut aux.scope,
                    ast,
                    "frame_update",
                    (
                        core::mem::take(&mut aux.state),
                        scene.0.clone(),
                        elapsed as f32,
                    ),
                )
                .expect("Script crashed");
        }
    }

    let scene = &mut *scene.0.borrow_mut();

    scene.frames += 1;
    scene.camera_controller.update(&scene.ctx);
    scene.camera.transform = scene.camera_controller.transform();
    scene.update_camera();

    renderer.clear_screen(&scene.bg, conv_params, &mut scene.dithering, buf, w, h);

    let materials = get_materials();
    let mut materials = materials.borrow_mut();

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
