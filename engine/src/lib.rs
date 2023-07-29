extern crate interface;

#[cfg(all(feature="speedy2d", feature="piston"))]
compile_error!("Only one of speedy2d or piston can be enabled at the same time.");
#[cfg(not(any(feature="speedy2d", feature="piston")))]
compile_error!("One of the speedy2d or piston features must be enabled.");

#[cfg(feature="piston")]
mod piston;
#[cfg(feature="piston")]
pub use piston::*;

#[cfg(feature="speedy2d")]
mod speedy2d;
#[cfg(feature="speedy2d")]
pub use speedy2d::*;

#[cfg(feature="dyn")]
pub mod reload;
