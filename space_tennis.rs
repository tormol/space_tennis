/* Copyright 2018 Torbjørn Birch Moltu
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

/* Code structure:
 * The game logic and rendering is done by methods attached to the Game struct.
 * `main()` handles window setup and extracts wanted events from the event loop.
 */

#![cfg_attr(windows, windows_subsystem = "windows")]

use std::f64::consts::PI;

extern crate opengl_graphics;
use opengl_graphics::{OpenGL, GlGraphics};

extern crate piston_window;
use piston_window::{Event,Loop,RenderArgs,UpdateArgs,Input}; // from piston_input
use piston_window::{ButtonArgs,ButtonState,Button,MouseButton,Motion}; // from piston_input
use piston_window::{Context,DrawState,Transformed,color,math}; // from piston2d-graphics
use piston_window::draw_state::Blend; // from piston2d-graphics
use piston_window::PistonWindow;
use piston_window::WindowSettings; // from piston::window
use piston_window::Events; // from piston::event_loop
mod draw {
    pub use piston_window::{clear,rectangle,line,ellipse};
}

extern crate glutin_window;

const INITIAL_SIZE: [f64;2] = [500.0, 500.0];
const ARENA: [f64;3] = [1.0, 1.0, 2.0]; // 
const BALL_RADIUS: f64 = 0.125; // exact representable
const MISS_BALL_RADIUS: f64 = 0.025;
const MAX_MISSES: u32 = (ARENA[1]/(3.0*MISS_BALL_RADIUS)) as u32;
const RACKET_SIZE: [f64;2] = [0.22, 0.15];
const PLAYER_MAX_SPEED: [f64;2] = [0.9, 0.9];
const OPPONENT_MAX_SPEED: [f64;2] = [0.4, 0.4];
const PLAYER_RESTART_DELAY: f64 = 0.2; // of ARENA[2]
const OPPONENT_RESTART_DELAY: f64 = 0.3; // of ARENA[2]
const BALL_START_ZSPEED: f64 = 0.6;
const BALL_ZSPEED_LEVEL_ADD: f64 = 0.02;
const BRACKET_SPEED_TRANSFER: f64 = 0.75; // based on mass of ball and bracket
const FOV: f64 = PI/3.0; // 60°
const FRONT_FILLS: f64 = 0.8; // of the screen
//const BACK_FILLS: f64 = 0.3; // of the screen

const WALL_COLOR: &str = "222222";
const WALL_LINE_COLOR: &str = "008800";
const WALL_LINES: u32 = 5;
const LINE_WIDTH: f64 = 0.05;
const LINE_WIDTH_EDGE: f64 = LINE_WIDTH*1.5;
const RACKET_COLOR: &str = "ddddddaa";
const RACKET_BORDER_COLOR: &str = "5555dd";
const RACKET_BORDER_WIDTH: [f64;2] = [0.01333, 0.00666]; // [left/right, top/bottom]
const BALL_COLOR: &str = "aaff55ee";
const BALL_LINE_COLOR: &str = "eeeeee88";
const MISS_COLOR: &str = "ff3333";
const PAUSE_COLOR: &str = "888877aa";

fn clamp(p: f64,  (min,max): (f64,f64)) -> f64 {
         if p <= min   {min}
    else if p >= max   {max}
    else if p.is_nan() {(min+max)/2.0}
    else               {p}
}

#[derive(Clone,Copy, PartialEq,Eq)]
enum State {Playing, Paused, PlayerStart, OpponentStart}

struct Game {
    ball_pos: [f64; 3],
    ball_vel: [f64; 3],
    player_pos: [f64; 2],
    player_target: [f64; 2],
    player_misses: u32,
    opponent_pos: [f64; 2],
    opponent_target: [f64; 2],
    opponent_misses: u32,
    state: State
}
impl Game {
    fn new() -> Self {Game {
        player_misses: 0,
        opponent_misses: 0,
        player_pos: [ARENA[0]/2.0, ARENA[1]/2.0],
        player_target: [ARENA[0]/2.0, ARENA[1]/2.0],
        opponent_pos: [ARENA[0]/2.0, ARENA[1]/2.0],
        opponent_target: [ARENA[0]/2.0, ARENA[1]/2.0],
        ball_vel: [0.0, 0.0, BALL_START_ZSPEED],
        ball_pos: [ARENA[0]/2.0, ARENA[1]/2.0, BALL_RADIUS],// at player
        state: State::PlayerStart
    } }

