use std::sync::atomic::{AtomicPtr, Ordering::*};
use std::{env,fs};
use std::io::ErrorKind::*;

use common::*;

extern crate opengl_graphics;
use self::opengl_graphics::{OpenGL, GlGraphics};

extern crate piston_window;
use self::piston_window::{Event,Loop,RenderArgs,UpdateArgs,Input}; // from piston_input
use self::piston_window::{ButtonArgs,ButtonState,Button,Motion}; // from piston_input
use self::piston_window::MouseButton as pwMouseButton; // from piston_input
use self::piston_window::{Context,Transformed,color}; // from piston2d-graphics
use self::piston_window::draw_state::Blend; // from piston2d-graphics
use self::piston_window::PistonWindow;
use self::piston_window::WindowSettings; // from piston::window
use self::piston_window::Events; // from piston::event_loop
#[cfg(debug_assertions)]
use self::piston_window::Key; // from piston_input

#[cfg(debug_assertions)]
extern crate dlopen;
#[cfg(debug_assertions)]
use self::dlopen::raw::Library;

pub fn hex(s: &str) -> Color {
    piston_window::color::hex(s)
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

#[inline(never)]
pub fn start(s: StartUpInfo,  functions: Functions) {
    // AtomicPtr because I intend to update from another thread based on inotify events
    let f = AtomicPtr::new(Box::leak(Box::new(functions)));
    run(s, &f);
}

#[cfg(debug_assertions)]
fn reload() -> Option<Functions> {
    unsafe {
        let path = match env::current_exe() {
            Ok(path) => path,
            Err(e) => {
                eprintln!("Cannot get path of current executable: {}", e);
                return None;
            }
        };
        let mut path = match path.into_os_string().into_string() {
            Ok(path) => path,
            Err(_) => {
                eprintln!("Non-unicode paths are not supported, sorry.");
                return None;
            }
        };
        // (on linux) current_exe() appends " (deleted)" when the file has been replaced
        if path.ends_with(" (deleted)") {
            let len = path.len();
            path.truncate(len-" (deleted)".len());
        }
        // (on linux) dlopen refuses to open the same path multiple times
        for reload in 1.. {
            let new_name = format!("{}-reload.{}", path, reload);
            match fs::hard_link(&path, &new_name) {
                Ok(_) => {
                    path = new_name;
                    break;
                }
                Err(ref e) if e.kind() != AlreadyExists => {
                    eprintln!("link {:?} to {:?} failed with {}", path, new_name, e);
                    return None;
                }
                Err(_) => {}
            }
        }
        println!("Trying to reload game functions from {:?}", path);
        let lib = match Library::open(&path) {
            // leak the handle because unloading is very risky,
            // this should only happen a limited number of times,
            // and restarting isn't that bad either
            Ok(lib) => Box::leak(Box::new(lib)),
            Err(e) => {
                eprintln!("Failed to open {:?} as library: {}", path, e);
                return None;
            }
        };
        let functions = (
            lib.symbol("game_render"),
            lib.symbol("game_update"),
            lib.symbol("game_mouse_move"),
            lib.symbol("game_mouse_press"),
        );
        match functions {
            (Ok(render), Ok(update), Ok(mouse_move), Ok(mouse_press)) => {
                Some(Functions{render, update, mouse_move, mouse_press})
            }
            _ => {
                eprintln!("{:?} is missing symbols", path);
                None
            }
        }
    }
}

fn run(s: StartUpInfo,  functions: &AtomicPtr<Functions>) {
    if cfg!(feature="dyn") {
        println!("dyn");
    }
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

            #[cfg(debug_assertions)]
            Event::Input(Input::Button(ButtonArgs {
                    state: ButtonState::Press,
                    button: Button::Keyboard(Key::R),
                    ..
            })) => {
                if let Some(new_functions) = reload() {
                    println!("before: mouse_press={:p}->{:p}", f, f.mouse_press);
                    functions.store(Box::leak(Box::new(new_functions)), SeqCst);
                    let f = unsafe{&*functions.load(SeqCst)};
                    println!("after: mouse_press={:p}->{:p}", f, f.mouse_press);
                }
            }

            _ => {}
        }
    }
}
