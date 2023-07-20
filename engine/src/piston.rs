use interface::game::*;

extern crate opengl_graphics;
use self::opengl_graphics::{OpenGL, GlGraphics};

extern crate piston_window;
use self::piston_window::{Event,Loop,RenderArgs,UpdateArgs,Input}; // from piston_input
use self::piston_window::{ButtonArgs,ButtonState,Button,Motion}; // from piston_input
use self::piston_window::Key as pwKey; // from piston_input
use self::piston_window::MouseButton as pwMouseButton; // from piston_input
use self::piston_window::{Context,Transformed,color}; // from piston2d-graphics
use self::piston_window::draw_state::Blend; // from piston2d-graphics
use self::piston_window::PistonWindow;
use self::piston_window::WindowSettings; // from piston::window
use self::piston_window::Events; // from piston::event_loop

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

fn map_key(key: pwKey) -> Option<Key> {
    match key {
        pwKey::Up => Some(Key::ArrowUp),
        pwKey::Down => Some(Key::ArrowDown),
        pwKey::Left => Some(Key::ArrowLeft),
        pwKey::Right => Some(Key::ArrowRight),
        pwKey::Escape => Some(Key::Escape),
        pwKey::Return => Some(Key::Enter),
        pwKey::Space => Some(Key::Space),
        _ => None
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
pub fn start<G:Game>(game: &mut G,  name: &'static str,  initial_size: [f64; 2]) {
    let window_size = [initial_size[0] as u32, initial_size[1] as u32];
    let mut window: PistonWindow = WindowSettings::new(name, window_size)
        .vsync(true)
        .build()
        .unwrap();
    let mut g = GlGraphics::new(OpenGL::V3_2);

    let mut size = initial_size; // changes if window is resized

    let mut event_loop: Events = window.events;
    while let Some(event) = event_loop.next(&mut window) {
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
                    game.render(context.transform, wrapper);
                });
            }
            Event::Loop(Loop::Update(update_args)) => {
                let UpdateArgs{dt: deltatime} = update_args;
                game.update(deltatime);
            }

            Event::Input(Input::Button(ButtonArgs {
                    state: ButtonState::Press,
                    button: Button::Keyboard(key),
                    ..
            }), _) => {
                if let Some(key) = map_key(key) {
                    game.key_press(key);
                }
            }
            Event::Input(Input::Button(ButtonArgs {
                    state: ButtonState::Release,
                    button: Button::Keyboard(key),
                    ..
            }), _) => {
                if let Some(key) = map_key(key) {
                    game.key_release(key);
                }
            }

            Event::Input(Input::Button(ButtonArgs {
                    state: ButtonState::Press,
                    button: Button::Mouse(button),
                    ..
            }), _) => {
                game.mouse_press(map_button(button));
            }
            Event::Input(Input::Move(Motion::MouseCursor([x,y])), _) => {
                game.mouse_move([x/size[0], y/size[1]]);
            }
            // TODO pause when window loses focus (!= mouse leaves)

            _ => {}
        }
    }
}