    fn render(&mut self,  _: DrawState,  transform: math::Matrix2d,  gfx: &mut GlGraphics) {
        /*
        at the center of the window there is a view cone with a certain angle (field of view)
        at x dept the distance from top to bottom or left to right of view
        is 2*x*tan(fov/2). (hosliggende er 1, ikke hypotenusen)
        for now the aspect ratio is assumed to be 1:1, so horizontal and vertical FOV is equal.
        */
        // step 0: find view distance that satisfies FRONT_FILLS:
        // at x distance the viewable area*FRONT_FILLS is ARENA[0] =>
        // 2*view_distance*tan(fov/2)*FRONT_FILLS=ARENA[0]
        let view_distance = ARENA[0]/(2.0*FRONT_FILLS*f64::tan(FOV/2.0));

        // order depends on its z position
        fn draw_ball(
                ball_pos_game: [f64;3],  view_distance: f64,
                transform: [[f64;3];2],  gfx: &mut GlGraphics
        ) {
            let ball_color = color::hex(BALL_COLOR);
            let ball_viewable = 2.0*(view_distance+ball_pos_game[2])*f64::tan(FOV/2.0);
            let ball_depth_frac = (ARENA[0]/ball_viewable, ARENA[1]/ball_viewable);
            let ball_offset = (0.5 - ball_depth_frac.0/2.0,  0.5 - ball_depth_frac.1/2.0);
            let ball_pos_screen = (
                ball_offset.0 + ball_depth_frac.0*ball_pos_game[0],
                ball_offset.1 + ball_depth_frac.1*ball_pos_game[1]
            );
            let ball_frac = BALL_RADIUS*2.0 / ball_viewable;
            let ball_rect = [
                ball_pos_screen.0 - ball_frac/2.0,
                ball_pos_screen.1 - ball_frac/2.0,
                ball_frac, ball_frac
            ];
            draw::ellipse(ball_color, ball_rect, transform, gfx);
        }
        if self.ball_pos[2] > ARENA[2] {
            draw_ball(self.ball_pos, view_distance, transform, gfx);
        }

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
            let near_area_frac = (ARENA[0]/near_viewable, ARENA[1]/near_viewable);
            let far_area_frac = (ARENA[0]/far_viewable, ARENA[1]/far_viewable);
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
            draw::line(color, radius.1, top, transform, gfx);
            draw::line(color, radius.1, bottom, transform, gfx);
            draw::line(color, radius.0, left, transform, gfx);
            draw::line(color, radius.0, right, transform, gfx);
        }
        // draw the walls themselves
        draw_wall_marker(WALL_COLOR, view_distance+ARENA[2]/2.0, ARENA[2], transform, gfx);
        let interval = ARENA[2]/(WALL_LINES+1) as f64;
        // the markers on the edges are thicker
        draw_wall_marker(WALL_LINE_COLOR, view_distance, LINE_WIDTH_EDGE, transform, gfx);
        for n in 1..(WALL_LINES+1) {
            draw_wall_marker(WALL_LINE_COLOR, view_distance + interval*n as f64, LINE_WIDTH, transform, gfx);
        }
        draw_wall_marker(WALL_LINE_COLOR, view_distance+ARENA[2], LINE_WIDTH_EDGE, transform, gfx);

        fn draw_racket(
                pos: [f64;2]/*in arena*/, depth: f64/*from view*/,
                transform: math::Matrix2d,  gfx: &mut GlGraphics,
        ) {
            let fill_color = color::hex(RACKET_COLOR);
            let border_color = color::hex(RACKET_BORDER_COLOR);
            let viewable = 2.0*depth*f64::tan(FOV/2.0);
            let area_frac = [ARENA[0]/viewable, ARENA[1]/viewable];
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
            draw::rectangle(fill_color, fill_area, transform, gfx);
            let radius = [border_frac[0]/2.0, border_frac[1]/2.0]; // [left/right, top/bottom]
            let (left,top,right,bottom) = (
                racket_pos[0]-racket_frac[0]/2.0+radius[0],
                racket_pos[1]-racket_frac[1]/2.0+radius[1],
                racket_pos[0]+racket_frac[0]/2.0-radius[0],
                racket_pos[1]+racket_frac[1]/2.0-radius[1],
            );
            draw::line(border_color, radius[1], [left-radius[0], top, right-radius[0], top], transform, gfx);
            draw::line(border_color, radius[0], [right, top-radius[1], right, bottom-radius[1]], transform, gfx);
            draw::line(border_color, radius[1], [left+radius[0], bottom, right+radius[0], bottom], transform, gfx);
            draw::line(border_color, radius[0], [left, top+radius[1], left, bottom+radius[1]], transform, gfx);
        }
        // opponent racket
        draw_racket(self.opponent_pos, view_distance+ARENA[2], transform, gfx);

