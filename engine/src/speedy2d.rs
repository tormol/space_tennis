use interface::game::*;

use std::thread;
use std::time::{Duration, Instant};

extern crate speedy2d;
use speedy2d::{Graphics2D, Window};
use speedy2d::color::Color as spColor;
use speedy2d::dimen::Vector2;
use speedy2d::font::{Font, TextLayout, TextOptions};
use speedy2d::shape::Rectangle;
use speedy2d::window::{
    MouseButton as spMouseButton,
    VirtualKeyCode,
    WindowCreationOptions,
    WindowHandler,
    WindowHelper,
    WindowSize,
};

const UPDATE_RATE: u32 = 125; // the standard USB polling rate.

fn map_key(key: VirtualKeyCode) -> Option<Key> {
    match key {
        VirtualKeyCode::Up => Some(Key::ArrowUp),
        VirtualKeyCode::Down => Some(Key::ArrowDown),
        VirtualKeyCode::Left => Some(Key::ArrowLeft),
        VirtualKeyCode::Right => Some(Key::ArrowRight),
        VirtualKeyCode::Escape => Some(Key::Escape),
        VirtualKeyCode::Return => Some(Key::Enter),
        VirtualKeyCode::Space => Some(Key::Space),
        _ => None
    }
}

fn map_button(b: spMouseButton) -> Option<MouseButton> {
    match b {
        spMouseButton::Left => Some(MouseButton::Left),
        spMouseButton::Right => Some(MouseButton::Right),
        spMouseButton::Middle => Some(MouseButton::Middle),
        spMouseButton::Other(_) => None
    }
}

fn map_color([r, g, b, a]: [f32; 4]) -> spColor {
    spColor::from_rgba(r, g, b, a)
}

/// Creates square display area.
fn letterbox_and_scale(window_size: [f32; 2]) -> (Vector2<f32>, f32) {
    let scale = f32::min(window_size[0], window_size[1]);
    let offset = Vector2 {
        x: (window_size[0] - scale) / 2.0,
        y: (window_size[1] - scale) / 2.0,
    };
    (offset, scale)
}

struct GameWrapper<G: Game> {
    game: G,
    window_size: [f32; 2], // changes if window is resized
    last_physics: Instant,
    shapes: Graphics,
    font: Font,
}

impl<G: Game> WindowHandler for GameWrapper<G> {
    fn on_start(&mut self,
            h: &mut WindowHelper<()>,
            _: speedy2d::window::WindowStartupInfo
    ) {
        h.set_cursor_visible(true);
        h.set_cursor_grab(false).unwrap();
        let sender = h.create_user_event_sender();
        thread::spawn(move || {
            loop {
                sender.send_event(()).unwrap();
                thread::sleep(Duration::from_secs_f32((UPDATE_RATE as f32).recip()));
            }
        });
    }

    fn on_user_event(&mut self,  _: &mut WindowHelper<()>,  _: ()) {
        let prev = self.last_physics;
        self.last_physics = Instant::now();
        let elapsed = self.last_physics.saturating_duration_since(prev);
        self.game.update(elapsed.as_secs_f32());
    }

    fn on_draw(&mut self,  h: &mut WindowHelper<()>,  g: &mut Graphics2D) {
        g.clear_screen(spColor::BLACK);
        self.game.render(&mut self.shapes);

        let (offset, scale) = letterbox_and_scale(self.window_size);
        for shape in self.shapes.drain() {
            match shape {
                Shape::Line { color, width, area } => {
                    let start = Vector2::new(area[0],  area[1])*scale+offset;
                    let end = Vector2::new(area[2],  area[3])*scale+offset;
                    let thickness = width * scale * 2.0;
                    let color = map_color(color);
                    g.draw_line(start, end, thickness, color)
                }
                Shape::Rectangle { color, area } => {
                    let rect = Rectangle::new(
                        Vector2 { x: area[0],  y: area[1] }*scale + offset,
                        Vector2 { x: area[0]+area[2],  y: area[1]+area[3] }*scale + offset,
                    );
                    g.draw_rectangle(rect, map_color(color));
                }
                Shape::Circle{ color, center,  radius } => {
                    let center = Vector2 {x: center[0],  y: center[1]}*scale + offset;
                    let radius = radius * scale;
                    let color = map_color(color);
                    g.draw_circle(center, radius, color);
                }
                Shape::StaticText{ color, size, top_left, text } => {
                    let color = map_color(color);
                    let position = Vector2 { x: top_left[0], y: top_left[1] } * scale + offset;
                    let scale = size * scale;
                    let text = self.font.layout_text(text, scale, TextOptions::new());
                    g.draw_text(position, color, &text);
                }
                Shape::DynamicText{ color, size, top_left, text } => {
                    let color = map_color(color);
                    let position = Vector2 { x: top_left[0], y: top_left[1] } * scale + offset;
                    let scale = size * scale;
                    let text = self.font.layout_text(&text, scale, TextOptions::new());
                    g.draw_text(position, color, &text);
                }
            }
        }

        // Required to make the screen update.
        // Surprisingly doesn't cause 100% CPU usage.
        h.request_redraw();
    }

    fn on_resize(&mut self,  _: &mut WindowHelper<()>,  size: speedy2d::dimen::UVec2) {
        self.window_size[0] = size.into_f32().x;
        self.window_size[1] = size.into_f32().y;
    }

    fn on_mouse_move(&mut self,  _: &mut WindowHelper<()>,  pos: Vector2<f32>) {
        let (offset, scale) = letterbox_and_scale(self.window_size);
        let pos = (pos - offset) / scale;
        self.game.mouse_move([pos.x, pos.y]);
    }

    fn on_mouse_button_down(&mut self,  _: &mut WindowHelper<()>,  button: spMouseButton) {
        if let Some(button) = map_button(button) {
            self.game.mouse_press(button);
        }
    }

    fn on_key_down(
            &mut self,
            _: &mut WindowHelper<()>,
            key: Option<VirtualKeyCode>,
            _: speedy2d::window::KeyScancode
    ) {
        if let Some(key) = key.and_then(map_key) {
            self.game.key_press(key);
        }
    }

    fn on_key_up(
            &mut self,
            _: &mut WindowHelper<()>,
            key: Option<VirtualKeyCode>,
            _: speedy2d::window::KeyScancode
    ) {
        if let Some(key) = key.and_then(map_key) {
            self.game.key_release(key);
        }
    }

    // TODO pause when window loses focus (!= mouse leaves)
}

#[inline(never)]
pub fn start<G:Game+'static>(game: G,  name: &'static str,  initial_size: [f32; 2]) {
    let window_size = Vector2 { x: initial_size[0], y: initial_size[1] };
    let window_size = WindowSize::ScaledPixels(window_size);
    let options = WindowCreationOptions::new_windowed(window_size, None)
            .with_always_on_top(false)
            .with_decorations(true)
            .with_resizable(true)
            .with_transparent(false)
            .with_vsync(true);
    let window = Window::new_with_options(name, options).unwrap();

    let wrapper = GameWrapper {
        game,
        window_size: initial_size,
        last_physics: Instant::now(),
        shapes: Graphics::default(),
        font: Font::new(include_bytes!("../../font/font.ttf")).expect("Parsing font"),
    };
    window.run_loop(wrapper);
}
