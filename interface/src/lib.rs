pub mod game;
#[cfg(feature="dyn")]
pub mod reloading;

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
        let game = game::create_game();
        #[cfg(feature="dyn")]
        engine::reload::start_reloading(&game);
        engine::start(game, game::NAME, game::INITIAL_SIZE);
    }
}}
