//#![cfg_attr(debug_assertions, crate_type = "dylib")]
//#![cfg_attr(feature="dyn", crate_type = "dylib")]
//#![crate_type = "dylib"]

use std::mem::transmute;

mod common;
pub use common::{Graphics,MouseButton,Color,Matrix2d,Game,Functions,StartUpInfo};
mod hidden;
pub use hidden::hex;


pub fn start<G:Game>(game: &mut G,  name: &'static str,  initial_size: [f64;2]) {
    unsafe {
        let game = game as *mut G as *mut u8;
        let f = Functions {
            render: transmute::<fn(&mut G,Matrix2d,&mut dyn Graphics),_>(G::render),
            update: transmute::<fn(&mut G,f64),_>(G::update),
            mouse_move: transmute::<fn(&mut G,[f64;2]),_>(G::mouse_move),
            mouse_press: transmute::<fn(&mut G,MouseButton),_>(G::mouse_press),
        };
        let s = StartUpInfo {name, initial_size, game};
        hidden::start(s, f)
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
