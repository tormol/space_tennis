//! types used by both hidden.rs and lib.rs

use std::path::Path;
use std::borrow::Cow;
use std::os::raw::c_void;

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
    pub render: unsafe fn(*mut c_void,  Matrix2d,  &mut dyn Graphics),
    pub update: unsafe fn(*mut c_void,  f64),
    pub mouse_move: unsafe fn(*mut c_void,  [f64; 2]),
    pub mouse_press: unsafe fn(*mut c_void,  MouseButton),
    pub size: usize
}

pub struct StartUpInfo {
    pub name: Cow<'static,str>,
    pub src: Cow<'static,Path>,
    pub initial_size: [f64; 2],
    pub game: *mut c_void,
}
