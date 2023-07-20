use crate::game::*;

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

#[cfg(feature="dyn")]
#[macro_export]
macro_rules! expose_game_reloadably{($dir:literal/$mod:tt::$game:tt = $target:literal) => {
    mod $mod;

    pub use self::$mod::{NAME, INITIAL_SIZE};
    use self::$mod::$game;

    use std::os::raw::c_void;
    use std::mem::size_of;
    use ::interface::reloading::{Functions, ReloadableGame};
    use ::interface::game::{Game, Matrix2d, Graphics, MouseButton};

    unsafe fn game_render_dyn(
            gamestate: *mut c_void,
            transform: Matrix2d,
            gfx: &mut dyn Graphics,
    ) {
        (&mut*(gamestate as *mut $game)).render(transform, gfx)
    }
    unsafe fn game_update_dyn(gamestate: *mut c_void,  deltatime: f64) {
        (&mut*(gamestate as *mut $game)).update(deltatime)
    }
    unsafe fn game_mouse_move_dyn(gamestate: *mut c_void,  pos: [f64;2]) {
        (&mut*(gamestate as *mut $game)).mouse_move(pos)
    }
    unsafe fn game_mouse_press_dyn(gamestate: *mut c_void,  button: MouseButton) {
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
