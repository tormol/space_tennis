//! types used by both hidden.rs and lib.rs

use std::path::Path;
use std::borrow::Cow;

pub type Color = [f32;4];//piston_window::types::Color;
pub type Matrix2d = [[f64;3];2];//
pub trait Graphics {
    fn line(&mut self, Color, f64, [f64;4], Matrix2d);
    fn rectangle(&mut self, Color, [f64;4], Matrix2d);
    fn ellipse(&mut self, Color, [f64;4], Matrix2d);
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
    fn render(&mut self,  Matrix2d,  &mut dyn Graphics);
    fn update(&mut self,  f64);
    fn mouse_move(&mut self,  [f64; 2]);
    fn mouse_press(&mut self,  MouseButton);
}

pub struct Functions {
    pub render: unsafe fn(*mut u8,  Matrix2d,  &mut dyn Graphics),
    pub update: unsafe fn(*mut u8,  f64),
    pub mouse_move: unsafe fn(*mut u8,  [f64; 2]),
    pub mouse_press: unsafe fn(*mut u8,  MouseButton),
}

pub struct StartUpInfo {
    pub name: Cow<'static,str>,
    pub src: Cow<'static,Path>,
    pub initial_size: [f64; 2],
    pub game: *mut u8,
}
