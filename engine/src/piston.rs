use interface::game::*;
use piston_window::EventLoop;

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

fn map_button(b: pwMouseButton) -> Option<MouseButton> {
    match b {
        pwMouseButton::Left => Some(MouseButton::Left),
        pwMouseButton::Right => Some(MouseButton::Right),
        pwMouseButton::Middle => Some(MouseButton::Middle),
        _ => None
    }
}

#[inline(never)]
pub fn start<G:Game>(mut game: G,  name: &'static str,  initial_size: [f32; 2]) {
    let window_size = [initial_size[0] as u32, initial_size[1] as u32];
    let mut window: PistonWindow = WindowSettings::new(name, window_size)
        .vsync(true)
        .build()
        .unwrap();
    let mut g = GlGraphics::new(OpenGL::V3_2);

    let mut shapes = Graphics::default();
    // changes if window is resized
    let mut size = [initial_size[0] as f64, initial_size[1] as f64];
    let mut offset = [0.0, 0.0];

    let mut event_loop: Events = window.events;
    event_loop.set_ups(125); // default USB polling rate
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
                    let scale = f64::min(size[0], size[1]);
                    offset = [
                        (size[0]-scale) / 2.0,
                        (size[1]-scale) / 2.0,
                    ];
                    // Handle resized windows by scaling without stretching,
                    // and adding letterboxing to center.
                    let context = context.trans(offset[0], offset[1])
                                         .scale(scale, scale);

                    // by default alpha blending is disabled, which means all
                    // semi-transparent colors are considered opaque.
                    // Blend::Alpha blends colors pixel for pixel,
                    // which has a performance cost.
                    // The alternative would be to check for an existing color
                    // in the tile, and blend manually or even statically.
                    context.draw_state.blend(Blend::Alpha);

                    piston_window::clear(color::BLACK, g);

                    game.render(&mut shapes);

                    fn area_to_f64(area: [f32; 4]) -> [f64; 4] {
                        [area[0] as f64, area[1] as f64, area[2] as f64, area[3] as f64]
                    }
                    let transform = context.transform;
                    for shape in shapes.drain() {
                        match shape {
                            Shape::Line { color, width, area } => {
                                let area = area_to_f64(area);
                                piston_window::line(color, width as f64, area, transform, g);
                            }
                            Shape::Rectangle { color, area } => {
                                piston_window::rectangle(color, area_to_f64(area), transform, g);
                            }
                            Shape::Circle { color, center, radius } => {
                                let area = area_to_f64([
                                    center[0]-radius, center[1]-radius,
                                    radius*2.0, radius*2.0,
                                ]);
                                piston_window::ellipse(color, area, transform, g)
                            }
                        }
                    }
                });
            }
            Event::Loop(Loop::Update(update_args)) => {
                let UpdateArgs{dt: deltatime} = update_args;
                game.update(deltatime as f32);
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
                if let Some(button) = map_button(button) {
                    game.mouse_press(button);
                }
            }
            Event::Input(Input::Move(Motion::MouseCursor([x,y])), _) => {
                let scale = f64::min(size[0], size[1]);
                let x = (x-offset[0]) / scale;
                let y = (y-offset[1]) / scale;
                game.mouse_move([x as f32, y as f32]);
            }
            // TODO pause when window loses focus (!= mouse leaves)

            _ => {}
        }
    }
}
