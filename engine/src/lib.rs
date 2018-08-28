use std::mem::transmute;
extern crate engine_dylib;
pub use engine_dylib::{Graphics,MouseButton,Color,hex,Matrix2d};
use engine_dylib::Functions;

pub trait Game {
    fn render(&mut self,  Matrix2d,  &mut dyn Graphics);
    fn update(&mut self,  f64);
    fn mouse_move(&mut self,  [f64; 2]);
    fn mouse_press(&mut self,  MouseButton);
}

pub fn start<G:Game>(game: &mut G,  name: &'static str,  initial_size: [f64;2]) {
    unsafe {
        let g = game as *mut G as *mut u8;
        let f = Functions {
            render: transmute::<fn(&mut G,Matrix2d,&mut dyn Graphics),_>(G::render),
            update: transmute::<fn(&mut G,f64),_>(G::update),
            mouse_move: transmute::<fn(&mut G,[f64;2]),_>(G::mouse_move),
            mouse_press: transmute::<fn(&mut G,MouseButton),_>(G::mouse_press),
        };
        engine_dylib::start(name, initial_size, g, f);
    }
}
