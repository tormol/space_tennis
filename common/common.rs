//! types used by both hidden.rs and lib.rs


pub type Color = [f32;4];//piston_window::types::Color;
pub type Matrix2d = [[f64;3];2];//
pub trait Graphics {
    fn line(&mut self,  color: Color,  width: f64,  area: [f64;4],  transform: Matrix2d);
    fn rectangle(&mut self,  color: Color,  area: [f64;4],  transform: Matrix2d);
    fn ellipse(&mut self,  color: Color,  area: [f64;4],  transform: Matrix2d);
}

/// Parse a hex string of 6 or 8 bytes into a color.
/// Format is rrggbbaa, where the aa is optional.
#[track_caller]
pub fn hex(color: &str) -> Color {
    let a = match color.len() {
        8 => u8::from_str_radix(&color[6..], 16).unwrap(),
        6 => 255,
        _ => panic!("color string must be 6 or 8 characters")
    };
    let r = u8::from_str_radix(&color[..2], 16).unwrap();
    let g = u8::from_str_radix(&color[2..4], 16).unwrap();
    let b = u8::from_str_radix(&color[4..6], 16).unwrap();
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0]
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
    fn render(&mut self,  transform: Matrix2d,  gfx: &mut dyn Graphics);
    fn update(&mut self,  dt: f64);
    fn mouse_move(&mut self,  pos: [f64; 2]);
    fn mouse_press(&mut self,  button: MouseButton);
}

#[macro_export]
macro_rules! expose_game{($mod:tt::$game:tt) => {
    mod $mod;

    pub use self::$mod::{NAME, INITIAL_SIZE};
    use self::$mod::$game;

    pub fn create_game() -> $game {
        $game::new()
    }
}}

#[macro_export]
macro_rules! impl_main {($dir:tt) => {
    extern crate engine;

    #[cfg(feature="dyn")]
    extern crate $dir;
    #[cfg(not(feature="dyn"))]
    mod $dir;

    fn main() {
        let mut game = game::create_game();
        #[cfg(feature="dyn")]
        engine::reload::start_reloading(&game);
        engine::start(&mut game, game::NAME, game::INITIAL_SIZE);
    }
}}

#[cfg(feature="dyn")]
mod reload {
    use crate::*;

    use std::os::raw::c_void;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicPtr, Ordering::*};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Functions {
        pub render: unsafe fn(*mut c_void,  Matrix2d,  &mut dyn Graphics),
        pub update: unsafe fn(*mut c_void,  f64),
        pub mouse_move: unsafe fn(*mut c_void,  [f64; 2]),
        pub mouse_press: unsafe fn(*mut c_void,  MouseButton),
        pub size: usize
    }

    pub struct ReloadableGame {
        pub game_dir: &'static str,
        pub target_name: &'static str,
        pub functions: Arc<AtomicPtr<Functions>>,
        game: *mut c_void,
    }
    impl ReloadableGame {
        pub fn new<G:Game>(
                game: G,  initial_functions: Functions,
                game_dir: &'static str,  target_name: &'static str,
        ) -> Self {
            ReloadableGame {
                game_dir,
                target_name,
                functions: Arc::new(AtomicPtr::new(Box::leak(Box::new(initial_functions)))),
                game: Box::leak(Box::new(game)) as *mut G as *mut c_void,
            }
        }
        pub fn get(&self) -> &Functions {
            unsafe{ &*self.functions.load(Acquire) }
        }
    }
    impl Game for ReloadableGame {
        fn render(&mut self,  transform: Matrix2d,  gfx: &mut dyn Graphics) {
            unsafe{ (self.get().render)(self.game, transform, gfx) };
        }
        fn update(&mut self,  dt: f64) {
            unsafe{ (self.get().update)(self.game, dt) };
        }
        fn mouse_press(&mut self,  button: MouseButton) {
            unsafe{ (self.get().mouse_press)(self.game, button) };
        }
        fn mouse_move(&mut self,  pos: [f64; 2]) {
            unsafe{ (self.get().mouse_move)(self.game, pos) };
        }
    }
}
#[cfg(feature="dyn")]
pub use reload::*;

#[cfg(feature="dyn")]
#[macro_export]
macro_rules! expose_game_reloadably{($dir:literal/$mod:tt::$game:tt = $target:literal) => {
    mod $mod;

    pub use self::$mod::{NAME, INITIAL_SIZE};
    use self::$mod::$game;

    use std::os::raw::c_void;
    use std::mem::size_of;
    use ::common::{Game, Functions, ReloadableGame};

    unsafe fn game_render_dyn(
            gamestate: *mut c_void,
            transform: ::common::Matrix2d,
            gfx: &mut dyn ::common::Graphics,
    ) {
        (&mut*(gamestate as *mut $game)).render(transform, gfx)
    }
    unsafe fn game_update_dyn(gamestate: *mut c_void,  deltatime: f64) {
        (&mut*(gamestate as *mut $game)).update(deltatime)
    }
    unsafe fn game_mouse_move_dyn(gamestate: *mut c_void,  pos: [f64;2]) {
        (&mut*(gamestate as *mut $game)).mouse_move(pos)
    }
    unsafe fn game_mouse_press_dyn(gamestate: *mut c_void,  button: ::common::MouseButton) {
        (&mut*(gamestate as *mut $game)).mouse_press(button)
    }
    #[no_mangle]
    pub static GAME: Functions = Functions {
        render: game_render_dyn,
        update: game_update_dyn,
        mouse_move: game_mouse_move_dyn,
        mouse_press: game_mouse_press_dyn,
        size: size_of::<$game>()
    };

    pub fn create_game() -> ReloadableGame {
        ReloadableGame::new($game::new(), GAME, $dir, $target)
    }
}}