        // step 5: ball inside arena
        if self.ball_pos[2] <= ARENA[2]  &&  self.ball_pos[2] >= 0.0 {
            draw_wall_marker(BALL_LINE_COLOR, view_distance+self.ball_pos[2], LINE_WIDTH, transform, gfx);
            draw_ball(self.ball_pos, view_distance, transform, gfx);
        }

        // player racket
        draw_racket(self.player_pos, view_distance, transform, gfx);

        // misses
        let miss_color = color::hex(MISS_COLOR);
        let front_viewable = 2.0*view_distance*f64::tan(FOV/2.0);
        let radius_frac = MISS_BALL_RADIUS/front_viewable;
        let n_offset = 3.0*radius_frac;
        let start_y = 0.5-(ARENA[1]/front_viewable)/2.0;
        let player_x = 0.5 + (ARENA[0]/front_viewable)/2.0 + 2.0*radius_frac;
        let opponent_x = 0.5 - (ARENA[0]/front_viewable)/2.0 - 4.0*radius_frac;
        for n in (0..self.player_misses).take(MAX_MISSES as usize) {
            let rect = [player_x, start_y+n_offset*n as f64, 2.0*radius_frac, 2.0*radius_frac];
            draw::ellipse(miss_color, rect, transform, gfx);
        }
        if self.player_misses > MAX_MISSES {
            let top = start_y + n_offset*(MAX_MISSES as f64);
            let vertical = [player_x+radius_frac*2.0/3.0, top, radius_frac*2.0/3.0, radius_frac*2.0];
            let horizontal = [player_x, top+radius_frac*2.0/3.0, radius_frac*2.0, radius_frac*2.0/3.0];
            draw::rectangle(miss_color, vertical, transform, gfx);
            draw::rectangle(miss_color, horizontal, transform, gfx);
        }
        for n in (0..self.opponent_misses).take(MAX_MISSES as usize) {
            let rect = [opponent_x, start_y+n_offset*n as f64, 2.0*radius_frac, 2.0*radius_frac];
            draw::ellipse(miss_color, rect, transform, gfx);
        }
        if self.opponent_misses > MAX_MISSES {
            let top = start_y + n_offset*MAX_MISSES as f64;
            let vertical = [opponent_x+radius_frac*2.0/3.0, top, radius_frac*2.0/3.0, radius_frac*2.0];
            let horizontal = [opponent_x, top+radius_frac*2.0/3.0, radius_frac*2.0, radius_frac*2.0/3.0];
            draw::rectangle(miss_color, vertical, transform, gfx);
            draw::rectangle(miss_color, horizontal, transform, gfx);
        }

        if self.ball_pos[2] < 0.0 {
            draw_ball(self.ball_pos, view_distance, transform, gfx);
        }

