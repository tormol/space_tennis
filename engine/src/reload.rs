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

use interface::reloading::*;

extern crate dlopen;
extern crate notify;

use std::env::consts::{DLL_PREFIX, DLL_SUFFIX};
use std::fs;
use std::io::ErrorKind::*;
use std::path::{Path, PathBuf, MAIN_SEPARATOR_STR};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering::*};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use dlopen::raw::Library;
use notify::{recommended_watcher, Watcher, Error, RecursiveMode};
use notify::event::{Event, EventKind};

fn build_command(game_source_dir: &str,  cargo_args: &[&str]) -> Command {
    let mut cargo = Command::new("cargo");
    cargo.arg("build");
    //cargo.args(&["--verbose"]);
    cargo.args(cargo_args);
    let game_source_dir = match fs::canonicalize(game_source_dir) {
        Ok(dir) => dir,
        Err(_) => PathBuf::from(game_source_dir),
    };
    cargo.current_dir(game_source_dir);
    return cargo;
}

/// Load dynamic library, get the new function pointers
/// and do some trivial sanity checks.
///
/// This is kinda unsafe but the unsafety must end somewhere.
fn reload(lib: &str,  current_size: usize) -> Option<&'static Functions> {
    static ITERATIONS: AtomicUsize = AtomicUsize::new(1);
    // (on linux) dlopen refuses to open the same path multiple times
    let new_name = loop {
        let iterations = ITERATIONS.fetch_add(1, Relaxed);
        let new_name = format!("{}-reload.{}", lib, iterations);
        if let Err(e) = fs::remove_file(&new_name) {
            if e.kind() != NotFound {
                eprintln!("Cannot delete pre-existing {:?}: {}, trying: .{}",
                    &new_name, e, iterations+1
                );
                continue; // increase iterations and try again
            }
        } else {
            //println!("Deleted pre-existing {:?}", &new_name);
        }
        break new_name;
    };
    if let Err(e) = fs::hard_link(lib, &new_name) {
        eprintln!("Cannot create link to {:?} at {:?}: {}", lib, new_name, e);
        return None;
    }
    println!("Trying to reload game functions from {:?}", new_name);
    unsafe {
        let lib = match Library::open(&new_name) {
            Ok(lib) => lib,
            Err(e) => {
                eprintln!("Failed to open {:?} as library: {}", new_name, e);
                return None;
            }
        };
        if let Err(e) = fs::remove_file(&new_name) {
            eprintln!("Cannot delete {:?} after creating and loading it: {}", &new_name, e);
        }
        let symbol: Result<&Functions, _> = lib.symbol("GAME");
        match symbol {
            Ok(ref game) if game.size == current_size => {
                // leak the handle because unloading is very risky.
                // this should only happen a limited number of times,
                // and restarting isn't that bad either
                Box::leak(Box::new(lib));
                Some(game)
            }
            Ok(_) => {
                eprintln!("Game struct has changed size, refusing to swap functions");
                None
            }
            Err(_) => {
                eprintln!("{:?} does not have symbol GAME", new_name);
                eprintln!("\tYou need to add `expose_game!{{$GameStruct}}`");
                None
            }
        }
    }
}


/// Watch for changes in the folder with the game logic,
/// and call a function at most once per second
fn watch(src: &str,  callback: &mut dyn FnMut()) {
    const DEBOUNCE_INTERVAL: Duration = Duration::from_secs(1);
    let mut last_forwarded = Instant::now();
    // run reloads on another thread than the one that handles events,
    // so that the time between events is't affected by the reload time.
    let (tx, rx) = mpsc::channel();
    let debouncer = move |event: Result<Event,Error>| {
        match event {
            Ok(Event { kind: EventKind::Access(_), .. }) => {},
            // Ok(Event { kind: EventKind::Modify(Modify::Metatdata), .. }) => {},
            Ok(ev) => {
                eprintln!("fs event: {:?}", ev);
                let now = Instant::now();
                if now.saturating_duration_since(last_forwarded) >= DEBOUNCE_INTERVAL {
                    last_forwarded = now;
                    tx.send(()).unwrap();
                }
            },
            Err(e) => {
                eprintln!("fs watch error: {} ({:?})", e, e.paths);
            },
        }
    };
    let mut watcher = match recommended_watcher(debouncer) {
        Ok(watcher) => watcher,
        Err(e) => {
            eprintln!("Cannot create fs watcher: {} - Hotswapping will not work", e);
            return;
        }
    };
    if let Err(e) = watcher.watch(Path::new(src), RecursiveMode::NonRecursive) {
        eprintln!("Cannot watch {:?}: {} - Hotswapping will not work", src, e);
        return;
    }

    loop {
        match rx.recv() {
            Ok(()) => callback(),
            Err(e) => {
                eprintln!("fs watcher channel receive error: {}, quitting", e);
                return;
            }
        }
    }
}

pub fn start_reloading(reloadable: &ReloadableGame) {
    let game_dir = reloadable.game_dir;
    let lib = DLL_PREFIX.to_string() + reloadable.target_name + DLL_SUFFIX;
    let lib = [game_dir, "target", "debug", &lib].join(MAIN_SEPARATOR_STR);
    let functions = reloadable.functions.clone();
    thread::spawn(move|| {
        // Don't delay game start on compiling
        let mut command = build_command(game_dir, &[]);
        println!("command: {:?}", &command);
        // for module mode to work, the source code cannot be inside a subdir.
        println!("Watching {:?} for source code changes", game_dir);
        watch(game_dir, &mut|| {
            let started = Instant::now();
            match command.status() {// runs the command
                Ok(exit) if exit.success() => {},
                Ok(_) => return,// cargo printed error
                Err(e) => {
                    eprintln!("Failed to start cargo build: {}", e);
                    return;
                }
            }
            let before = unsafe{ &*functions.load(SeqCst) };
            if let Some(new_functions) = reload(&lib, before.size) {
                functions.store(new_functions as *const _ as *mut _, SeqCst);
                let after = new_functions;
                println!("before: mouse_press={:p}->{:p}", before, before.mouse_press);
                println!("after : mouse_press={:p}->{:p}", after, after.mouse_press);
                println!("Loaded new code in {:?}", started.elapsed());
            }
        });
    });
}
