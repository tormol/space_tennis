extern crate opengl_graphics;
pub use opengl_graphics::{OpenGL, GlGraphics};

extern crate piston_window;
pub use piston_window::{Context,DrawState,Transformed,color,math}; // from piston2d-graphics
pub use piston_window::mouse::MouseButton; // from piston::input
pub use piston_window::{Input,Button,Motion,RenderArgs,UpdateArgs}; // from piston::input
pub use piston_window::draw_state::Blend; // from piston2d-graphics
pub use piston_window::WindowSettings; // from piston::window
pub use piston_window::Events; // from piston::event_loop

extern crate glutin_window;
pub type PistonWindow = piston_window::PistonWindow<glutin_window::GlutinWindow>; // from piston_window

type Color = [f32;4];
type Rect = [f64;4];
type Matrix2d = [[f64;3];2];
pub fn clear(color: Color,  g: &mut GlGraphics) {
    piston_window::clear(color, g);
}
pub fn rectangle(color: Color,  rect: Rect,  transform: Matrix2d,  g: &mut GlGraphics) {
    piston_window::rectangle(color, rect, transform, g)
}
pub fn line(color: Color,  radius: f64,  rect: Rect,  transform: Matrix2d,  g: &mut GlGraphics) {
    piston_window::line(color, radius, rect, transform, g)
}
pub fn ellipse(color: Color,  rect: Rect,  transform: Matrix2d,  g: &mut GlGraphics) {
    piston_window::ellipse(color, rect, transform, g)
}
