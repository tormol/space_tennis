use crate::game::*;

use std::os::raw::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, Ordering::*};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Functions {
    pub render: unsafe fn(*mut c_void,  &mut Graphics),
    pub update: unsafe fn(*mut c_void,  f32),
    pub key_press: unsafe fn(*mut c_void,  Key),
    pub key_release: unsafe fn(*mut c_void,  Key),
    pub mouse_move: unsafe fn(*mut c_void,  [f32; 2]),
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
    fn render(&mut self,  gfx: &mut Graphics) {
        unsafe{ (self.get().render)(self.game, gfx) };
    }
    fn update(&mut self,  dt: f32) {
        unsafe{ (self.get().update)(self.game, dt) };
    }
    fn key_press(&mut self,  key: Key) {
        unsafe{ (self.get().key_press)(self.game, key) };
    }
    fn key_release(&mut self,  key: Key) {
        unsafe{ (self.get().key_release)(self.game, key) };
    }
    fn mouse_press(&mut self,  button: MouseButton) {
        unsafe{ (self.get().mouse_press)(self.game, button) };
    }
    fn mouse_move(&mut self,  pos: [f32; 2]) {
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
    use ::interface::game::{Game, Graphics, Key, MouseButton};

    unsafe fn game_render_dyn(gamestate: *mut c_void,  g: &mut Graphics) {
        (&mut*(gamestate as *mut $game)).render(g)
    }
    unsafe fn game_update_dyn(gamestate: *mut c_void,  deltatime: f32) {
        (&mut*(gamestate as *mut $game)).update(deltatime)
    }
    unsafe fn game_key_press_dyn(gamestate: *mut c_void,  key: Key) {
        (&mut*(gamestate as *mut $game)).key_press(key)
    }
    unsafe fn game_key_release_dyn(gamestate: *mut c_void,  key: Key) {
        (&mut*(gamestate as *mut $game)).key_release(key)
    }
    unsafe fn game_mouse_move_dyn(gamestate: *mut c_void,  pos: [f32;2]) {
        (&mut*(gamestate as *mut $game)).mouse_move(pos)
    }
    unsafe fn game_mouse_press_dyn(gamestate: *mut c_void,  button: MouseButton) {
        (&mut*(gamestate as *mut $game)).mouse_press(button)
    }
    #[no_mangle]
    pub static GAME: Functions = Functions {
        render: game_render_dyn,
        update: game_update_dyn,
        key_press: game_key_press_dyn,
        key_release: game_key_release_dyn,
        mouse_move: game_mouse_move_dyn,
        mouse_press: game_mouse_press_dyn,
        size: size_of::<$game>()
    };

    pub fn create_game() -> ReloadableGame {
        ReloadableGame::new($game::new(), GAME, $dir, $target)
    }
}}
