pub type Color = [f32;4];//piston_window::types::Color;
pub type Matrix2d = [[f64;3];2];//
pub trait Graphics {
    fn line(&mut self,  color: Color,  width: f64,  area: [f64;4],  transform: Matrix2d);
    fn rectangle(&mut self,  color: Color,  area: [f64;4],  transform: Matrix2d);
    fn ellipse(&mut self,  color: Color,  area: [f64;4],  transform: Matrix2d);
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

pub trait Game {
    fn render(&mut self,  transform: Matrix2d,  gfx: &mut dyn Graphics);
    fn update(&mut self,  dt: f64);
    fn key_press(&mut self,  key: Key);
    fn key_release(&mut self,  key: Key);
    fn mouse_move(&mut self,  pos: [f64; 2]);
    fn mouse_press(&mut self,  button: MouseButton);
}
