//! THis file is only compiled when dyn feature is disabled
extern crate common;

mod space_tennis;
pub use self::space_tennis::{NAME, INITIAL_SIZE};
use self::space_tennis::SpaceTennis;

pub fn create_game() -> SpaceTennis {
    SpaceTennis::new()
}
