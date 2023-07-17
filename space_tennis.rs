/* Copyright 2016-2019, 2022-2023 Torbjørn Birch Moltu
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

use std::f32::consts::PI;
use std::time::Instant;

extern crate speedy2d;
use speedy2d::{Graphics2D, Window};
use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::shape::Rectangle;
use speedy2d::window::{
    MouseButton,
    WindowCreationOptions,
    WindowHandler,
    WindowHelper,
    WindowSize,
};

const INITIAL_SIZE: [f32;2] = [500.0, 500.0];
const ARENA: [f32;3] = [1.0, 1.0, 2.0]; // 
const BALL_RADIUS: f32 = 0.125; // exact representable
const MISS_BALL_RADIUS: f32 = 0.025;
const MAX_MISSES: u32 = (ARENA[1]/(3.0*MISS_BALL_RADIUS)) as u32;
const RACKET_SIZE: [f32;2] = [0.22, 0.15];
const PLAYER_MAX_SPEED: [f32;2] = [0.9, 0.9];
const OPPONENT_MAX_SPEED: [f32;2] = [0.4, 0.4];
const PLAYER_RESTART_DELAY: f32 = 0.2; // of ARENA[2]
const OPPONENT_RESTART_DELAY: f32 = 0.3; // of ARENA[2]
const BALL_START_ZSPEED: f32 = 0.6;
const BALL_ZSPEED_LEVEL_ADD: f32 = 0.02;
const BRACKET_SPEED_TRANSFER: f32 = 0.75; // based on mass of ball and bracket
const FOV: f32 = PI/3.0; // 60°
const FRONT_FILLS: f32 = 0.8; // of the screen
//const BACK_FILLS: f32 = 0.3; // of the screen

#[track_caller]
fn hex(color: &str) -> Color {
    let a = match color.len() {
        8 => u8::from_str_radix(&color[6..], 16).unwrap(),
        6 => 255,
        _ => panic!("color string must be 6 or 8 characters")
    };
    let r = u8::from_str_radix(&color[..2], 16).unwrap();
    let g = u8::from_str_radix(&color[2..4], 16).unwrap();
    let b = u8::from_str_radix(&color[4..6], 16).unwrap();
    Color::from_int_rgba(r, g, b, a)
}

const WALL_COLOR: &str = "222332f8";
const WALL_LINE_COLOR: &str = "008800";
const WALL_LINES: u32 = 5;
const LINE_WIDTH: f32 = 0.05;
const LINE_WIDTH_EDGE: f32 = LINE_WIDTH*1.5;
const RACKET_COLOR: &str = "ddddddaa";
const RACKET_BORDER_COLOR: &str = "5555dd";
const RACKET_BORDER_WIDTH: [f32;2] = [0.01333, 0.00666]; // [left/right, top/bottom]
const BALL_COLOR: &str = "aaff55ee";
const BALL_LINE_COLOR: &str = "eeeeee88";
const MISS_COLOR: &str = "ff3333";
const PAUSE_COLOR: &str = "888877aa";

fn clamp(p: f32,  (min,max): (f32,f32)) -> f32 {
         if p <= min   {min}
    else if p >= max   {max}
    else if p.is_nan() {(min/2.0)+(max/2.0)}
    else               {p}
}

fn rect(x: f32,  y: f32,  width: f32,  height: f32) -> Rectangle {
    let top_left = Vector2::new(x, y);
    let bottom_right = Vector2::new(x+width, y+height);
    Rectangle::new(top_left, bottom_right)
}

#[derive(Clone,Copy, PartialEq,Eq)]
enum State {Playing, Paused, PlayerStart, OpponentStart}

struct Game {
    window_size: [f32; 2], // changes if window is resized
    last_physics: Instant,
    ball_pos: [f32; 3],
    ball_vel: [f32; 3],
    player_pos: [f32; 2],
    player_target: [f32; 2],
    player_misses: u32,
    opponent_pos: [f32; 2],
    opponent_target: [f32; 2],
    opponent_misses: u32,
    state: State
}
impl Game {
    fn new() -> Self {Game {
        window_size: INITIAL_SIZE,
        last_physics: Instant::now(),
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

    fn render(&mut self,  g: &mut Graphics2D) {
        /*
        at the center of the window there is a view cone with a certain angle (field of view)
        at x dept the distance from top to bottom or left to right of view
        is 2*x*tan(fov/2). (hosliggende er 1, ikke hypotenusen)
        for now the aspect ratio is assumed to be 1:1, so horizontal and vertical FOV is equal.
        */
        // step 0: find view distance that satisfies FRONT_FILLS:
        // at x distance the viewable area*FRONT_FILLS is ARENA[0] =>
        // 2*view_distance*tan(fov/2)*FRONT_FILLS=ARENA[0]
        let view_distance = ARENA[0]/(2.0*FRONT_FILLS*f32::tan(FOV/2.0));

        // order depends on its z position
        fn draw_ball(
                ball_pos_game: [f32;3],  view_distance: f32,
                window_size: [f32;2],  g: &mut Graphics2D,
        ) {
            let ball_viewable = 2.0*(view_distance+ball_pos_game[2])*f32::tan(FOV/2.0);
            let ball_depth_frac = (ARENA[0]/ball_viewable, ARENA[1]/ball_viewable);
            let ball_offset = (0.5 - ball_depth_frac.0/2.0,  0.5 - ball_depth_frac.1/2.0);
            let ball_pos_screen = (
                ball_offset.0 + ball_depth_frac.0*ball_pos_game[0],
                ball_offset.1 + ball_depth_frac.1*ball_pos_game[1]
            );
            let ball_frac = BALL_RADIUS*2.0 / ball_viewable;
            // scale
            let ball_pos_screen = (ball_pos_screen.0*window_size[0], ball_pos_screen.1*window_size[1]);
            let ball_radius = ball_frac * (window_size[0]/2.0 + window_size[1]/2.0) / 2.0;
            g.draw_circle(ball_pos_screen, ball_radius, hex(BALL_COLOR));
        }
        if self.ball_pos[2] > ARENA[2] {
            draw_ball(self.ball_pos, view_distance, self.window_size, g);
        }

        fn draw_wall_marker(
                color: Color,  depth: f32,  width: f32,
                window_size: [f32;2],  g: &mut Graphics2D,
        ) {
            // width is on the wall, aka the z-dimension.
            // find the draw width by calculating the rectangle of the near and
            // far edge, and setting the center of the lines to the median.
            let near_viewable = 2.0*(depth-width/2.0)*f32::tan(FOV/2.0);
            let far_viewable = 2.0*(depth+width/2.0)*f32::tan(FOV/2.0);
            let near_area_frac = (ARENA[0]/near_viewable, ARENA[1]/near_viewable);
            let far_area_frac = (ARENA[0]/far_viewable, ARENA[1]/far_viewable);
            let near_topleft = (0.5 - (near_area_frac.0/2.0),  0.5 - (near_area_frac.1/2.0));
            let near_bottomright = (0.5 + (near_area_frac.0/2.0),  0.5 + (near_area_frac.1/2.0));
            let far_topleft = (0.5 - (far_area_frac.0/2.0),  0.5 - (far_area_frac.1/2.0));
            let radius = ((far_topleft.0-near_topleft.0)/2.0, (far_topleft.1-near_topleft.1)/2.0);
            let offset_x = (near_topleft.0+radius.0, near_bottomright.0-radius.0);
            let offset_y = (near_topleft.1+radius.1, near_bottomright.1-radius.1);
            // scale to window size
            let radius = (radius.0*window_size[0], radius.1*window_size[1]);
            let offset_x = (offset_x.0*window_size[0], offset_x.1*window_size[0]);
            let offset_y = (offset_y.0*window_size[1], offset_y.1*window_size[1]);
            // draw corners completely, and only once in case the color is translucent
            let top_start = (offset_x.0-radius.0, offset_y.0);
            let top_end = (offset_x.1-radius.0, offset_y.0);
            let bottom_start = (offset_x.0+radius.0, offset_y.1);
            let bottom_end = (offset_x.1+radius.0, offset_y.1);
            let left_start = (offset_x.0, offset_y.0+radius.1);
            let left_end = (offset_x.0, offset_y.1+radius.1);
            let right_start = (offset_x.1, offset_y.0-radius.1);
            let right_end = (offset_x.1, offset_y.1-radius.1);
            g.draw_line(top_start, top_end, radius.1*2.0, color);
            g.draw_line(bottom_start, bottom_end, radius.1*2.0, color);
            g.draw_line(left_start, left_end, radius.0*2.0, color);
            g.draw_line(right_start, right_end, radius.0*2.0, color);
        }
        // draw the walls themselves
        draw_wall_marker(hex(WALL_COLOR), view_distance+ARENA[2]/2.0, ARENA[2], self.window_size, g);
        let interval = ARENA[2]/(WALL_LINES+1) as f32;
        // the markers on the edges are thicker
        let wall_line_color = hex(WALL_LINE_COLOR);
        draw_wall_marker(wall_line_color, view_distance, LINE_WIDTH_EDGE, self.window_size, g);
        for n in 1..(WALL_LINES+1) {
            draw_wall_marker(wall_line_color, view_distance + interval*n as f32, LINE_WIDTH, self.window_size, g);
        }
        draw_wall_marker(wall_line_color, view_distance+ARENA[2], LINE_WIDTH_EDGE, self.window_size, g);

        fn draw_racket(
                pos: [f32;2]/*in arena*/,  depth: f32/*from view*/,
                window_size: [f32;2],  g: &mut Graphics2D,
        ) {
            let fill_color = hex(RACKET_COLOR);
            let border_color = hex(RACKET_BORDER_COLOR);
            let viewable = 2.0*depth*f32::tan(FOV/2.0);
            let area_frac = [ARENA[0]/viewable, ARENA[1]/viewable];
            let border_frac = [RACKET_BORDER_WIDTH[0]/viewable, RACKET_BORDER_WIDTH[1]/viewable];
            let area_offset = [0.5-area_frac[0]/2.0, 0.5-area_frac[1]/2.0];
            let racket_frac = [RACKET_SIZE[0]/viewable, RACKET_SIZE[1]/viewable];
            let racket_pos = [area_offset[0]+area_frac[0]*pos[0], area_offset[1]+area_frac[1]*pos[1]];
            let fill_area = rect(
                (racket_pos[0]-racket_frac[0]/2.0+border_frac[0]) * window_size[0],
                (racket_pos[1]-racket_frac[1]/2.0+border_frac[1]) * window_size[1],
                (racket_frac[0]-2.0*border_frac[0]) * window_size[0],
                (racket_frac[1]-2.0*border_frac[1]) * window_size[1],
            );
            g.draw_rectangle(fill_area, fill_color);
            let radius = [border_frac[0]/2.0, border_frac[1]/2.0]; // [left/right, top/bottom]
            let (left,top,right,bottom) = (
                racket_pos[0]-racket_frac[0]/2.0+radius[0],
                racket_pos[1]-racket_frac[1]/2.0+radius[1],
                racket_pos[0]+racket_frac[0]/2.0-radius[0],
                racket_pos[1]+racket_frac[1]/2.0-radius[1],
            );
            g.draw_line((left-radius[0], top), (right-radius[0], top), radius[1], border_color);
            g.draw_line((right, top-radius[1]), (right, bottom-radius[1]), radius[0], border_color);
            g.draw_line((left+radius[0], bottom), (right+radius[0], bottom), radius[1], border_color);
            g.draw_line((left, top+radius[1]), (left, bottom+radius[1]), radius[0], border_color);
        }
        // opponent racket
        draw_racket(self.opponent_pos, view_distance+ARENA[2], self.window_size, g);

        // step 5: ball inside arena
        if self.ball_pos[2] <= ARENA[2]  &&  self.ball_pos[2] >= 0.0 {
            draw_wall_marker(hex(BALL_LINE_COLOR), view_distance+self.ball_pos[2], LINE_WIDTH, self.window_size, g);
            draw_ball(self.ball_pos, view_distance, self.window_size, g);
        }

        // player racket
        draw_racket(self.player_pos, view_distance, self.window_size, g);

        // misses
        let miss_color = hex(MISS_COLOR);
        let front_viewable = 2.0*view_distance*f32::tan(FOV/2.0);
        let radius_frac = MISS_BALL_RADIUS/front_viewable;
        let n_offset = 3.0*radius_frac;
        let start_y = 0.5-(ARENA[1]/front_viewable)/2.0;
        let player_x = 0.5 + (ARENA[0]/front_viewable)/2.0 + 2.0*radius_frac;
        let opponent_x = 0.5 - (ARENA[0]/front_viewable)/2.0 - 4.0*radius_frac;
        for n in (0..self.player_misses).take(MAX_MISSES as usize) {
            g.draw_circle((player_x, start_y+n_offset*n as f32), 2.0*radius_frac, miss_color);
        }
        if self.player_misses > MAX_MISSES {
            let top = start_y + n_offset*(MAX_MISSES as f32);
            let vertical = rect(player_x+radius_frac*2.0/3.0, top, radius_frac*2.0/3.0, radius_frac*2.0);
            let horizontal = rect(player_x, top+radius_frac*2.0/3.0, radius_frac*2.0, radius_frac*2.0/3.0);
            g.draw_rectangle(vertical, miss_color);
            g.draw_rectangle(horizontal, miss_color);
        }
        for n in (0..self.opponent_misses).take(MAX_MISSES as usize) {
            g.draw_circle((opponent_x, start_y+n_offset*n as f32), 2.0*radius_frac, miss_color);
        }
        if self.opponent_misses > MAX_MISSES {
            let top = start_y + n_offset*MAX_MISSES as f32;
            let vertical = rect(opponent_x+radius_frac*2.0/3.0, top, radius_frac*2.0/3.0, radius_frac*2.0);
            let horizontal = rect(opponent_x, top+radius_frac*2.0/3.0, radius_frac*2.0, radius_frac*2.0/3.0);
            g.draw_rectangle(vertical, miss_color);
            g.draw_rectangle(horizontal, miss_color);
        }

        if self.ball_pos[2] < 0.0 {
            draw_ball(self.ball_pos, view_distance, self.window_size, g);
        }

        if self.state == State::Paused {
            // draw pause sign
            let pause_color = hex(PAUSE_COLOR);
            g.draw_rectangle(rect(0.4*self.window_size[0], 0.4*self.window_size[1], 0.075*self.window_size[0], 0.2*self.window_size[1]), pause_color);
            g.draw_rectangle(rect(0.525*self.window_size[0], 0.4*self.window_size[1], 0.075*self.window_size[0], 0.2*self.window_size[1]), pause_color);
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

    fn update(&mut self,  dt: f32) {
        if self.state == State::Paused {
            return;
        }

        // move rackets: be kind to the players and do that first
        fn move_racket(racket: &mut[f32;2],  target: &[f32;2],  max_speed: [f32;2],  dt: f32) -> [f32;2] {
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

            fn within(pos: [f32; 3], racket_center: [f32; 2]) -> bool {
                f32::abs(pos[0] - racket_center[0]) <= RACKET_SIZE[0] &&
                f32::abs(pos[1] - racket_center[1]) <= RACKET_SIZE[1]
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

    fn mouse_move(&mut self,  pos: [f32; 2]) {
        let view_distance = ARENA[0]/(2.0*FRONT_FILLS*f32::tan(FOV/2.0));
        let front_viewable = 2.0*view_distance*f32::tan(FOV/2.0);
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

impl WindowHandler for Game {
    fn on_start(&mut self,
            h: &mut WindowHelper<()>,
            _: speedy2d::window::WindowStartupInfo
    ) {
        h.set_cursor_visible(false);
        h.set_cursor_grab(false).unwrap();
    }
    fn on_draw(&mut self,  h: &mut WindowHelper<()>,  g: &mut Graphics2D) {
        let prev = self.last_physics;
        self.last_physics = Instant::now();
        let elapsed = self.last_physics.saturating_duration_since(prev);
        self.update(elapsed.as_secs_f32());

        g.clear_screen(Color::BLACK);
        self.render(g);

        // Required to make the screen update.
        // Surprisingly doesn't cause 100% CPU usage.
        h.request_redraw();
    }

    fn on_resize(&mut self,  _: &mut WindowHelper<()>,  size: speedy2d::dimen::UVec2) {
        self.window_size[0] = size.into_f32().x;
        self.window_size[1] = size.into_f32().y;
    }

    fn on_mouse_move(&mut self,  _: &mut WindowHelper<()>,  pos: Vector2<f32>) {
        self.mouse_move([pos.x/self.window_size[0], pos.y/self.window_size[1]]);
    }

    fn on_mouse_button_down(&mut self,  _: &mut WindowHelper<()>,  button: MouseButton) {
        println!("on_mouse_button_down");
        self.mouse_press(button);        
    }

    // TODO pause when window loses focus (!= mouse leaves)
}

fn main() {
    let size = (INITIAL_SIZE[0], INITIAL_SIZE[1]);
    let size = WindowSize::ScaledPixels(size.into());
    let options = WindowCreationOptions::new_windowed(size, None)
            .with_always_on_top(false)
            .with_decorations(true)
            .with_resizable(true)
            .with_transparent(false)
            .with_vsync(true);
    let window = Window::new_with_options("space tennis", options).unwrap();
    let game = Game::new();
    window.run_loop(game);
}
