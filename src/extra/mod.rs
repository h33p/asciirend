//! Extra misceleneous structures.

#[cfg(feature = "bindings")]
pub mod bindings;
#[cfg(feature = "global-state")]
pub mod global_state;

pub mod camera_controller;
use super::*;

#[derive(Default)]
pub struct Modifiers {
    pub shift: bool,
}

#[derive(Default)]
pub struct Pointer {
    pub primary_down: bool,
    pub secondary_down: bool,
    pub interact_pos: Option<Vector2>,
    pub modifiers: Modifiers,
}

#[derive(Default)]
pub struct Input {
    pub pointer: Pointer,
    pub scroll_delta: Vector2,
    pub screen_rect: Vector4,
}

/// Defines a context state.
///
/// This state is used by other systems, such as
/// [`CameraController`](camera_controller::CameraController) to perform frame update changes. It
/// is up to the user to decide how to fill the data of `Ctx`, however, with `crossterm` feature
/// enabled, there are specific functions [`Ctx::new_frame`] and [`Ctx::event`] that help with
/// processing raw terminal input.
pub struct Ctx {
    pub focused: bool,
    pub input: Input,
    pub should_stop: bool,
}

impl Default for Ctx {
    fn default() -> Self {
        Self {
            focused: true,
            input: Default::default(),
            should_stop: false,
        }
    }
}

impl Ctx {
    /// Prepares a new frame with given screen dimensions.
    pub fn new_frame(&mut self, x: u16, y: u16, w: u16, h: u16) {
        self.input.pointer.modifiers = Default::default();
        self.input.scroll_delta = Vector2::default();
        self.input.screen_rect = Vector4::new(x as f32, y as f32, w as f32, h as f32);
    }
}

#[cfg(feature = "crossterm")]
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

#[cfg(feature = "crossterm")]
impl Ctx {
    /// Processes a crossterm event.
    pub fn event(&mut self, e: Event) {
        match e {
            Event::FocusGained => self.focused = true,
            Event::FocusLost => self.focused = false,
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => {
                if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
                    self.should_stop = true;
                }
            }
            Event::Mouse(MouseEvent {
                kind,
                column,
                row,
                modifiers,
                ..
            }) => {
                self.input.pointer.modifiers.shift = modifiers.contains(KeyModifiers::SHIFT);
                self.input.pointer.interact_pos = Some(Vector2::new(column as f32, row as f32));
                match kind {
                    MouseEventKind::Down(b) => match b {
                        MouseButton::Left => self.input.pointer.primary_down = true,
                        MouseButton::Right => self.input.pointer.secondary_down = true,
                        _ => (),
                    },
                    MouseEventKind::Up(b) => {
                        self.input.pointer.interact_pos = None;
                        match b {
                            MouseButton::Left => self.input.pointer.primary_down = false,
                            MouseButton::Right => self.input.pointer.secondary_down = false,
                            _ => (),
                        }
                    }
                    MouseEventKind::Drag(b) => match b {
                        MouseButton::Left => self.input.pointer.primary_down = true,
                        MouseButton::Right => self.input.pointer.secondary_down = true,
                        _ => (),
                    },
                    MouseEventKind::ScrollUp => {
                        self.input.scroll_delta.y += 1.0;
                    }
                    MouseEventKind::ScrollDown => {
                        self.input.scroll_delta.y -= 1.0;
                    }
                    MouseEventKind::ScrollLeft => {
                        self.input.scroll_delta.x -= 1.0;
                    }
                    MouseEventKind::ScrollRight => {
                        self.input.scroll_delta.x += 1.0;
                    }
                    _ => (),
                }
            }
            _ => (),
        }
    }
}

pub fn create_transform(
    position: na::Vector3<f32>,
    rotation: na::UnitQuaternion<f32>,
    scale: na::Vector3<f32>,
) -> na::Transform3<f32> {
    // Create a translation matrix
    let translation = na::Translation3::from(position).to_homogeneous();

    // Convert quaternion to a rotation matrix
    let rotation_matrix = rotation.to_homogeneous();

    // Create a scaling matrix
    let scale_matrix = na::Matrix4::new_nonuniform_scaling(&scale);

    // Combine the transformations: T * R * S
    let mat = translation * rotation_matrix * scale_matrix; //translation * rotation_matrix * scale_matrix;

    na::Transform3::from_matrix_unchecked(mat)
}

pub fn ortho_proj(
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    znear: f32,
    zfar: f32,
) -> na::Orthographic3<f32> {
    let m = *na::Orthographic3::new(left, right, bottom, top, znear, zfar).as_matrix();

    #[rustfmt::skip]
    pub const OPENGL_TO_AR_MATRIX: na::Matrix4<f32> = na::matrix![
        1.0, 0.0, 0.0, 0.0;
        0.0, 1.0, 0.0, 0.0;
        0.0, 0.0, 0.5, 0.5;
        0.0, 0.0, 0.0, 1.0
    ];

    na::Orthographic3::from_matrix_unchecked(OPENGL_TO_AR_MATRIX * m)
}
