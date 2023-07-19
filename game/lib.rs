//! This file is only compiled when the dyn feature is enabled

#[macro_use]
extern crate common;
mod space_tennis;

pub use self::space_tennis::{NAME, INITIAL_SIZE};
use self::space_tennis::SpaceTennis;

expose_game!{SpaceTennis}

pub fn create_game() -> ::common::ReloadableGame {
    ::common::ReloadableGame::new(SpaceTennis::new(), GAME, "game", "game")
}
