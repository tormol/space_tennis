use interface::game::*;

use std::collections::HashMap;
use std::rc::Rc;
#[cfg(not(target_arch = "wasm32"))]
use std::thread;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

extern crate speedy2d;
use speedy2d::Graphics2D;
use speedy2d::color::Color as spColor;
use speedy2d::dimen::Vector2;
use speedy2d::font::{Font, TextLayout, TextOptions, FormattedTextBlock};
use speedy2d::shape::Rectangle;
use speedy2d::time::Stopwatch;
use speedy2d::window::{
    MouseButton as spMouseButton,
    VirtualKeyCode,
    WindowHandler,
    WindowHelper,
};
#[cfg(target_arch="wasm32")]
use speedy2d::WebCanvas;
#[cfg(not(target_arch = "wasm32"))]
use speedy2d::Window;
#[cfg(not(target_arch="wasm32"))]
use speedy2d::window::{WindowCreationOptions, WindowSize};

#[cfg(not(target_arch="wasm32"))]
extern crate image;
#[cfg(not(target_arch="wasm32"))]
use image::{ImageFormat, GenericImageView};

extern crate fxhash;
use fxhash::FxBuildHasher;

#[cfg(not(target_arch="wasm32"))]
const UPDATE_RATE: u32 = 125; // the standard USB polling rate.
#[cfg(not(target_arch="wasm32"))]
const ICON: &[u8] = include_bytes!("../../wasm/favicon.ico");

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

struct TextCache {
    font: Font,
    statics: HashMap<(&'static str, i32), Rc<FormattedTextBlock>, FxBuildHasher>,
}

impl TextCache {
    fn new() -> Self {
        TextCache {
            font: Font::new(include_bytes!("../../font/font.ttf")).expect("Parsing font"),
            statics: HashMap::default(),
        }
    }
    fn create(&self,  text: &str,  scaled_size: f32) -> Rc<FormattedTextBlock> {
        self.font.layout_text(text, scaled_size, TextOptions::new())
    }
    fn get_static(&mut self,  text: &'static str,  scaled_size: f32) -> Rc<FormattedTextBlock> {
        let key = (text, scaled_size as i32);
        self.statics.entry(key).or_insert_with(|| {
            self.font.layout_text(text, scaled_size, TextOptions::new())
        }).clone()
    }
}

struct GameWrapper<G: Game> {
    game: G,
    window_size: [f32; 2], // changes if window is resized
    stopwatch: Stopwatch,
    last_physics: f64,
    shapes: Graphics,
    text: TextCache,
}

impl<G: Game> WindowHandler for GameWrapper<G> {
    fn on_start(&mut self,
            h: &mut WindowHelper<()>,
            info: speedy2d::window::WindowStartupInfo
    ) {
        let size = info.viewport_size_pixels().into_f32();
        self.window_size = [size.x, size.y];
        h.set_cursor_visible(true);
        h.set_cursor_grab(false).unwrap();

        // icon is not used in wasm, and threads don't work there.
        #[cfg(not(target_arch="wasm32"))]
        {
            let icon = image::load_from_memory_with_format(ICON, ImageFormat::Ico)
                .expect("parse icon");
            let size = Vector2::new(icon.width(), icon.height());
            let icon = icon.as_rgba8().expect("get rgba");
            h.set_icon_from_rgba_pixels(icon.as_raw().clone(), size).expect("set icon");

            let sender = h.create_user_event_sender();
            thread::spawn(move || {
                loop {
                    sender.send_event(()).unwrap();
                    thread::sleep(Duration::from_secs_f32((UPDATE_RATE as f32).recip()));
                }
            });
        }
    }

    fn on_user_event(&mut self,  _: &mut WindowHelper<()>,  _: ()) {
        let prev = self.last_physics;
        self.last_physics = self.stopwatch.secs_elapsed();
        let elapsed = self.last_physics - prev;
        self.game.update(elapsed as f32);
    }

    fn on_draw(&mut self,  h: &mut WindowHelper<()>,  g: &mut Graphics2D) {
        #[cfg(target_arch="wasm32")]
        self.on_user_event(h, ());

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
                Shape::StaticText{ color, size, position, center, text } => {
                    let text = self.text.get_static(text, size * scale);
                    let mut position = Vector2 { x: position[0], y: position[1] } * scale + offset;
                    position.x = match center[0] {
                        Align::Left => position.x,
                        Align::Center => position.x - text.width()/2.0,
                        Align::Right => position.x - text.width(),
                    };
                    position.y = match center[1] {
                        Align::Left => position.y,
                        Align::Center => position.y - text.height()/2.0,
                        Align::Right => position.y - text.height(),
                    };
                    let color = map_color(color);
                    g.draw_text(position, color, &text);
                }
                Shape::DynamicText{ color, size, position, center, text } => {
                    let text = self.text.create(&text, size * scale);
                    let mut position = Vector2 { x: position[0], y: position[1] } * scale + offset;
                    position.x = match center[0] {
                        Align::Left => position.x,
                        Align::Center => position.x - text.width()/2.0,
                        Align::Right => position.x - text.width(),
                    };
                    position.y = match center[1] {
                        Align::Left => position.y,
                        Align::Center => position.y - text.height()/2.0,
                        Align::Right => position.y - text.height(),
                    };
                    let color = map_color(color);
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
    let wrapper = GameWrapper {
        game,
        window_size: initial_size,
        stopwatch: Stopwatch::new().expect("create stopwatch"),
        last_physics: 0.0,
        shapes: Graphics::default(),
        text: TextCache::new(),
    };

    #[cfg(target_arch="wasm32")]
    {
        let _ = name;
        WebCanvas::new_for_id("space_tennis_game", wrapper)
            .expect("bind to canvas");
        // .unregister_when_dropped() would make the game end immediately.
    }
    #[cfg(not(target_arch="wasm32"))]
    {
        let window_size = Vector2 { x: initial_size[0], y: initial_size[1] };
        let window_size = WindowSize::ScaledPixels(window_size);
        let options = WindowCreationOptions::new_windowed(window_size, None)
                .with_always_on_top(false)
                .with_decorations(true)
                .with_resizable(true)
                .with_transparent(false)
                .with_vsync(true);
        let window = Window::new_with_options(name, options).unwrap();
        window.run_loop(wrapper);
    }
}
