use super::{Ctx, Vector2};
use nalgebra as na;

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum InMotion {
    None,
    Orbit(Vector2, na::UnitQuaternion<f32>),
    Pan(Vector2, na::Point3<f32>),
}

/// Simple camera controller.
///
/// This controller enables orbit, pan, and zoom motions using mouse input.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CameraController {
    pub fov_y: f32,
    pub focus_point: na::Point3<f32>,
    pub rot: na::UnitQuaternion<f32>,
    pub dist: f32,
    in_motion: InMotion,
    pub scroll_sensitivity: f32,
    pub orbit_sensitivity: f32,
    last_down: bool,
    pressed: bool,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            fov_y: 90.0,
            focus_point: Default::default(),
            rot: na::UnitQuaternion::from_euler_angles(
                -30f32.to_radians(),
                0.0,
                30f32.to_radians(),
            ),
            dist: 1.0,
            in_motion: InMotion::None,
            scroll_sensitivity: 0.02,
            orbit_sensitivity: 1.0,
            last_down: false,
            pressed: false,
        }
    }
}

impl CameraController {
    /// Updates the controller with given context.
    ///
    /// This function is expected to be called once per frame.
    pub fn update(&mut self, ctx: &Ctx) {
        let input = &ctx.input;

        let pressed = input.pointer.primary_down;

        self.pressed = pressed && (self.pressed || !self.last_down);
        self.last_down = pressed;

        self.dist = (self.dist * (1.0 - input.scroll_delta.y * self.scroll_sensitivity)).max(0.1);

        if let Some((pointer, true)) = input.pointer.interact_pos.map(|p| (p, self.pressed)) {
            let pan_key = input.pointer.modifiers.shift;

            match self.in_motion {
                InMotion::None => {
                    if pan_key {
                        self.in_motion = InMotion::Pan(pointer, self.focus_point);
                    } else {
                        self.in_motion = InMotion::Orbit(pointer, self.rot);
                    }
                }
                InMotion::Orbit(pointer_start, start_rot) => {
                    let delta = pointer - pointer_start;

                    // Use euler angles to never have any rolling rotation.
                    let (x, _, z) = start_rot.euler_angles();

                    // The coords are in screen space. Let's divide by the smallest dimension to
                    // have consistent delta across screen sizes.
                    let dim = libm::fminf(input.screen_rect.z, input.screen_rect.w);
                    let delta = delta / dim;

                    self.rot = na::UnitQuaternion::from_euler_angles(
                        x - delta.y * self.orbit_sensitivity,
                        0.0,
                        z - delta.x * self.orbit_sensitivity,
                    );

                    if pan_key {
                        self.in_motion = InMotion::Pan(pointer, self.focus_point);
                    }
                }
                InMotion::Pan(pointer_start, start_pos) => {
                    let delta = pointer - pointer_start;
                    let screen_size = Vector2::new(input.screen_rect.z, input.screen_rect.w);
                    let delta = delta.component_div(&screen_size);
                    let aspect = screen_size.x / screen_size.y;
                    let fov = self.fov_y.to_radians() / 2.0;

                    // A little bit geometry to move the pan the camera in a pixel perfect way.
                    let move_delta = na::matrix![
                        -libm::tanf(fov) * delta.x * aspect;
                        0.0;
                        libm::tanf(fov) * delta.y
                    ] * (self.dist * 2.0);

                    self.focus_point = start_pos + self.rot * move_delta;

                    if !pan_key {
                        self.in_motion = InMotion::Orbit(pointer, self.rot);
                    }
                }
            }
        } else {
            self.in_motion = InMotion::None;
        }
    }

    /// Gets the current camera transformation.
    pub fn transform(&self) -> na::Transform3<f32> {
        let dir = self.rot * na::matrix![0.0; -1.0; 0.0];
        super::create_transform(
            self.focus_point.coords + dir * self.dist,
            self.rot,
            na::vector![1.0, 1.0, 1.0],
        )
    }
}
