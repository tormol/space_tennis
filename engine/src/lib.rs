//#![cfg_attr(debug_assertions, crate_type = "dylib")]
//#![cfg_attr(feature="dyn", crate_type = "dylib")]
//#![crate_type = "dylib"]

use std::path::Path;
use std::borrow::Cow;
use std::mem::transmute;

mod common;
pub use common::{Graphics,MouseButton,Color,Matrix2d,Game,Functions,StartUpInfo};

#[cfg(debug_assertions)]
mod reload;
#[cfg(not(debug_assertions))]
mod reload {
    pub struct FunctionGetter(Functions);
    impl FunctionGetter {
        pub fn new(f: Functions,  _: Cow<'static,Path>) -> Self {
            FunctionGetter(f)
        }
        pub fn get(&self) -> &Functions {
            &self.0
        }
    }
}

mod piston;
pub use piston::hex;


pub fn start<G:Game, S:Into<Cow<'static,str>>>
(game: &mut G,  name: S,  initial_size: [f64;2],  source_start: &'static str) {
    unsafe {
        let f = Functions {
            render: transmute::<fn(&mut G,Matrix2d,&mut dyn Graphics),_>(G::render),
            update: transmute::<fn(&mut G,f64),_>(G::update),
            mouse_move: transmute::<fn(&mut G,[f64;2]),_>(G::mouse_move),
            mouse_press: transmute::<fn(&mut G,MouseButton),_>(G::mouse_press),
        };
        let s = StartUpInfo {
            game: game as *mut G as *mut u8,
            name: name.into(),
            initial_size: initial_size,
            src: Cow::Borrowed(Path::new(source_start)),
        };
        piston::start(s, f)
    }
}

#[macro_export]
macro_rules! expose_game{($game:ty) => {
    #[cfg(debug_assertions)]
    #[export_name="game_render"]
    pub unsafe fn game_render_dyn(gamestate: *mut u8,  transform: Matrix2d,  gfx: &mut dyn Graphics) {
        (&mut*(gamestate as *mut $game)).render(transform, gfx)
    }
    #[cfg(debug_assertions)]
    #[export_name="game_update"]
    pub unsafe fn game_update_dyn(gamestate: *mut u8,  deltatime: f64) {
        (&mut*(gamestate as *mut $game)).update(deltatime)
    }
    #[cfg(debug_assertions)]
    #[export_name="game_mouse_move"]
    pub unsafe fn game_mouse_move_dyn(gamestate: *mut u8,  pos: [f64;2]) {
        (&mut*(gamestate as *mut $game)).mouse_move(pos)
    }
    #[cfg(debug_assertions)]
    #[export_name="game_mouse_press"]
    pub unsafe fn game_mouse_press_dyn(gamestate: *mut u8,  button: MouseButton) {
        (&mut*(gamestate as *mut $game)).mouse_press(button)
    }
}}
