use common::*;
use reload::*;

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
    let f = FunctionGetter::new(functions, s.src.clone());
    run(s, f);
}


fn run(s: StartUpInfo,  functions: FunctionGetter) {
    let window_size = [s.initial_size[0] as u32, s.initial_size[1] as u32];
    let mut window: PistonWindow = WindowSettings::new(s.name.to_owned(), window_size)
        .vsync(true)
        .opengl(OpenGL::V3_2)
        .build()
        .unwrap();
    let mut g = GlGraphics::new(OpenGL::V3_2);

    let mut size = s.initial_size; // changes if window is resized

    let mut event_loop: Events = window.events;
    while let Some(event) = event_loop.next(&mut window) {
        let f = functions.get();
        match event {
            Event::Loop(Loop::Render(render_args)) => {
                let render_args: RenderArgs = render_args;
                // An optimization introduced in opengl_graphics 0.39.1 causes
                // severe glitching if not wrapped in `gfx.draw()`.
                // (just calling it at the end with an empty closure
                //  seems to work too, for now...)
                g.draw(render_args.viewport(), |context, g| {
                    let context: Context = context;
                    let g: &mut GlGraphics = g; // the same instance as outside
                    size = context.get_view_size();
                    let context = context.scale(size[0], size[1]);

                    // by default alpha blending is disabled, which means all
                    // semi-transparent colors are considered opaque.
                    // Blend::Alpha blends colors pixel for pixel,
                    // which has a performance cost.
                    // The alternative would be to check for an existing color
                    // in the tile, and blend manually or even statically.
                    context.draw_state.blend(Blend::Alpha);
                    piston_window::clear(color::BLACK, g);
                    let wrapper = &mut GlWrap(g);
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
