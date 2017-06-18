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

use std::f64::consts::PI;
extern crate piston_window;
use piston_window::{Context,DrawState,Transformed,color,math}; // from piston2d-graphics
// from piston::input:
use piston_window::keyboard::Key;
use piston_window::mouse::MouseButton;
extern crate opengl_graphics;
use opengl_graphics::GlGraphics;

const INITIAL_SIZE: [f64;2] = [500.0, 500.0];
const UPDATE_TIME: f64 = 1.0/60.0;
const AREA: [f64;3] = [1.0, 1.0, 2.0]; // 
const BALL_RADIUS: f64 = 0.125; // exact representable
const MISS_BALL_RADIUS: f64 = 0.025;
const RACKET_SIZE: [f64;2] = [0.22, 0.15];
const RACKET_BORDER_WIDTH: [f64;2] = [0.01, 0.004];
const RACKET_MAX_SPEED: [f64;2] = [0.4, 0.4];
const BRACKET_SPEED_TRANSFER: f64 = 0.75; // based on mass of ball and bracket
const FOV: f64 = PI/3.0; // 60°
const FRONT_FILLS: f64 = 0.8; // of the screen
//const BACK_FILLS: f64 = 0.3; // of the screen
const WALL_LINES: u32 = 5;
const LINE_WIDTH: f64 = 0.05;
const LINE_WIDTH_EDGE: f64 = LINE_WIDTH*1.5;
const BALL_ZSPEED_LEVEL_ADD: f64 = 0.125;

const WALL_COLOR: &str = "444444";
const WALL_LINE_COLOR: &str = "008800";
const RACKET_COLOR: &str = "ddddddaa";
const RACKET_BORDER_COLOR: &str = "5555dd";
const BALL_COLOR: &str = "33ff33cc";
const BALL_LINE_COLOR: &str = "eeeeee88";
const MISS_COLOR: &str = "ff3333";


//const FONT_PATH: &'static str = "/usr/share/fonts/truetype/msttcorefonts/arial.ttf";
//const FONT_RESOLUTION: f64 = 100.0;

fn clamp(p: f64,  (min,max): (f64,f64)) -> f64 {
         if p <= min   {min}
    else if p >= max   {max}
    else if p.is_nan() {(min+max)/2.0}
    else               {p}
}


struct Game {
    ball_pos: [f64; 3],
    ball_vel: [f64; 3],
    player_pos: [f64; 2],
    player_target: [f64; 2],
    player_misses: u32,
    opponent_pos: [f64; 2],
    opponent_target: [f64; 2],
    opponent_misses: u32,
    paused: bool
}
impl Game {
    fn new() -> Self {Game {
        player_misses: 0,
        opponent_misses: 0,
        player_pos: [0.5, 0.5],
        player_target: [0.5, 0.5],
        opponent_pos: [0.5, 0.5],
        opponent_target: [0.5, 0.5],
        ball_vel: [0.0, 0.0, -0.4],
        ball_pos: [0.5, 0.5, 1.0],
        paused: true
    } }