        if self.state == State::Paused {
            // draw pause sign
            let pause_color = color::hex(PAUSE_COLOR);
            draw::rectangle(pause_color, [0.4, 0.4, 0.075, 0.2], transform, gfx);
            draw::rectangle(pause_color, [0.525, 0.4, 0.075, 0.2], transform, gfx);
        }
    }

    fn opponent(&mut self) {
        if self.state == State::OpponentStart {
            self.opponent_target = [ARENA[0]/2.0, ARENA[1]/2.0];
            self.state = State::Playing;
            return;
        }

        // predict where ball will end up without walls, and do nothing if not within reach
        if self.ball_vel[2] <= 0.0 {// moving away
            //self.opponent_target = [ARENA[0]/2.0, ARENA[1]/2.0];
            return
        }
        let dist = ARENA[2]-BALL_RADIUS-self.ball_pos[2];
        let time = dist / self.ball_vel[2];
        let moves = [self.ball_vel[0]*time, self.ball_vel[1]*time, dist];
        let ends = [self.ball_pos[0]+moves[0], self.ball_pos[1]+moves[1], ARENA[2]-BALL_RADIUS];
        if ends[0] < BALL_RADIUS || ends[0] > ARENA[0]-BALL_RADIUS
        || ends[1] < BALL_RADIUS || ends[1] > ARENA[1]-BALL_RADIUS {
            //self.opponent_target = [ARENA[0]/2.0, ARENA[1]/2.0];
            return
        }
        let target_x = clamp(ends[0], (RACKET_SIZE[0]/2.0, ARENA[0]-RACKET_SIZE[0]/2.0));
        let target_y = clamp(ends[1], (RACKET_SIZE[1]/2.0, ARENA[1]-RACKET_SIZE[1]/2.0));
        self.opponent_target = [target_x, target_y];
    }

    fn update(&mut self,  dt: f64) {
        if self.state == State::Paused {
            return;
        }

        // move rackets: be kind to the players and do that first
        fn move_racket(racket: &mut[f64;2],  target: &[f64;2],  max_speed: [f64;2],  dt: f64) -> [f64;2] {
            let max_move = [max_speed[0]*dt, max_speed[1]*dt];
            let diff = [target[0]-racket[0], target[1]-racket[1]];
            let move_x = clamp(diff[0], (-max_move[0], max_move[0]));
            let move_y = clamp(diff[1], (-max_move[1], max_move[1]));
            *racket = [racket[0]+move_x, racket[1]+move_y];
            return [move_x/dt, move_y/dt];
        }
        let player_speed = move_racket(&mut self.player_pos, &self.player_target, PLAYER_MAX_SPEED, dt);
        let opponent_speed = move_racket(&mut self.opponent_pos, &self.opponent_target, OPPONENT_MAX_SPEED, dt);

        if self.state == State::PlayerStart {
            self.ball_pos = [self.player_pos[0], self.player_pos[1], BALL_RADIUS];
            // no loss of speed because it was following the racket without delay
            self.ball_vel = [player_speed[0], player_speed[1], self.ball_vel[2]];
        } else if self.state == State::OpponentStart {
            self.ball_pos = [self.opponent_pos[0], self.opponent_pos[1], ARENA[2]-BALL_RADIUS];
            self.ball_vel = [opponent_speed[0], opponent_speed[1], self.ball_vel[2]];
        } else if self.state == State::Playing {
            // check boundaries and bounce / gameover
            let moved = [self.ball_vel[0]*dt, self.ball_vel[1]*dt, self.ball_vel[2]*dt];
            let mut pos = [self.ball_pos[0]+moved[0], self.ball_pos[1]+moved[1], self.ball_pos[2]+moved[2]];
            // check for score. allow the ball to leave the arena for a bit so that it doesn't
            // look like a bug
            if pos[2] < -ARENA[2]*PLAYER_RESTART_DELAY {
                // game over, restart
                self.player_misses += 1;
                self.ball_pos = [self.player_pos[0], self.player_pos[1], BALL_RADIUS];
                let z_speed = self.ball_vel[2];
                self.ball_vel = [0.0, 0.0, -z_speed+BALL_ZSPEED_LEVEL_ADD];
                self.state = State::PlayerStart;
                return;
            } else if pos[2] > ARENA[2]*(1.0+OPPONENT_RESTART_DELAY) {
                self.opponent_misses += 1;
                self.ball_pos = [self.opponent_pos[0], self.opponent_pos[1], ARENA[2]-BALL_RADIUS];
                let z_speed = self.ball_vel[2];
                self.ball_vel = [0.0, 0.0, -z_speed-BALL_ZSPEED_LEVEL_ADD];
                self.state = State::OpponentStart;
                return;
            } else if pos[2] < 0.0  ||  pos[2] > ARENA[2] {
                // update pos but don't do wall or racket interaction
                self.ball_pos = pos;
                self.opponent();
                return;
            }
            if pos[0] < BALL_RADIUS {
                self.ball_vel[0] *= -1.0;
                pos[0] = BALL_RADIUS+(BALL_RADIUS-pos[0]);
            } else if pos[0] > ARENA[0]-BALL_RADIUS {
                self.ball_vel[0] *= -1.0;
                pos[0] = (ARENA[0]-BALL_RADIUS)-(pos[0]-(ARENA[0]-BALL_RADIUS));
            }
            if pos[1] < BALL_RADIUS {
                self.ball_vel[1] *= -1.0;
                pos[1] = BALL_RADIUS+(BALL_RADIUS-pos[1]);
            } else if pos[1] > ARENA[1]-BALL_RADIUS {
                // println!("wrong: {}", (pos[1]-(ARENA[1]-BALL_RADIUS)));
                // println!("old: {:?}, {:?}", self.ball_vel, self.ball_pos);
                self.ball_vel[1] *= -1.0;
                pos[1] = (ARENA[1]-BALL_RADIUS)-(pos[1]-(ARENA[1]-BALL_RADIUS));
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
            } else if pos[2] > ARENA[2]-BALL_RADIUS && within(pos, self.opponent_pos) {
                self.ball_vel[0] += opponent_speed[0]*BRACKET_SPEED_TRANSFER;
                self.ball_vel[1] += opponent_speed[1]*BRACKET_SPEED_TRANSFER;
                self.ball_vel[2] *= -1.0;
                pos[2] = (ARENA[2]-BALL_RADIUS)-(pos[2]-(ARENA[2]-BALL_RADIUS));
            }
            self.ball_pos = pos;
            if pos[0] > 0.9 || pos[0] < 0.1 || pos[1] > 0.9 || pos[1] < 0.1 {
                self.state = State::Paused;
                println!("vel: {:?}, pos: {:?}", self.ball_vel, self.ball_pos);
            }
        }

        // move opponent
        self.opponent();
    }

    fn mouse_move(&mut self,  pos: [f64; 2]) {
        let view_distance = ARENA[0]/(2.0*FRONT_FILLS*f64::tan(FOV/2.0));
        let front_viewable = 2.0*view_distance*f64::tan(FOV/2.0);
        let front_frac = [ARENA[0]/front_viewable, ARENA[1]/front_viewable];
        let front_offset = [0.5-front_frac[0]/2.0, 0.5-front_frac[1]/2.0];
        let pos = [(pos[0]-front_offset[0])/front_frac[0], (pos[1]-front_offset[1])/front_frac[1]];
        let movable_x = (RACKET_SIZE[0]/2.0, ARENA[0]-RACKET_SIZE[0]/2.0);
        let movable_y = (RACKET_SIZE[1]/2.0, ARENA[1]-RACKET_SIZE[1]/2.0);
        self.player_target = [clamp(pos[0], movable_x), clamp(pos[1], movable_y)];
    }

    fn mouse_press(&mut self,  _: MouseButton) {
        if self.state == State::Paused  ||  self.state == State::PlayerStart {
            self.state = State::Playing;
        } else {
            self.state = State::Paused;
        }
    }
}

