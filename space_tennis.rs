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
extern crate piston_window;
use piston_window::{Context,DrawState,Transformed,color,math}; // from piston2d-graphics
use piston_window::types::Color; // from piston2d-graphics

use std::f64::consts::PI;
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
const AREA: [f64;3] = [1.0, 1.0, 2.0]; // 
const BALL_RADIUS: f64 = 0.1;
const RACKET_SIZE: [f64; 2] = [0.9, 0.6];
const FOV: f64 = PI/2.0; // 90°
const VIEW_DISTANCE: f64 = 0.8; // distance to arena
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
        player_pos: [0.0, 0.0, AREA[2]/2.0],
        opponent_pos: [0.0, 0.0, -AREA[2]/2.0],
        ball_vel: [0.0, 0.0, 0.1],
        ball_pos: [0.0, 0.0, 0.0],
        paused: true
    } }

    fn render(&mut self,  draw_state: DrawState,  transform: math::Matrix2d,  gfx: &mut GlGraphics) {
        /*
        at the center of the window there is a view cone with a certain angle (field of view)
        at x dept the distance from top to bottom or left to right of view
        is 2*x*tan(fov/2). (hosliggende er 1, ikke hypotenusen)
        for now the aspect ratio is assumed to be 1:1, so horizontal and vertical FOV is equal.
        */ 
        // step 1: draw a rectangle that fills the view of the arena
        // i.e: render a centered cube at distance VIEW_DISTANCE with width and height 1
        // adjust width and height to fraction of view
        // view at VIEW_DISTANCE
        let front_viewable = 2.0*VIEW_DISTANCE*f64::tan(FOV/2.0);
        let front_frac = AREA[0] / front_viewable;
        let front_offset_min = 0.5 - (front_frac/2.0);
        let front_size = front_frac;
        let front_area = [front_offset_min, front_offset_min, front_size, front_size];
        piston_window::rectangle(color::hex("aaaaaa"), front_area, transform, gfx);
        if self.paused {
            println!("VIEW_DISTANCE: {}, FOV: {}", VIEW_DISTANCE, FOV/PI*180.0);
            println!("front_viewable: {}, AREA: {:?}", front_viewable, AREA);
            println!("front_frac: {}, min: {}", front_frac, front_offset_min);
            self.paused = false;
        }

        // step 2: draw a rectangle that fills the back of the arena
        // i.e: render a centered cube at distance VIEW_DISTANCE with width and height 1
        // adjust width and height to fraction of view
        // view at VIEW_DISTANCE
        let back_viewable = 2.0*(VIEW_DISTANCE+AREA[2])*f64::tan(FOV/2.0);
        let back_frac = AREA[0] / back_viewable;
        let back_offset_min = 0.5 - (back_frac/2.0);
        let back_size = back_frac;
        let back_area = [back_offset_min, back_offset_min, back_size, back_size];
        piston_window::rectangle(color::hex("000000"), back_area, transform, gfx);

        
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
    let window_size = [INITIAL_SIZE[0] as u32, INITIAL_SIZE[1] as u32];
    let mut window: PistonWindow = WindowSettings::new("space tennis", window_size)
        .exit_on_esc(true)
        .vsync(true)
        .opengl(OpenGL::V3_2)
        .build()
        .unwrap();
    let mut gfx = GlGraphics::new(OpenGL::V3_2);

    let mut size = INITIAL_SIZE;//changes if window is resized
    let mut offset = [0.0; 2];//letterboxing after resize

    let mut game = Game::new();
    let mut event_loop: Events = window.events;
    let mut changed = true;
    while let Some(e) = event_loop.next(&mut window) {
        match e {
            Input::Render(render_args/*: RenderArgs*/) => {
                let render_args: RenderArgs = render_args;
                // An optimization introduced in opengl_graphics 0.39.1 causes
                // severe glitching if not wrapped in .draw.
                // (calling it afterwards with an empty closure seems to work too)
                gfx.draw(render_args.viewport(), |context, gfx| {
                    let context: Context = context;
                    let gfx: &mut GlGraphics = gfx; // the same instance as outside
                    if changed {
                        let v = render_args.viewport();
                        println!("{:?} {:?} {:?}", v.rect, v.draw_size, v.window_size);
                        changed = false;
                    }

                    let context = context.scale(context.get_view_size()[0], context.get_view_size()[1]);
                    // let vds = render_args.viewport().draw_size;
                    // println!("gvs: {:?}, view: {:?}, transform {:?}", context.get_view_size(), context.view, context.transform);

                    //by default alpha blending is disabled, which means all semi-transparent colors are considered opaque.
                    //since colors are blended pixel for pixel, this has a performance cost,
                    //the alternative is to check for existing color in tile, and blend manually, or even statically
                    context.draw_state.blend(Blend::Alpha);
                    piston_window::clear(color::BLACK, gfx);

                    game.render(context.draw_state, context.transform, gfx);
                });
            }
            Input::Update(update_args) => {
                game.update(update_args.dt);// deltatime is its only field
            }

            Input::Resize(x,y) => {
                changed = true;
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