    fn render(&mut self,  _: DrawState,  transform: math::Matrix2d,  gfx: &mut GlGraphics) {
        /*
        at the center of the window there is a view cone with a certain angle (field of view)
        at x dept the distance from top to bottom or left to right of view
        is 2*x*tan(fov/2). (hosliggende er 1, ikke hypotenusen)
        for now the aspect ratio is assumed to be 1:1, so horizontal and vertical FOV is equal.
        */
        // step 0: find view distance that satisfies FRONT_FILLS:
        // at x distance the viewable area*FRONT_FILLS is AREA[0] =>
        // 2*view_distance*tan(fov/2)*FRONT_FILLS=AREA[0]
        let view_distance = AREA[0]/(2.0*FRONT_FILLS*f64::tan(FOV/2.0));

        fn draw_wall_marker(
                color: &str,  depth: f64,  width: f64,
                transform: math::Matrix2d,  gfx: &mut GlGraphics
        ) {
            // width is on the wall, aka the z-dimension.
            // find the draw width by calculating the rectangle of the near and
            // far edge, and setting the center of the lines to the median.
            let color = color::hex(color);
            let near_viewable = 2.0*(depth-width/2.0)*f64::tan(FOV/2.0);
            let far_viewable = 2.0*(depth+width/2.0)*f64::tan(FOV/2.0);
            let near_area_frac = (AREA[0]/near_viewable, AREA[1]/near_viewable);
            let far_area_frac = (AREA[0]/far_viewable, AREA[1]/far_viewable);
            let near_topleft = (0.5 - (near_area_frac.0/2.0),  0.5 - (near_area_frac.1/2.0));
            let near_bottomright = (0.5 + (near_area_frac.0/2.0),  0.5 + (near_area_frac.1/2.0));
            let far_topleft = (0.5 - (far_area_frac.0/2.0),  0.5 - (far_area_frac.1/2.0));
            let radius = ((far_topleft.0-near_topleft.0)/2.0, (far_topleft.1-near_topleft.1)/2.0);
            let offset_x = (near_topleft.0+radius.0, near_bottomright.0-radius.0);
            let offset_y = (near_topleft.1+radius.1, near_bottomright.1-radius.1);
            // draw corners completely, and only once in case the color is translucent
            let top    = [offset_x.0-radius.0, offset_y.0, offset_x.1-radius.0, offset_y.0];
            let bottom = [offset_x.0+radius.0, offset_y.1, offset_x.1+radius.0, offset_y.1];
            let left   = [offset_x.0, offset_y.0+radius.1, offset_x.0, offset_y.1+radius.1];
            let right  = [offset_x.1, offset_y.0-radius.1, offset_x.1, offset_y.1-radius.1];
            piston_window::line(color, radius.1, top, transform, gfx);
            piston_window::line(color, radius.1, bottom, transform, gfx);
            piston_window::line(color, radius.0, left, transform, gfx);
            piston_window::line(color, radius.0, right, transform, gfx);
        }
        // draw the walls themselves
        draw_wall_marker(WALL_COLOR, view_distance+AREA[2]/2.0, AREA[2], transform, gfx);
        let interval = AREA[2]/(WALL_LINES+1) as f64;
        // the markers on the edges are thicker
        draw_wall_marker(WALL_LINE_COLOR, view_distance, LINE_WIDTH_EDGE, transform, gfx);
        for n in 1..(WALL_LINES+1) {
            draw_wall_marker(WALL_LINE_COLOR, view_distance + interval*n as f64, LINE_WIDTH, transform, gfx);
        }
        draw_wall_marker(WALL_LINE_COLOR, view_distance+AREA[2], LINE_WIDTH_EDGE, transform, gfx);

        fn draw_racket(
                pos: [f64;2]/*in arena*/, depth: f64/*from view*/,
                transform: math::Matrix2d,  gfx: &mut GlGraphics,
        ) {
            let fill_color = color::hex(RACKET_COLOR);
            let border_color = color::hex(RACKET_BORDER_COLOR);
            let viewable = 2.0*depth*f64::tan(FOV/2.0);
            let area_frac = [AREA[0]/viewable, AREA[1]/viewable];
            let border_frac = [RACKET_BORDER_WIDTH[0]/viewable, RACKET_BORDER_WIDTH[1]/viewable];
            let area_offset = [0.5-area_frac[0]/2.0, 0.5-area_frac[1]/2.0];
            let racket_frac = [RACKET_SIZE[0]/viewable, RACKET_SIZE[1]/viewable];
            let racket_pos = [area_offset[0]+area_frac[0]*pos[0], area_offset[1]+area_frac[1]*pos[1]];
            let fill_area = [
                racket_pos[0]-racket_frac[0]/2.0+border_frac[0],
                racket_pos[1]-racket_frac[1]/2.0+border_frac[1],
                racket_frac[0]-2.0*border_frac[0],
                racket_frac[1]-2.0*border_frac[1],
            ];
            piston_window::rectangle(fill_color, fill_area, transform, gfx);
            let radius = [border_frac[0]/2.0, border_frac[1]/2.0];
            let (left,top,right,bottom) = (
                racket_pos[0]-racket_frac[0]/2.0+radius[0],
                racket_pos[1]-racket_frac[1]/2.0+radius[1],
                racket_pos[0]+racket_frac[0]/2.0-radius[0],
                racket_pos[1]+racket_frac[1]/2.0-radius[1],
            );
            piston_window::line(border_color, radius[0], [left-radius[1], top, right-radius[1], top], transform, gfx);
            piston_window::line(border_color, radius[1], [right, top-radius[0], right, bottom-radius[0]], transform, gfx);
            piston_window::line(border_color, radius[0], [left+radius[1], bottom, right+radius[1], bottom], transform, gfx);
            piston_window::line(border_color, radius[1], [left, top+radius[0], left, bottom+radius[0]], transform, gfx);
        }
        // opponent racket
        draw_racket(self.opponent_pos, view_distance+AREA[2], transform, gfx);

        // step 5: ball
        draw_wall_marker(BALL_LINE_COLOR, view_distance+self.ball_pos[2], LINE_WIDTH, transform, gfx);
        let ball_color = color::hex(BALL_COLOR);
        let ball_viewable = 2.0*(view_distance+self.ball_pos[2])*f64::tan(FOV/2.0);
        let ball_depth_frac = (AREA[0]/ball_viewable, AREA[1]/ball_viewable);
        let ball_offset = (0.5 - ball_depth_frac.0/2.0,  0.5 - ball_depth_frac.1/2.0);
        let ball_pos = (ball_offset.0 + ball_depth_frac.0*self.ball_pos[0],  ball_offset.1 + ball_depth_frac.1*self.ball_pos[1]);
        let ball_frac = BALL_RADIUS*2.0 / ball_viewable;
        let ball_rect = [ball_pos.0-ball_frac/2.0, ball_pos.1-ball_frac/2.0, ball_frac, ball_frac];
        piston_window::ellipse(ball_color, ball_rect, transform, gfx);

        // player racket
        draw_racket(self.player_pos, view_distance, transform, gfx);

        // misses
        let miss_color = color::hex(MISS_COLOR);
        let front_viewable = 2.0*view_distance*f64::tan(FOV/2.0);
        let radius_frac = MISS_BALL_RADIUS/front_viewable;
        let n_offset = 3.0*radius_frac;
        let start_y = 0.5-(AREA[1]/front_viewable)/2.0;
        let player_x = 0.5 + (AREA[0]/front_viewable)/2.0 + 2.0*radius_frac;
        let opponent_x = 0.5 - (AREA[0]/front_viewable)/2.0 - 4.0*radius_frac;
        for n in 0..self.player_misses {
            let rect = [player_x, start_y+n_offset*n as f64, 2.0*radius_frac, 2.0*radius_frac];
            piston_window::ellipse(miss_color, rect, transform, gfx);
        }
        for n in 0..self.opponent_misses {
            let rect = [opponent_x, start_y+n_offset*n as f64, 2.0*radius_frac, 2.0*radius_frac];
            piston_window::ellipse(miss_color, rect, transform, gfx);
        }
    }

