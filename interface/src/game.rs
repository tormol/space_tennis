/// Matches `piston_window::types::Color`
pub type Color = [f32; 4];

/// An element to render.
#[derive(Clone,Copy, Debug)]
pub enum Shape {
    /// A line that doesn't need to be horizontal or vertical.
    /// `[area[0], area[1]]` is the top left end,
    /// and `[area[2], area[3]]` are the length of the line.
    Line{ color: Color,  width: f32,  area: [f32;4] },
    /// A rectangle aligned to the X/Y axes.
    /// `[area[0], area[1]]` is the top left corner
    /// and `[area[2], area[3]]` is the size.
    Rectangle{ color: Color,  area: [f32;4] },
    Circle{ color: Color,  center: [f32; 2],  radius: f32 },
}

/// A list of `Shape`s to render.
///
/// Games add elements to it in `Game.render()`,
/// and engines consume it with `drain()`
#[derive(Default, Debug)]
pub struct Graphics {
    commands: Vec<Shape>,
}

impl Graphics {
    pub fn add(&mut self,  shape: Shape) {
        self.commands.push(shape);
    }
    pub fn line(&mut self,  color: Color,  width: f32,  area: [f32;4]) {
        self.commands.push(Shape::Line{color, width, area});
    }
    pub fn rectangle(&mut self,  color: Color,  area: [f32;4]) {
        self.commands.push(Shape::Rectangle{ color, area });
    }
    pub fn circle(&mut self,  color: Color,  center: [f32; 2],  radius: f32) {
        self.commands.push(Shape::Circle{ color, center, radius });
    }
    /// Iterate over all elements and leave the list empty.
    pub fn drain<'a>(&'a mut self) -> impl Iterator<Item=Shape> + 'a {
        self.commands.drain(..)
    }
}

/// Parse a hex string of 6 or 8 bytes into a color.
/// Format is rrggbbaa, where the aa is optional.
#[track_caller]
pub fn hex(color: &str) -> Color {
    let a = match color.len() {
        8 => u8::from_str_radix(&color[6..], 16).unwrap(),
        6 => 255,
        _ => panic!("color string must be 6 or 8 characters")
    };
    let r = u8::from_str_radix(&color[..2], 16).unwrap();
    let g = u8::from_str_radix(&color[2..4], 16).unwrap();
    let b = u8::from_str_radix(&color[4..6], 16).unwrap();
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0]
}

/// Keys that the game cares about.
#[derive(Debug, Clone,Copy, PartialEq,Eq)]
pub enum Key {
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Enter,
    Escape,
    Space,
}

/// All mouse buttons piston supports.
#[derive(Debug, Clone,Copy, PartialEq,Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

pub trait Game {
    fn render(&mut self,  gfx: &mut Graphics);
    fn update(&mut self,  dt: f32);
    fn key_press(&mut self,  key: Key);
    fn key_release(&mut self,  key: Key);
    fn mouse_move(&mut self,  pos: [f32; 2]);
    fn mouse_press(&mut self,  button: MouseButton);
}
