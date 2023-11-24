use super::*;

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
    fn fragment_shade(&self, primitive: usize, pos: Vector2, depth: f32) -> Option<Vector4>;
}

impl AsMut<dyn Material> for dyn Material {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl AsMut<dyn Material + Send> for dyn Material + Send {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

/// Very simple form of material - shade everything gray
#[derive(Default)]
pub struct Unlit {
    idx: usize,
}

impl Material for Unlit {
    fn new_frame(&mut self) {
        self.idx = 0;
    }

    fn primitive_shade(
        &mut self,
        mut pri: Primitive,
        proj: na::Matrix4<f32>,
        model: na::Matrix4<f32>,
    ) -> (usize, Primitive) {
        let idx = self.idx;
        self.idx += 1;

        match &mut pri {
            Primitive::Triangle(Triangle { a, b, c }) => {
                *a = proj * model * *a;
                *b = proj * model * *b;
                *c = proj * model * *c;
            }
            Primitive::Line(Line { start, end }) => {
                *start = proj * model * *start;
                *end = proj * model * *end;
            }
        };

        (idx, pri)
    }

    fn fragment_shade(&self, _: usize, _pos: Vector2, _: f32) -> Option<Vector4> {
        Some(na::vector![1.0, 1.0, 1.0, 1.0] * 0.5)
    }
}

/// Simple shader that represents world-space vertext normals as fragment colors.
pub struct Diffuse {
    ambient: Vector3,
    light_dir: Vector3,
    light_col: Vector3,
    normals: Vec<Vector3>,
}

impl Default for Diffuse {
    fn default() -> Self {
        Self {
            ambient: na::vector![0.1, 0.13, 0.25] * 5.0,
            light_dir: na::vector![0.5, 0.5, -0.5].normalize(),
            light_col: na::vector![0.7, 0.4, 0.1] * 10.0,
            normals: alloc::vec![],
        }
    }
}

impl Material for Diffuse {
    fn new_frame(&mut self) {
        self.normals.clear();
    }

    fn primitive_shade(
        &mut self,
        mut pri: Primitive,
        proj: na::Matrix4<f32>,
        model: na::Matrix4<f32>,
    ) -> (usize, Primitive) {
        let idx = self.normals.len();

        let normal = match &mut pri {
            Primitive::Triangle(Triangle { a, b, c }) => {
                *a = model * *a;
                *b = model * *b;
                *c = model * *c;

                let e1 = a.xyz() - b.xyz();
                let e2 = c.xyz() - b.xyz();

                let n = e1.cross(&e2).normalize();

                *a = proj * *a;
                *b = proj * *b;
                *c = proj * *c;

                n
            }
            Primitive::Line(Line { start, end }) => {
                *start = proj * model * *start;
                *end = proj * model * *end;

                Default::default()
            }
        };

        self.normals.push(normal);

        (idx, pri)
    }

    fn fragment_shade(&self, triangle: usize, _pos: Vector2, _: f32) -> Option<Vector4> {
        let light_dot = self.normals[triangle].dot(&self.light_dir);
        let light = self.light_col * libm::fmaxf(0.0, libm::fminf(light_dot, 1.0));
        let color = self.ambient + light;

        // Apply tone mapping
        let color = color.component_div(&(color + na::vector![1.0, 1.0, 1.0]));

        Some(na::vector![color.x, color.y, color.z, 1.0])
    }
}

/// Text-only screen-space rendering
///
/// Implies orthographic projection with clip bounds of:
///
/// - X 0-100.
/// - Y 0-100.
/// - Z 1-1000.
pub struct UiText {
    idx: usize,
    proj: na::Matrix4<f32>,
}

impl Default for UiText {
    fn default() -> Self {
        let dir = na::vector![0.0, 1.0, 0.0];
        let view = Matrix4::look_at_rh(
            &Default::default(),
            &dir.into(),
            &na::vector![0.0, 0.0, 1.0],
        );

        Self {
            idx: 0,
            proj: crate::extra::ortho_proj(0.0, 100.0, 0.0, 100.0, 1.0, 1000.0).as_matrix() * view,
        }
    }
}

impl Material for UiText {
    fn new_frame(&mut self) {
        self.idx = 0;
    }

    fn primitive_shade(
        &mut self,
        mut pri: Primitive,
        _: na::Matrix4<f32>,
        model: na::Matrix4<f32>,
    ) -> (usize, Primitive) {
        let idx = self.idx;
        self.idx += 1;

        let proj = self.proj;

        match &mut pri {
            Primitive::Triangle(Triangle { a, b, c }) => {
                *a = proj * model * *a;
                *b = proj * model * *b;
                *c = proj * model * *c;
            }
            Primitive::Line(Line { start, end }) => {
                *start = proj * model * *start;
                *end = proj * model * *end;
            }
        };

        (idx, pri)
    }

    fn fragment_shade(&self, _: usize, _pos: Vector2, _: f32) -> Option<Vector4> {
        Some(na::vector![1.0, 1.0, 1.0, 0.0])
    }
}
