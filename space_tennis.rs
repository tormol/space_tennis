/* Copyright 2017 Torbjørn Birch Moltu
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see <http://www.gnu.org/licenses/>.
 */


use std::ops::Neg;
extern crate num;
use num::{Zero,One,ToPrimitive};
extern crate vecmath;
use vecmath::vec2_add; // Vector2 is [T; 2]
// Why not +?
// Any math library in rust have to make a choice: Either use primitive slices or tuples
// which make constructing and destructuring pain-free, or use std::ops::*.
// Rusts coherence rules prevents them for doing both:
// You cannot implement an external trait for a foreign type.
extern crate piston_window;
use piston_window::{Context,DrawState,Transformed,color,math}; // from piston2d-graphics
use piston_window::types::Color; // from piston2d-graphics

use std::cmp;
use std::collections::vec_deque::VecDeque;
 // from piston::input:
use piston_window::keyboard::Key;
use piston_window::mouse::MouseButton;
extern crate opengl_graphics;
use opengl_graphics::GlGraphics;
use opengl_graphics::glyph_cache::GlyphCache;
extern crate rand;
use rand::Rng;

const INITIAL_SIZE: [f64;2] = [500.0, 500.0];
const UPDATE_TIME: f64 = 1.0/60.0;
const AREA_DEPTH: f64 = 2.0;
const BALL_RADIUS: f64 = 0.1;
const RACKET_SIZE: [f64; 2] = [0.9, 0.6];
//const FONT_PATH: &'static str = "/usr/share/fonts/truetype/msttcorefonts/arial.ttf";
//const FONT_RESOLUTION: f64 = 100.0;

struct Game {
    ball_pos: [f64; 3],
    ball_vel: [f64; 3],
    player_pos: [f64; 3],
    opponent_pos: [f64; 3],
    paused: bool
}
impl Game {
    fn new() -> Self {Game {
        player_pos: [0.0, 0.0, AREA_DEPTH],
        opponent_pos: [0.0, 0.0, -AREA_DEPTH],
        ball_vel: [0.0, 0.0, 0.0],
        ball_pos: [0.0, 0.0, AREA_DEPTH-BALL_RADIUS],
        paused: true
    } }

    fn render(&mut self,  draw_state: DrawState,  transform: math::Matrix2d,  gfx: &mut GlGraphics) {

    }

    fn update(&mut self, dt: f64) {

    }

    fn mouse_move(&mut self,  pos: Option<[f64; 2]>) {

    }

    fn mouse_press(&mut self,  button: MouseButton) {

    }
    fn mouse_release(&mut self,  button: MouseButton) {

    }

    fn key_press(&mut self,  key: Key) {
        if key == Key::P {

        }
    }
}

use opengl_graphics::OpenGL;
use piston_window::{Input,Button,Motion,RenderArgs,UpdateArgs}; // from piston::input
use piston_window::draw_state::Blend; // from piston2d-graphics
use piston_window::WindowSettings; // from piston::window
use piston_window::Events; // from piston::event_loop
use piston_window::PistonWindow; // from piston_window

use std::time::Instant;

fn main() {
    let mut window: PistonWindow = WindowSettings::new(
            "space tennis", [INITIAL_SIZE[0] as u32, INITIAL_SIZE[1] as u32]
        ).exit_on_esc(true)
        .opengl(OpenGL::V3_2)
        .build()
        .unwrap();
    let mut gfx = GlGraphics::new(OpenGL::V3_2);

    let mut size = INITIAL_SIZE;//changes if window is resized
    let mut offset = [0.0; 2];//letterboxing after resize

    let mut game = Game::new();
    let mut event_loop: Events = window.events;
    while let Some(e) = event_loop.next(&mut window) {
        match e {
            Input::Render(render_args/*: RenderArgs*/) => {
                let context: Context = Context::new_viewport(render_args.viewport())
                                               .trans(offset[0], offset[1])
                                               .scale(size[0], size[1]);
                //by default alpha blending is disabled, which means all semi-transparent colors are considered opaque.
                //since colors are blended pixel for pixel, this has a performance cost,
                //the alternative is to check for existing color in tile, and blend manually, or even statically
                context.draw_state.blend(Blend::Alpha);
                piston_window::clear(color::BLACK, &mut gfx);

                game.render(context.draw_state, context.transform, &mut gfx);
            }
            Input::Update(update_args) => {
                game.update(update_args.dt);// deltatime is its only field
            }

            Input::Resize(x,y) => {
                let min = f64::min(x as f64 / INITIAL_SIZE[0],
                                   y as f64 / INITIAL_SIZE[1]);
                size = [INITIAL_SIZE[0]*min, INITIAL_SIZE[1]*min];
                offset = [(x as f64 - size[0]) / 2.0,
                          (y as f64 - size[1]) / 2.0];
                gfx.viewport(0, 0, x as i32, y as i32);
            }

            Input::Press(Button::Keyboard(key)) => {
                game.key_press(key);
            }
            Input::Move(Motion::MouseCursor(x,y)) => {
                let mut pos: Option<[f64; 2]> = None;
                let x = (x - offset[0]) / size[0];
                let y = (y - offset[1]) / size[1];
                if x >= 0.0  &&  x < 1.0
                && y >= 0.0  &&  y < 1.0 {
                    pos = Some([x*INITIAL_SIZE[0], y*INITIAL_SIZE[1]]);
                }
                game.mouse_move(pos);
            }
            Input::Cursor(_) => {//only happens if a button is pressed
                game.mouse_move(None);
            }
            _ => {}
        }
    }
}
