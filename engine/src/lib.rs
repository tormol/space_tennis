//#![cfg_attr(debug_assertions, crate_type = "dylib")]
//#![cfg_attr(feature="dyn", crate_type = "dylib")]
//#![crate_type = "dylib"]

#[cfg(debug_assertions)]
extern crate dlopen;
#[cfg(debug_assertions)]
extern crate notify;
#[cfg(debug_assertions)]
extern crate serde_json;
#[cfg(debug_assertions)]
#[macro_use] extern crate serde_derive;
#[cfg(debug_assertions)]
extern crate serde;

use std::borrow::Cow;
use std::os::raw::c_void;
use std::mem::{transmute,size_of};

mod common;
pub use common::{Graphics,MouseButton,Color,Matrix2d,Game,Functions,StartUpInfo};

#[cfg(debug_assertions)]
mod reload;
#[cfg(not(debug_assertions))]
mod reload {
    pub struct FunctionGetter(Functions);
    impl FunctionGetter {
        pub fn new(f: Functions,  _: Vec<Box<str>>) -> Self {
            FunctionGetter(f)
        }
        pub fn get(&self) -> &Functions {
            &self.0
        }
    }
}

mod piston;
pub use piston::hex;


pub fn start<G:Game, S:Into<Cow<'static,str>>, A:Into<Vec<Box<str>>>>
(game: &mut G,  name: S,  initial_size: [f64;2],  cargo_args: A) {
    unsafe {
        let f = Functions {
            render: transmute::<fn(&mut G,Matrix2d,&mut dyn Graphics),_>(G::render),
            update: transmute::<fn(&mut G,f64),_>(G::update),
            mouse_move: transmute::<fn(&mut G,[f64;2]),_>(G::mouse_move),
            mouse_press: transmute::<fn(&mut G,MouseButton),_>(G::mouse_press),
            size: size_of::<G>()
        };
        let f = reload::FunctionGetter::new(f, cargo_args.into());
        let s = StartUpInfo {
            game: game as *mut G as *mut c_void,
            name: name.into(),
            initial_size: initial_size,
        };
        piston::start(s, f)
    }
}

#[macro_export]
macro_rules! expose_game{($game:ty) => {
    use std::os::raw::c_void;
    use std::mem::size_of;

    #[cfg(debug_assertions)]
    unsafe fn game_render_dyn(gamestate: *mut c_void,  transform: Matrix2d,  gfx: &mut dyn Graphics) {
        (&mut*(gamestate as *mut $game)).render(transform, gfx)
    }
    #[cfg(debug_assertions)]
    unsafe fn game_update_dyn(gamestate: *mut c_void,  deltatime: f64) {
        (&mut*(gamestate as *mut $game)).update(deltatime)
    }
    #[cfg(debug_assertions)]
    unsafe fn game_mouse_move_dyn(gamestate: *mut c_void,  pos: [f64;2]) {
        (&mut*(gamestate as *mut $game)).mouse_move(pos)
    }
    #[cfg(debug_assertions)]
    unsafe fn game_mouse_press_dyn(gamestate: *mut c_void,  button: MouseButton) {
        (&mut*(gamestate as *mut $game)).mouse_press(button)
    }
    #[cfg(debug_assertions)]
    #[no_mangle]
    pub static GAME: Functions = Functions {
        render: game_render_dyn,
        update: game_update_dyn,
        mouse_move: game_mouse_move_dyn,
        mouse_press: game_mouse_press_dyn,
        size: size_of::<$game>()
    };
}}