fn main() {
    let window_size = [INITIAL_SIZE[0] as u32, INITIAL_SIZE[1] as u32];
    let mut window: PistonWindow = WindowSettings::new("space tennis", window_size)
        .vsync(true)
        .build()
        .unwrap();
    let mut gfx = GlGraphics::new(OpenGL::V3_2);

    let mut size = INITIAL_SIZE; // changes if window is resized

    let mut game = Game::new();
    let mut event_loop: Events = window.events;
    while let Some(e) = event_loop.next(&mut window) {
        match e {
            Event::Loop(Loop::Render(render_args)) => {
                let render_args: RenderArgs = render_args;
                // An optimization introduced in opengl_graphics 0.39.1 causes
                // severe glitching if not wrapped in `gfx.draw()`.
                // (just calling it at the end with an empty closure
                //  seems to work too, for now...)
                gfx.draw(render_args.viewport(), |context, gfx| {
                    let context: Context = context;
                    let gfx: &mut GlGraphics = gfx; // the same instance as outside
                    size = context.get_view_size();
                    let context = context.scale(size[0], size[1]);

                    // by default alpha blending is disabled, which means all
                    // semi-transparent colors are considered opaque.
                    // Blend::Alpha blends colors pixel for pixel,
                    // which has a performance cost.
                    // The alternative would be to check for an existing color
                    // in the tile, and blend manually or even statically.
                    context.draw_state.blend(Blend::Alpha);
                    draw::clear(color::BLACK, gfx);

                    game.render(context.draw_state, context.transform, gfx);
                });
            }
            Event::Loop(Loop::Update(update_args)) => {
                let UpdateArgs{dt: deltatime} = update_args;
                game.update(deltatime);
            }

            Event::Input(Input::Button(ButtonArgs {
                    state: ButtonState::Press,
                    button: Button::Mouse(button),
                    ..
            }), _) => {
                game.mouse_press(button);
            }
            Event::Input(Input::Move(Motion::MouseCursor([x,y])), _) => {
                game.mouse_move([x/size[0], y/size[1]]);
            }
            // TODO pause when window loses focus (!= mouse leaves)
            _ => {}
        }
    }
}
