/* Copyright 2018, 2023 Torbjørn Birch Moltu
 *
 * This file is part of space_tennis.
 * You can redistribute it and/or modify it under the terms of the
 * GNU General Public License as published by the Free Software Foundation,
 * either version 3 of the License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

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
