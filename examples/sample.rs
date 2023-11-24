use asciirend::{
    color::{ColorConvParams, TermColorMode},
    dithering::XorShufDither,
    extra::{camera_controller::CameraController, create_transform, Ctx},
    material::{Diffuse, Material},
    *,
};
use crossterm::{
    cursor,
    event::{self, KeyboardEnhancementFlags},
    style, terminal, QueueableCommand,
};
use nalgebra as na;
use std::io::{stdout, Write};

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

fn main() -> anyhow::Result<()> {
    let mut buf: Vec<(style::Colors, u8)> = vec![];

    let aspect = 16.0 / 9.0;

    let mut proj = na::Perspective3::new(1f32, 90f32.to_radians(), 0.1, 500.0);

    proj.set_aspect(aspect);

    let mut camera = Camera::new(proj.to_projective());
    let mut bg = Background::default();

    let mut renderer = Renderer::default();

    // Just to be sure
    println!("Hello, world!");

    let mut stdout = stdout();
    stdout.queue(cursor::Hide)?;
    stdout.queue(event::EnableMouseCapture)?;
    stdout.queue(event::PushKeyboardEnhancementFlags(
        KeyboardEnhancementFlags::REPORT_EVENT_TYPES
            | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
            | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS,
    ))?;

    // In case we get an outside sigterm/sigint, we want to gracefully shutdown without leaving the
    // terminal in raw mode.
    let stop = Arc::new(AtomicBool::new(false));

    signal_hook::flag::register(signal_hook::consts::SIGTERM, stop.clone())?;
    signal_hook::flag::register(signal_hook::consts::SIGINT, stop.clone())?;

    let mut frame = 0;
    let time = Instant::now();

    let _color = bg.color;

    let materials: &mut [Box<dyn Material>] = &mut [
        Box::new(Diffuse::default()),
        Box::new(NormalShading::default()),
    ];

    // Create 3 objects - 2 cubes and 1 line.
    //
    // First cube will rotate in place, second cube will rotate in place, and translate up and
    // down.
    //
    // The line endpoints will be set to the world space origin points of the cubes.
    let mut objects = [
        Object {
            transform: Default::default(),
            material: 0,
            ty: ObjType::Cube {
                size: Vector3::new(1.0, 1.0, 1.0),
            },
            text: Some("asciirend".into()),
        },
        Object {
            transform: Default::default(),
            material: 1,
            ty: ObjType::Cube {
                size: Vector3::new(1.0, 1.0, 1.0),
            },
            text: Some("Example 2".into()),
        },
        Object {
            transform: Default::default(),
            material: 0,
            ty: ObjType::Primitive(Primitive::Line(Line::default())),
            text: None,
        },
    ];

    terminal::enable_raw_mode()?;

    let (tx, rx) = std::sync::mpsc::channel();
    let _ = std::thread::spawn(move || {
        while let Ok(e) = event::read() {
            if tx.send(e).is_err() {
                break;
            }
        }
    });

    let mut evts = 0;
    let mut levt = None;

    let mut ctx = Ctx::default();
    let mut cam_control = CameraController::default();
    cam_control.dist = 30.0;

    // Dithering enables perceptually smoother color transitions by quantizing output color
    // values with different noise offsets, depending on different pixels.
    let mut dithering = XorShufDither::default();

    while !stop.load(Ordering::SeqCst) && !ctx.should_stop {
        let start = time.elapsed();

        let (x, y) = term_char_aspect();
        let (w, _h) = terminal::size()?;
        let w = w as usize;
        let h = (((w * x) as f32) / (aspect * y as f32)) as usize;

        // Process events
        const Y_OFF: u16 = 3;
        ctx.new_frame(0, Y_OFF, w as _, h as _);

        while let Ok(e) = rx.try_recv() {
            evts += 1;
            levt = Some(e.clone());
            ctx.event(e);
        }

        cam_control.update(&mut ctx);

        let events = time.elapsed();

        // Shift through the background color
        let color2 = colorsys::Hsl::new((start.as_secs_f64() * 30.0) % 360.0, 1.0, 5.0, None);

        let color = colorsys::Rgb::from(&color2);
        let color = Vector3::new(
            color.red() as f32,
            color.green() as f32,
            color.blue() as f32,
        ) / 255.0;
        bg.color = color;

        // Update object and camera transformations

        camera.transform = cam_control.transform();

        // Just to push positions that are used to help the line renderer
        let mut positions = vec![];

        for (i, obj) in objects.iter_mut().enumerate() {
            if let ObjType::Primitive(Primitive::Line(line)) = &mut obj.ty {
                line.start = positions[positions.len() - 2];
                line.end = positions[positions.len() - 1];
                obj.transform = create_transform(
                    Default::default(),
                    na::UnitQuaternion::identity(),
                    na::vector![1.0, 1.0, 1.0],
                );
            } else {
                let pos = na::vector![
                    (i as f32) * 250.0,
                    (i as f32) * -75.0,
                    -2.0 - 5.5 * (i as f32) * (f32::sin(start.as_secs_f32()))
                ];
                positions.push(Vector4::new(pos.x, pos.y, pos.z, 1.0));
                obj.transform = create_transform(
                    pos,
                    na::UnitQuaternion::from_euler_angles(0.0, 0.0, start.as_secs_f32() * 0.5),
                    na::vector![30.1, 30.1, 30.1],
                );
            }
        }

        // We are able to set how colors will be represented on the terminal
        let color_conv = ColorConvParams {
            colors: TermColorMode::Col256,
        };
        let conv_params = (color_conv, ());

        let updates = time.elapsed();

        // Perform actual rendering

        renderer.clear_screen(&bg, &conv_params, &mut dithering, &mut buf, w, h);

        renderer.render(
            &camera,
            &conv_params,
            materials,
            &objects,
            &mut dithering,
            &mut buf,
        );

        renderer.text_pass(&objects, &mut buf);

        let rendered = time.elapsed();

        for (y, row) in buf.chunks(w).enumerate() {
            stdout.queue(cursor::MoveTo(0 as u16, y as u16 + Y_OFF))?;
            for (_x, (cols, val)) in row.iter().enumerate() {
                stdout.queue(style::SetColors(*cols))?;
                stdout.queue(style::Print(*val as char))?;
            }
            stdout.queue(style::Print('\n'))?;
        }

        let drawn = time.elapsed();

        if Y_OFF > 0 {
            stdout.queue(cursor::MoveTo(0, Y_OFF))?;
            stdout.queue(terminal::Clear(terminal::ClearType::FromCursorUp))?;
        }
        stdout.queue(cursor::MoveTo(0, 0))?;
        stdout.queue(style::SetColors(style::Colors {
            background: None,
            foreground: Some(style::Color::White),
        }))?;

        let v = format!(
            "{frame} {:.02}FPS ({:.02} => {:.02} + {:.02} + {:.02} + {:.02}) @ {cam_control:?} + {evts} ({levt:?})",
            1.0 / (drawn - start).as_secs_f32(),
            (drawn - start).as_secs_f32() * 1000.0,
            (updates - events).as_secs_f32() * 1000.0,
            (rendered - updates).as_secs_f32() * 1000.0,
            (rendered - updates).as_secs_f32() * 1000.0,
            (drawn - rendered).as_secs_f32() * 1000.0,
        );
        stdout.queue(style::Print(v))?;

        stdout.flush()?;

        let drawn = time.elapsed();
        let drawn_delta = drawn - start;

        let frametime_target = Duration::from_millis(0);

        if drawn_delta < frametime_target {
            std::thread::sleep(frametime_target - drawn_delta);
        }

        let _slept = time.elapsed();
        frame += 1;
    }

    terminal::disable_raw_mode()?;

    stdout.queue(style::ResetColor)?;
    stdout.queue(event::PopKeyboardEnhancementFlags)?;
    stdout.queue(event::DisableMouseCapture)?;
    stdout.queue(cursor::Show)?;
    stdout.flush()?;

    Ok(())
}

/// Simple shader that represents world-space vertext normals as fragment colors.
#[derive(Default)]
struct NormalShading {
    normals: Vec<na::Vector3<f32>>,
}

impl Material for NormalShading {
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
                *start = model * proj * *start;
                *end = model * proj * *end;

                Default::default()
            }
        };

        self.normals.push(normal);

        (idx, pri)
    }

    fn fragment_shade(&self, triangle: usize, _pos: Vector2, _: f32) -> Option<na::Vector4<f32>> {
        let color = (self.normals[triangle] + na::vector![1.0, 1.0, 1.0]) * 0.5;
        Some(na::vector![color.x, color.y, color.z, 1.0])
    }
}
