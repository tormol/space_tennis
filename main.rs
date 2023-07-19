/* Copyright 2018 Torbjørn Birch Moltu
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

#![cfg_attr(windows, windows_subsystem = "windows")]

extern crate engine;

#[cfg(feature="dyn")]
extern crate game;
#[cfg(not(feature="dyn"))]
mod game;

fn main() {
    let mut game = game::create_game();
    #[cfg(feature="dyn")]
    engine::reload::start_reloading(&game);
    engine::start(&mut game, game::NAME, game::INITIAL_SIZE);
}