    fn opponent(&mut self) {
        // predict where ball will end up without walls, and do nothing if not within reach
        if self.ball_vel[2] <= 0.0 {// moving away
            //self.opponent_target = [AREA[0]/2.0, AREA[1]/2.0];
            return
        }
        let dist = AREA[2]-BALL_RADIUS-self.ball_pos[2];
        let time = dist / self.ball_vel[2];
        let moves = [self.ball_vel[0]*time, self.ball_vel[1]*time, dist];
        let ends = [self.ball_pos[0]+moves[0], self.ball_pos[1]+moves[1], AREA[2]-BALL_RADIUS];
        if ends[0] < BALL_RADIUS || ends[0] > AREA[0]-BALL_RADIUS
        || ends[1] < BALL_RADIUS || ends[1] > AREA[1]-BALL_RADIUS {
            //self.opponent_target = [AREA[0]/2.0, AREA[1]/2.0];
            return
        }
        let target_x = clamp(ends[0], (RACKET_SIZE[0]/2.0, AREA[0]-RACKET_SIZE[0]/2.0));
        let target_y = clamp(ends[1], (RACKET_SIZE[1]/2.0, AREA[1]-RACKET_SIZE[1]/2.0));
        self.opponent_target = [target_x, target_y];
    }

    fn update(&mut self, dt: f64) {
        if self.paused {
            return;
        }

        // move rackets: be kind to the players and do that first
        fn move_racket(racket: &mut[f64;2], target: &[f64;2], dt: f64) -> [f64;2] {
            let max_move = [RACKET_MAX_SPEED[0]*dt, RACKET_MAX_SPEED[1]*dt];
            let diff = [target[0]-racket[0], target[1]-racket[1]];
            let move_x = clamp(diff[0], (-max_move[0], max_move[0]));
            let move_y = clamp(diff[1], (-max_move[1], max_move[1]));
            *racket = [racket[0]+move_x, racket[1]+move_y];
            return [move_x/dt, move_y/dt];
        }
        let player_speed = move_racket(&mut self.player_pos, &self.player_target, dt);
        let opponent_speed = move_racket(&mut self.opponent_pos, &self.opponent_target, dt);

        // check boundaries and bounce / gameover
        let moved = [self.ball_vel[0]*dt, self.ball_vel[1]*dt, self.ball_vel[2]*dt];
        let mut pos = [self.ball_pos[0]+moved[0], self.ball_pos[1]+moved[1], self.ball_pos[2]+moved[2]];
        if pos[2] < 0.0 || pos[2] > AREA[2] {
            // game over, restart
            self.ball_pos = [0.5, 0.5, 1.0];
            let z_speed = self.ball_vel[2];
            self.ball_vel = [0.0, 0.0, z_speed+z_speed.signum()*BALL_ZSPEED_LEVEL_ADD];
            self.paused = true;
            if pos[2] < 0.0 {
                self.player_misses += 1;
            } else {
                self.opponent_misses += 1;
            }
            return;
        }
        if pos[0] < BALL_RADIUS {
            self.ball_vel[0] *= -1.0;
            pos[0] = BALL_RADIUS+(BALL_RADIUS-pos[0]);
        } else if pos[0] > AREA[0]-BALL_RADIUS {
            self.ball_vel[0] *= -1.0;
            pos[0] = (AREA[0]-BALL_RADIUS)-(pos[0]-(AREA[0]-BALL_RADIUS));
        }
        if pos[1] < BALL_RADIUS {
            self.ball_vel[1] *= -1.0;
            pos[1] = BALL_RADIUS+(BALL_RADIUS-pos[1]);
        } else if pos[1] > AREA[1]-BALL_RADIUS {
            // println!("wrong: {}", (pos[1]-(AREA[1]-BALL_RADIUS)));
            // println!("old: {:?}, {:?}", self.ball_vel, self.ball_pos);
            self.ball_vel[1] *= -1.0;
            pos[1] = (AREA[1]-BALL_RADIUS)-(pos[1]-(AREA[1]-BALL_RADIUS));
            // println!("new: {:?}, {:?}", self.ball_vel, self.ball_pos);
        }

        fn within(pos: [f64; 3], racket_center: [f64; 2]) -> bool {
            f64::abs(pos[0] - racket_center[0]) <= RACKET_SIZE[0] &&
            f64::abs(pos[1] - racket_center[1]) <= RACKET_SIZE[1]
        }
        if pos[2] < BALL_RADIUS && within(pos, self.player_pos) {
            self.ball_vel[0] += player_speed[0]*BRACKET_SPEED_TRANSFER;
            self.ball_vel[1] += player_speed[1]*BRACKET_SPEED_TRANSFER;
            self.ball_vel[2] *= -1.0;
            pos[2] = BALL_RADIUS-(pos[2]-BALL_RADIUS);
        } else if pos[2] > AREA[2]-BALL_RADIUS && within(pos, self.opponent_pos) {
            self.ball_vel[0] += opponent_speed[0]*BRACKET_SPEED_TRANSFER;
            self.ball_vel[1] += opponent_speed[1]*BRACKET_SPEED_TRANSFER;
            self.ball_vel[2] *= -1.0;
            pos[2] = (AREA[2]-BALL_RADIUS)-(pos[2]-(AREA[2]-BALL_RADIUS));
        }
        self.ball_pos = pos;
        if pos[0] > 0.9 || pos[0] < 0.1 || pos[1] > 0.9 || pos[1] < 0.1 {
            self.paused = true;
            println!("vel: {:?}, pos: {:?}", self.ball_vel, self.ball_pos);
        }

        // move opponent
        self.opponent();
    }

