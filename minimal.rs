#![cfg_attr(windows, windows_subsystem = "windows")]
//#![no_std] // but engine uses std

extern crate engine;
use engine::*;

const INITIAL_SIZE: [f64;2] = [500.0, 500.0];
const BALL_COLOR: &str = "aaff55ee";

struct Minimal {
    mouse: [f64; 2],
}

impl Game for Minimal {
    fn render(&mut self,  transform: [[f64;3];2],  gfx: &mut Graphics) {
        let ball_color = hex(BALL_COLOR);
        let [x,y] = self.mouse;
        let ball_rect = [x-0.1, y-0.125, 0.2, 0.25];
        gfx.ellipse(ball_color, ball_rect, transform);
    }
    fn update(&mut self,  _: f64) {}
    fn mouse_move(&mut self,  pos: [f64; 2]) {
        self.mouse = pos;
    }
    fn mouse_press(&mut self,  _: MouseButton) {}
}

expose_game!{Minimal}
fn main() {
    start(&mut Minimal{mouse:[0.5;2]}, "minimal", INITIAL_SIZE, &[][..]);
}
