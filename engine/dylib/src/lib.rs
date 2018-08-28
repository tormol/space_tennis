use std::sync::atomic::{AtomicPtr, Ordering::*};

extern crate opengl_graphics;
use opengl_graphics::{OpenGL, GlGraphics};

extern crate piston_window;
use piston_window::{Event,Loop,RenderArgs,UpdateArgs,Input}; // from piston_input
use piston_window::{ButtonArgs,ButtonState,Button,Motion}; // from piston_input
use piston_window::MouseButton as pwMouseButton;
use piston_window::{Context,Transformed,color}; // from piston2d-graphics
use piston_window::draw_state::Blend; // from piston2d-graphics
use piston_window::PistonWindow;
use piston_window::WindowSettings; // from piston::window
use piston_window::Events; // from piston::event_loop


#[derive(Debug, Clone,Copy, PartialEq,Eq)]
pub enum MouseButton {
    Unknown,
    Left,
    Right,
    Middle,
    X1,
    X2,
    Button6,
    Button7,
    Button8,
}


pub type Color = [f32;4];//piston_window::types::Color;
pub type Matrix2d = [[f64;3];2];//
pub fn hex(s: &str) -> Color {
    piston_window::color::hex(s)
}
pub trait Graphics {
    fn line(&mut self, Color, f64, [f64;4], Matrix2d);
    fn rectangle(&mut self, Color, [f64;4], Matrix2d);
    fn ellipse(&mut self, Color, [f64;4], Matrix2d);
}
struct GlWrap<'a>(&'a mut GlGraphics);
impl<'a> Graphics for GlWrap<'a> {
    fn line(&mut self,  color: Color,  width: f64,  where_: [f64;4],  transform: Matrix2d) {
        piston_window::line(color, width, where_, transform, self.0)
    }
    fn rectangle(&mut self,  color: Color,  area: [f64;4],  transform: Matrix2d) {
        piston_window::rectangle(color, area, transform, self.0)
    }
    fn ellipse(&mut self,  color: Color,  where_: [f64;4],  transform: Matrix2d) {
        piston_window::ellipse(color, where_, transform, self.0)
    }
}

pub struct Functions {
    pub render: unsafe fn(*mut u8,  Matrix2d,  &mut dyn Graphics),
    pub update: unsafe fn(*mut u8,  f64),
    pub mouse_move: unsafe fn(*mut u8,  [f64; 2]),
    pub mouse_press: unsafe fn(*mut u8,  MouseButton),
}

struct StartUpInfo {
    name: &'static str,
    initial_size: [f64; 2],
    game: *mut u8,
}

fn map_button(b: pwMouseButton) -> MouseButton {
    match b {
        pwMouseButton::Unknown => MouseButton::Unknown,
        pwMouseButton::Left => MouseButton::Left,
        pwMouseButton::Right => MouseButton::Right,
        pwMouseButton::Middle => MouseButton::Middle,
        pwMouseButton::X1 => MouseButton::X1,
        pwMouseButton::X2 => MouseButton::X2,
        pwMouseButton::Button6 => MouseButton::Button6,
        pwMouseButton::Button7 => MouseButton::Button7,
        pwMouseButton::Button8 => MouseButton::Button8,
    }
}

pub fn start(name: &'static str,  initial_size: [f64;2],  game: *mut u8,  functions: Functions) {
    let f = AtomicPtr::new(Box::into_raw(Box::new(functions)));
    let s = StartUpInfo {name, initial_size, game};
    run(s, &f);
}

fn run(s: StartUpInfo,  functions: &AtomicPtr<Functions>) {
    let window_size = [s.initial_size[0] as u32, s.initial_size[1] as u32];
    let mut window: PistonWindow = WindowSettings::new(s.name, window_size)
        .vsync(true)
        .opengl(OpenGL::V3_2)
        .build()
        .unwrap();
    let mut gfx = GlGraphics::new(OpenGL::V3_2);

    let mut size = s.initial_size; // changes if window is resized

    let mut event_loop: Events = window.events;
    while let Some(e) = event_loop.next(&mut window) {
        let f = unsafe{&*functions.load(SeqCst)};
        match e {
            Event::Loop(Loop::Render(render_args)) => {
                let render_args: RenderArgs = render_args;
                // An optimization introduced in opengl_graphics 0.39.1 causes
                // severe glitching if not wrapped in `gfx.draw()`.
                // (just calling it at the end with an empty closure
                //  seems to work too, for now...)
                gfx.draw(render_args.viewport(), |context, gfx| {
                    let context: Context = context;
                    let gfx: &mut GlGraphics = gfx; // the same instance as outside
                    size = context.get_view_size();
                    let context = context.scale(size[0], size[1]);

                    // by default alpha blending is disabled, which means all
                    // semi-transparent colors are considered opaque.
                    // Blend::Alpha blends colors pixel for pixel,
                    // which has a performance cost.
                    // The alternative would be to check for an existing color
                    // in the tile, and blend manually or even statically.
                    context.draw_state.blend(Blend::Alpha);
                    piston_window::clear(color::BLACK, gfx);
                    let wrapper = &mut GlWrap(gfx);
                    unsafe{ (f.render)(s.game, context.transform, wrapper) };
                });
            }
            Event::Loop(Loop::Update(update_args)) => {
                let UpdateArgs{dt: deltatime} = update_args;
                unsafe{ (f.update)(s.game, deltatime) };
            }

            Event::Input(Input::Button(ButtonArgs {
                    state: ButtonState::Press,
                    button: Button::Mouse(button),
                    ..
            })) => {
                unsafe{ (f.mouse_press)(s.game, map_button(button)) };
            }
            Event::Input(Input::Move(Motion::MouseCursor(x,y))) => {
                unsafe{ (f.mouse_move)(s.game, [x/size[0], y/size[1]]) };
            }
            // TODO pause when window loses focos (!= mouse leaves)
            _ => {}
        }
    }
}