    fn mouse_move(&mut self,  pos: [f64; 2]) {
        let view_distance = AREA[0]/(2.0*FRONT_FILLS*f64::tan(FOV/2.0));
        let front_viewable = 2.0*view_distance*f64::tan(FOV/2.0);
        let front_frac = [AREA[0]/front_viewable, AREA[1]/front_viewable];
        let front_offset = [0.5-front_frac[0]/2.0, 0.5-front_frac[1]/2.0];
        let pos = [(pos[0]-front_offset[0])/front_frac[0], (pos[1]-front_offset[1])/front_frac[1]];
        let movable_x = (RACKET_SIZE[0]/2.0, AREA[0]-RACKET_SIZE[0]/2.0);
        let movable_y = (RACKET_SIZE[1]/2.0, AREA[1]-RACKET_SIZE[1]/2.0);
        self.player_target = [clamp(pos[0], movable_x), clamp(pos[1], movable_y)];
    }

    fn mouse_press(&mut self,  _: MouseButton) {
        self.paused = !self.paused;
    }
    fn mouse_release(&mut self,  _: MouseButton) {

    }

    fn key_press(&mut self,  key: Key) {
        if key == Key::P {
            self.paused = !self.paused;
        }
    }
}

use opengl_graphics::OpenGL;
use piston_window::{Input,Button,Motion,RenderArgs,UpdateArgs}; // from piston::input
use piston_window::draw_state::Blend; // from piston2d-graphics
use piston_window::WindowSettings; // from piston::window
use piston_window::Events; // from piston::event_loop
use piston_window::PistonWindow; // from piston_window

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
                    size = context.get_view_size();
                    let context = context.scale(size[0], size[1]);
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
                let update_args: UpdateArgs = update_args;
                game.update(update_args.dt);// deltatime is its only field
            }

            Input::Resize(x,y) => {
                changed = true;
                // let min = f64::min(x as f64 / INITIAL_SIZE[0],
                //                    y as f64 / INITIAL_SIZE[1]);
                // size = [INITIAL_SIZE[0]*min, INITIAL_SIZE[1]*min];
                // offset = [(x as f64 - size[0]) / 2.0,
                //           (y as f64 - size[1]) / 2.0];
                // gfx.viewport(0, 0, x as i32, y as i32);
            }

            Input::Press(Button::Keyboard(key)) => {
                game.key_press(key);
            }
            Input::Press(Button::Mouse(button)) => {
                game.mouse_press(button);
            }
            Input::Release(Button::Mouse(button)) => {
                game.mouse_release(button);
            }
            Input::Move(Motion::MouseCursor(x,y)) => {
                game.mouse_move([x/INITIAL_SIZE[0], y/INITIAL_SIZE[1]]);
            }
            _ => {}
        }
    }
}
